import CoreLocation
import Foundation
import MapKit
import QGCMapTileC

final class MissionStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "mission"

    @Published private(set) var items: [MissionItem] = []
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published private(set) var vehiclePosition: VehicleMarker?
    @Published private(set) var dirty = false
    @Published private(set) var connected = false
    @Published var arming: String?
    @Published private(set) var patterns: [String] = []
    @Published private(set) var canUndo = false
    @Published private(set) var canRedo = false
    @Published private(set) var commands: [MissionCommand] = []
    @Published private(set) var commandCategories: [String] = []
    @Published var pickingCommandFor: Int?

    @Published private(set) var pickerCategory = ""
    @Published private(set) var selectedFacts: [ItemFact] = []
    @Published private(set) var selectedSpeed = ItemSpeed.unavailable
    @Published private(set) var launch = LaunchPosition.unknown
    @Published private(set) var surveyStats = SurveyStats.none
    @Published private(set) var camera = CameraChoice.empty
    @Published private(set) var distanceMode = AltitudeMode.none
    @Published private(set) var itemAltitudeMode = AltitudeMode.none
    @Published private(set) var globalAltitudeMode = AltitudeMode.none
    @Published private(set) var missionModes: [AltitudeModeOffer] = []
    @Published private(set) var itemModes: [AltitudeModeOffer] = []
    @Published private(set) var distanceModes: [AltitudeModeOffer] = []
    @Published private(set) var defaultAltitude = ""
    @Published private(set) var summary = PlanSummary.empty
    @Published private(set) var vehicle = MissionVehicle.unknown
    @Published private(set) var cruiseSpeed = ""
    @Published private(set) var hoverSpeed = ""
    @Published private(set) var planFile = ""
    @Published private(set) var readyToSave = false
    @Published private(set) var notReadyReason = ""
    @Published var uploadWarning: PlanUpload?
    @Published var writeFailure: String?
    @Published var focus: MapFrame?
    @Published var centreMenuOpen = false
    @Published private(set) var scaleBar = MapScaleBar.none
    @Published private(set) var terrain = TerrainProfile.empty
    @Published private(set) var kinds = MissionKinds.empty

    private var undoPoll: Timer?
    private var watchPoll: Timer?

    // The Fly view only reads this plan; the Plan window is what edits it, and a plan can also
    // arrive from the vehicle or a file. So a reader has to look again rather than wait for a
    // telemetry tick that a vehicle on the ground never sends.
    func startWatching() {
        guard watchPoll == nil else { return }
        reload()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.reload()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    func reload() {
        let controller = Bridge.group("plan.missionController")
        guard controller["kind"] as? String == "object" else {
            status = "No vehicle is connected."
            items = []
            patterns = []
            return
        }

        let readiness = PlanReadiness(Bridge.group("view.plan")["readiness"]) ?? .unknown
        if readiness.ready != readyToSave { readyToSave = readiness.ready }
        if readiness.reason != notReadyReason { notReadyReason = readiness.reason }

        let offered = (controller["complexMissionItemNames"] as? [String]) ?? []
        if offered != patterns { patterns = offered }

        let model = Bridge.group("plan.missionController.visualItems")
        let listed = ((model["elements"] as? [[String: Any]]) ?? [])
            .enumerated().map {
                MissionItem(json: $0.element, index: $0.offset,
                            verticalMeasure: AppUnits.measure(AppUnits.vertical))
            }
        if listed != items { items = listed }
        let plan = Bridge.group("plan")
        let busy = (plan["syncInProgress"] as? NSNumber)?.boolValue ?? false
        if busy != syncing { syncing = busy }
        let changed = (plan["dirty"] as? NSNumber)?.boolValue ?? false
        if changed != dirty { dirty = changed }
        let file = (plan["currentPlanFile"] as? String) ?? ""
        if file != planFile { planFile = file }
        let bar = MissionMap.lastScale["plan"] ?? .none
        if bar != scaleBar { scaleBar = bar }
        let profile = TerrainProfile(Bridge.group("view.terrainProfile"))
        if profile != terrain { terrain = profile }
        let catalogue = MissionKinds(Bridge.group("view.missionKinds"))
        if !catalogue.all.isEmpty, catalogue != kinds { kinds = catalogue }
        let mode = AltitudeMode.read(controller["globalAltitudeMode"])
        if mode != globalAltitudeMode { globalAltitudeMode = mode }
        let missionOffers = AltitudeMode.offers(
            Bridge.group("view.altitudeModes(\(AltitudeMode.missionContext),\(mode))"))
        if missionOffers != missionModes { missionModes = missionOffers }

        let hover = (controller["missionHoverDistance"] as? NSNumber)?.doubleValue ?? 0
        let cruise = (controller["missionCruiseDistance"] as? NSNumber)?.doubleValue ?? 0
        summary = PlanSummary(
            distanceMetres: hover + cruise,
            seconds: (controller["missionTime"] as? NSNumber)?.doubleValue ?? 0,
            maxTelemetryMetres: (controller["missionMaxTelemetry"] as? NSNumber)?.doubleValue ?? 0,
            measure: AppUnits.measure(AppUnits.horizontal))
        defaultAltitude = (Bridge.group("settings.appSettings.defaultMissionItemAltitude")["valueString"] as? String) ?? ""

        let controllerVehicle = Bridge.group("plan.controllerVehicle")
        vehicle = MissionVehicle(
            firmware: (controllerVehicle["firmwareTypeString"] as? String) ?? "",
            type: (controllerVehicle["vehicleTypeString"] as? String) ?? "",
            multiRotor: (controllerVehicle["multiRotor"] as? NSNumber)?.boolValue ?? false,
            vtol: (controllerVehicle["vtol"] as? NSNumber)?.boolValue ?? false,
            apmFirmware: (controllerVehicle["apmFirmware"] as? NSNumber)?.boolValue ?? false)
        cruiseSpeed = (Bridge.group("settings.appSettings.offlineEditingCruiseSpeed")["valueString"] as? String) ?? ""
        hoverSpeed = (Bridge.group("settings.appSettings.offlineEditingHoverSpeed")["valueString"] as? String) ?? ""
        launch = LaunchPosition(
            home: (controllerVehicle["homePosition"] as? [String: Any]) ?? [:],
            item: (model["elements"] as? [[String: Any]])?.first ?? [:],
            altitude: Bridge.group("plan.missionController.visualItems.0.plannedHomePositionAltitude"))
        canUndo = (plan["canUndo"] as? NSNumber)?.boolValue ?? false
        canRedo = (plan["canRedo"] as? NSNumber)?.boolValue ?? false
        loadCommands()
        loadSelectedFacts()
        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        status = items.isEmpty ? "This plan has no items." : ""

        let vehicle = Bridge.group("vehicle")
        let coordinate = vehicle["coordinate"] as? [String: Any]
        let heading = ((vehicle["facts"] as? [[String: Any]]) ?? [])
            .first { ($0["name"] as? String) == "heading" }
            .flatMap { ($0["value"] as? NSNumber)?.doubleValue }
        vehiclePosition = VehicleMarker(
            latitude: (coordinate?["latitude"] as? NSNumber)?.doubleValue,
            longitude: (coordinate?["longitude"] as? NSNumber)?.doubleValue,
            heading: heading)
    }

    func downloadFromVehicle() {
        guard offersDownload else { return }
        Bridge.invoke("plan.loadFromVehicle")
        syncing = true
        reload()
    }

    func uploadToVehicle() {
        guard let check = preCheck() else {
            writeFailure = PlanUpload.uncheckable
            return
        }
        if !check.canSend { uploadWarning = check } else { send() }
    }

    // Read fresh on the attempt: terrain arriving does not raise an event, so the answer
    // held from the last reload can be out of date by the time the operator presses send.
    func preCheck() -> PlanUpload? {
        PlanUpload(Bridge.group("view.plan")["upload"])
    }

    func confirmUpload() {
        guard let warning = uploadWarning, warning.canProceed else { return }
        if warning.pausesFirst {
            Bridge.invoke("vehicle.pauseVehicle")
        }
        uploadWarning = nil
        send()
    }

    func cancelUpload() {
        uploadWarning = nil
    }

    private func send() {
        Bridge.invoke("plan.sendToVehicle")
        syncing = true
        reload()
    }

    func startEditing() {
        _ = Bridge.set("plan.undoTracking", true)
        undoPoll?.invalidate()
        undoPoll = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            self?.refreshUndo()
        }
        refreshUndo()
    }

    func stopEditing() {
        undoPoll?.invalidate()
        undoPoll = nil
        _ = Bridge.set("plan.undoTracking", false)
    }

    private func refreshUndo() {
        let plan = Bridge.group("plan")
        let undo = (plan["canUndo"] as? NSNumber)?.boolValue ?? false
        let redo = (plan["canRedo"] as? NSNumber)?.boolValue ?? false
        let unsent = (plan["dirty"] as? NSNumber)?.boolValue ?? false

        if undo != canUndo { canUndo = undo }
        if redo != canRedo { canRedo = redo }
        if unsent != dirty { dirty = unsent }
    }

    // PlanMasterController::_shiftSnapshot returns silently on an empty stack, and the bridge's
    // ok only says the method was found, so the gate has to live where every caller reads it:
    // the view's .disabled, this guard and the probe's refusal were three chances to disagree.
    var offersUndo: Bool { canUndo && !syncing }

    var offersRedo: Bool { canRedo && !syncing }

    var offersDownload: Bool { connected && !syncing }

    func undo() {
        guard offersUndo else { return }
        Bridge.invoke("plan.undo")
        reload()
    }

    func redo() {
        guard offersRedo else { return }
        Bridge.invoke("plan.redo")
        reload()
    }

    func addWaypoint(latitude: Double, longitude: Double) {
        let asked = arming ?? "waypoint"
        let index = items.count
        let at = ["latitude": latitude, "longitude": longitude]

        if let simple = kinds.byId(asked), simple.simple {
            Bridge.invoke("plan.missionController.\(simple.invokable)", [at, index, true])
        } else {
            Bridge.invoke("plan.missionController.insertComplexMissionItem",
                          [asked, at, index, true])
            if let known = kinds.byComplexName(asked) {
                seed(known, at: index, latitude: latitude, longitude: longitude)
            }
        }

        arming = nil
        reload()
    }

    private func seed(_ kind: MissionItemKind, at index: Int, latitude: Double, longitude: Double) {
        guard let plan = MissionSeed(
            Bridge.group("view.missionSeed(\(kind.id),\(latitude),\(longitude))")) else { return }
        let path = "plan.missionController.visualItems.\(index).\(plan.property)"
        plan.points.forEach { point in
            Bridge.invoke("\(path).appendVertex",
                          [["latitude": point.latitude, "longitude": point.longitude]])
        }
    }

    private func loadSurveyStats(for item: MissionItem) {
        guard item.isSurveyItem else {
            if surveyStats != .none { surveyStats = .none }
            return
        }
        let read = SurveyStats(Bridge.group("view.surveyStats(\(item.index))"))
        if read != surveyStats { surveyStats = read }
    }

    private func surveyPolygon(of item: MissionItem) -> [GeoPoint] {
        guard let property = kinds.areaProperty(forCommand: item.command)
        else { return [] }
        let polygon = Bridge.group("plan.missionController.visualItems.\(item.index).\(property)")
        return ((polygon["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
    }

    private func corridorPath(of item: MissionItem) -> [GeoPoint] {
        guard let property = kinds.lineProperty(forCommand: item.command)
        else { return [] }
        let line = Bridge.group("plan.missionController.visualItems.\(item.index).\(property)")
        return ((line["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
    }

    var corridorPaths: [[GeoPoint]] {
        items.map(corridorPath).filter { $0.count >= 2 }
    }

    var surveyAreas: [[GeoPoint]] {
        items.map(surveyPolygon).filter { $0.count >= 3 }
    }

    private func polygon(at path: String, ring: Bool) -> EditablePolygon? {
        EditablePolygon(Bridge.group("view.polygon(\(path)\(ring ? "" : ",line"))"))
    }

    var editablePolygons: [EditablePolygon] {
        let areas = items.compactMap { item -> EditablePolygon? in
            guard let property = kinds.areaProperty(forCommand: item.command) else {
                return nil
            }
            return polygon(at: "plan.missionController.visualItems.\(item.index).\(property)",
                           ring: true)
        }
        let lines = items.compactMap { item -> EditablePolygon? in
            guard let property = kinds.lineProperty(forCommand: item.command) else {
                return nil
            }
            return polygon(at: "plan.missionController.visualItems.\(item.index).\(property)",
                           ring: false)
        }
        let fences = FenceShape.list(Bridge.group("view.fences")["polygons"]).compactMap {
            polygon(at: $0.path, ring: true)
        }
        return areas + lines + fences
    }

    func moveVertex(_ polygon: EditablePolygon, _ index: Int,
                    latitude: Double, longitude: Double) {
        guard polygon.points.indices.contains(index) else { return }
        Bridge.invoke("\(polygon.path).\(polygon.adjustInvokable)",
                      [index, ["latitude": latitude, "longitude": longitude, "altitude": 0]])
        reload()
    }

    func removeVertex(_ polygon: EditablePolygon, _ index: Int) {
        guard PolygonEdit.removes(index, in: polygon) else { return }
        Bridge.invoke("\(polygon.path).\(polygon.removeInvokable)", [index])
        reload()
    }

    func splitSegment(_ polygon: EditablePolygon, after index: Int) {
        guard PolygonEdit.splits(index, in: polygon) else { return }
        Bridge.invoke("\(polygon.path).\(polygon.splitInvokable)", [index])
        reload()
    }

    func move(sequence: Int, latitude: Double, longitude: Double) {
        guard let item = items.first(where: { $0.sequence == sequence }), item.canRemove else { return }
        write("plan.missionController.visualItems.\(item.index).coordinate",
              ["latitude": latitude, "longitude": longitude], "where this item is")
        reload()
    }

    private func loadSelectedFacts() {
        guard let item = items.first(where: \.isCurrent) else {
            selectedFacts = []
            if selectedSpeed != .unavailable { selectedSpeed = .unavailable }
            if surveyStats != .none { surveyStats = .none }
            return
        }
        let speed = ItemSpeed(json: Bridge.group(
            "plan.missionController.visualItems.\(item.index).speedSection"))
        if speed != selectedSpeed { selectedSpeed = speed }

        let listed = ItemFact.lists.flatMap { list in
            ItemFact.from(
                (Bridge.group("plan.missionController.visualItems.\(item.index).\(list)")["elements"] as? [Any]) ?? [],
                list: list, label: Labels.humanise)
        }

        let calc = item.isSimpleItem
            ? [:]
            : Bridge.group("plan.missionController.visualItems.\(item.index).cameraCalc")
        camera = CameraChoice(json: calc)
        loadSurveyStats(for: item)
        distanceMode = AltitudeMode.read(calc["distanceMode"])
        itemAltitudeMode = item.specifiesAltitude
            ? AltitudeMode.read(Bridge.group("plan.missionController.visualItems.\(item.index)")["altitudeMode"])
            : AltitudeMode.none
        let context = AltitudeMode.itemContext
        let itemOffers = AltitudeMode.offers(
            Bridge.group("view.altitudeModes(\(context),\(itemAltitudeMode))"))
        if itemOffers != itemModes { itemModes = itemOffers }
        let distanceOffers = AltitudeMode.offers(
            Bridge.group("view.altitudeModes(\(context),\(distanceMode))"))
        if distanceOffers != distanceModes { distanceModes = distanceOffers }

        let cameraFacts = item.isSimpleItem ? [] : ItemFact.camera(
            (calc["facts"] as? [Any]) ?? [], label: Labels.humanise)

        selectedFacts = cameraFacts + (listed.isEmpty && !item.isSimpleItem
            ? ItemFact.owned((Bridge.group("plan.missionController.visualItems.\(item.index)")["facts"] as? [Any]) ?? [],
                          label: Labels.humanise)
            : listed)
    }

    func setCamera(brand: String? = nil, model: String? = nil) {
        guard let item = items.first(where: \.isCurrent) else { return }
        let path = "plan.missionController.visualItems.\(item.index).cameraCalc"
        if let brand { write("\(path).cameraBrand", brand, "the camera") }
        if let model { write("\(path).cameraModel", model, "the camera model") }
        reload()
    }

    func setGlobalAltitudeMode(_ raw: Int) {
        if let refused = AltitudeMode.refusal(missionModes, raw: raw) {
            writeFailure = refused
            return
        }
        write("plan.missionController.globalAltitudeMode", raw, "the altitude mode")
        reload()
    }

    func setDefaultAltitude(_ value: String) {
        guard let metres = Double(value), metres.isFinite else { return }
        write("settings.appSettings.defaultMissionItemAltitude", metres,
              "the altitude for new items")
        reload()
    }

    func setLaunchAltitude(_ value: Double) {
        guard value.isFinite else { return }
        write("plan.missionController.visualItems.0.plannedHomePositionAltitude", value,
              "the launch altitude")
        reload()
    }

    func setLaunchToMapCentre() {
        guard let centre = mapCentre else { return }
        write("plan.missionController.visualItems.0.coordinate",
              ["latitude": centre.latitude, "longitude": centre.longitude,
               "altitude": launchAltitudeMetres],
              "the launch position")
        reload()
    }

    private var launchAltitudeMetres: Double {
        (Bridge.group("plan.missionController.visualItems.0.plannedHomePositionAltitude.rawValue")["value"]
            as? NSNumber)?.doubleValue ?? 0
    }

    func setCruiseSpeed(_ value: String) {
        setSpeed("offlineEditingCruiseSpeed", value)
    }

    func setHoverSpeed(_ value: String) {
        setSpeed("offlineEditingHoverSpeed", value)
    }

    private func setSpeed(_ setting: String, _ value: String) {
        guard let speed = Double(value), speed.isFinite else { return }
        let path = "settings.appSettings.\(setting)"
        let slowest = (Bridge.group(path)["min"] as? NSNumber)?.doubleValue ?? 1
        write(path, max(speed, slowest), "the speed")
        reload()
    }

    func setItemAltitudeMode(_ raw: Int) {
        if let refused = AltitudeMode.refusal(itemModes, raw: raw) {
            writeFailure = refused
            return
        }
        guard let item = items.first(where: \.isCurrent) else { return }
        write("plan.missionController.visualItems.\(item.index).altitudeMode", raw,
              "this item's altitude mode")
        reload()
    }

    func setDistanceMode(_ raw: Int) {
        if let refused = AltitudeMode.refusal(distanceModes, raw: raw) {
            writeFailure = refused
            return
        }
        guard let item = items.first(where: \.isCurrent) else { return }
        write("plan.missionController.visualItems.\(item.index).cameraCalc.distanceMode", raw,
              "the camera distance mode")
        reload()
    }

    func setFact(_ fact: ItemFact, to value: String) {
        guard !fact.readOnly else {
            writeFailure = FactWrite.readOnly
            return
        }
        guard let item = items.first(where: \.isCurrent) else { return }
        write("plan.missionController.visualItems.\(item.index).\(fact.pathSuffix)",
              Double(value) ?? value, fact.name)
        reload()
    }

    func loadCommands() {
        guard commandCategories.isEmpty, connected else { return }
        commandCategories = (Bridge.invoke("missionCommandTree.categoriesForVehicle",
                                           ["@vehicle"])["result"] as? [String]) ?? []
        showCategory(commandCategories.first ?? "")
    }

    func showCategory(_ category: String) {
        pickerCategory = category
        commands = MissionCommand.from(
            (Bridge.invoke("missionCommandTree.getCommandsForCategory",
                           ["@vehicle", category, true])["result"] as? [Any]) ?? [])
    }

    func pickCommand(for item: MissionItem) {
        loadCommands()
        showCategory(commandCategories.contains(item.category)
            ? item.category
            : commandCategories.first ?? "")
        pickingCommandFor = item.sequence
    }

    func setCommand(of item: MissionItem, to command: Int) {
        guard item.canChangeCommand else { return }
        if let centre = mapCentre {
            _ = Bridge.invoke(
                "plan.missionController.visualItems.\(item.index).setMapCenterHintForCommandChange",
                [["latitude": centre.latitude, "longitude": centre.longitude]])
        }
        write("plan.missionController.visualItems.\(item.index).command", command,
              "what this item does")
        pickingCommandFor = nil
        reload()
    }

    func save(to file: URL) {
        Bridge.invoke("plan.saveToFile", [file.path])
        reload()
    }

    func load(from file: URL) {
        Bridge.invoke("plan.loadFromFile", [file.path])
        reload()
    }

    func removeAll() {
        Bridge.invoke("plan.removeAll")
        reload()
    }

    // Reading the status never prompts; only asking for authorisation does, and QGC's own
    // position manager is what asks.
    private static let locationProbe = CLLocationManager()

    static var locationAccess: LocationAccess {
        switch locationProbe.authorizationStatus {
        case .notDetermined: return .notAsked
        case .denied, .restricted: return .refused
        case .authorized, .authorizedAlways: return .waiting
        @unknown default: return .unknown
        }
    }

    // Built once: every centreState call reads the bridge, and asking six times over could
    // report a menu no single moment ever showed.
    private func centreProbe() -> [String: Any] {
        let points = FenceRallyStore.planPoints()
        let state = centreState(fence: points.fence, rally: points.rally)
        return ["open": centreMenuOpen,
                "focused": focus != nil,
                "access": "\(MissionStore.locationAccess)",
                "enabled": MapCentre.allCases.filter { $0.enabled(in: state) }.map(\.title),
                "notes": MapCentre.allCases
                    .map { [$0.title, $0.note(in: state)] }
                    .filter { !$0[1].isEmpty }]
    }

    static func frameProbe(_ items: [MissionItem]) -> [String: Any] {
        let placed = items.filter(\.hasPosition)
        let frame = MapFrame(latitudes: placed.compactMap(\.latitude),
                             longitudes: placed.compactMap(\.longitude))
        return ["centreLatitude": frame.centreLatitude, "centreLongitude": frame.centreLongitude,
                "latitudeDelta": frame.latitudeDelta, "longitudeDelta": frame.longitudeDelta]
    }

    func centreState(fence: [GeoPoint], rally: [GeoPoint]) -> MapCentreState {
        var read = MapCentreState()
        read.missionPoints = items.filter(\.hasPosition).compactMap { item in
            guard let latitude = item.latitude, let longitude = item.longitude else { return nil }
            return GeoPoint(latitude: latitude, longitude: longitude)
        }
        read.otherPoints = fence + rally + surveyAreas.flatMap { $0 }
        read.launch = items.first.flatMap { item in
            guard let latitude = item.latitude, let longitude = item.longitude,
                  latitude != 0 || longitude != 0 else { return nil }
            return GeoPoint(latitude: latitude, longitude: longitude)
        }
        read.vehicle = vehiclePosition.flatMap { marker in
            guard marker.latitude != 0 || marker.longitude != 0 else { return nil }
            return GeoPoint(latitude: marker.latitude, longitude: marker.longitude)
        }
        read.gcs = MapCentre.usable(Bridge.group("positionManager.gcsPosition"))
        read.access = MissionStore.locationAccess
        return read
    }

    func centre(_ choice: MapCentre, fence: [GeoPoint], rally: [GeoPoint]) {
        centreMenuOpen = false
        guard let built = choice.frame(in: centreState(fence: fence, rally: rally)) else { return }
        focus = built
    }

    func centre(latitude: Double, longitude: Double) {
        centreMenuOpen = false
        guard let built = MapCentre.frame(latitude: latitude, longitude: longitude) else { return }
        focus = built
    }

    var mapCentre: GeoPoint? {
        guard let centre = MissionMap.lastRender["plan"]?["centre"] as? [String: Double],
              let latitude = centre["lat"], let longitude = centre["lon"] else { return nil }
        return GeoPoint(latitude: latitude, longitude: longitude)
    }

    func createPlan(_ kind: MissionItemKind?) -> String? {
        Bridge.invoke("plan.removeAll")

        guard let kind, let complex = kind.complexName else {
            reload()
            return nil
        }
        guard let centre = mapCentre else {
            reload()
            return "The map has not settled yet, so there is nowhere to put the plan."
        }

        let at = ["latitude": centre.latitude, "longitude": centre.longitude]
        Bridge.invoke("plan.missionController.insertTakeoffItem", [at, -1, false])
        Bridge.invoke("plan.missionController.insertComplexMissionItem", [complex, at, -1, false])
        Bridge.invoke("plan.missionController.insertLandItem", [at, -1, false])
        reload()

        guard let pattern = items.first(where: { $0.command == complex }) else {
            return "\(kind.title) could not be added to the plan."
        }
        seed(kind, at: pattern.index,
             latitude: centre.latitude, longitude: centre.longitude)

        if let takeoff = items.first(where: { $0.isLaunch && $0.sequence > 0 }) {
            Bridge.invoke("plan.missionController.setCurrentPlanViewSeqNum",
                          [takeoff.sequence, true])
        }
        reload()
        return nil
    }

    func exportKml(to file: URL) {
        Bridge.invoke("plan.saveToKml", [file.path])
    }

    func importShape(_ kind: MissionItemKind, from file: URL) -> String? {
        guard let complex = kind.complexName else {
            return "\(kind.title) is not drawn from a shape file."
        }
        Bridge.invoke("plan.missionController.insertComplexMissionItemFromKMLOrSHP",
                      [complex, file.path, items.count, true])
        reload()

        guard let placed = items.last, placed.command == complex else {
            return "\(file.lastPathComponent) added nothing to the plan."
        }
        let vertices = max(surveyPolygon(of: placed).count, corridorPath(of: placed).count)
        guard vertices >= 2 else {
            remove(placed)
            return "\(file.lastPathComponent) holds no \(kind.shapeNoun) for a \(kind.title.lowercased())."
        }
        return nil
    }

    var planName: String {
        planFile.isEmpty ? "Untitled" : URL(fileURLWithPath: planFile).deletingPathExtension().lastPathComponent
    }

    func select(_ item: MissionItem) {
        guard !item.isCurrent else { return }
        Bridge.invoke("plan.missionController.setCurrentPlanViewSeqNum", [item.sequence, true])
        reload()
    }

    func select(sequence: Int) {
        guard let item = items.first(where: { $0.sequence == sequence }) else { return }
        select(item)
    }

    func remove(_ item: MissionItem) {
        guard item.canRemove else { return }
        Bridge.invoke("plan.missionController.removeVisualItem", [item.index])
        reload()
    }

    func setItemSpeedSpecified(_ specified: Bool) {
        guard let item = items.first(where: \.isCurrent) else { return }
        write("plan.missionController.visualItems.\(item.index).speedSection.specifyFlightSpeed",
              specified, "whether this item sets its own speed")
        reload()
    }

    func setItemSpeed(_ value: Double) {
        guard let item = items.first(where: \.isCurrent) else { return }
        write("plan.missionController.visualItems.\(item.index).speedSection.\(ItemSpeed.property)",
              value, "this item's speed")
        reload()
    }

    // Cooked, like every Fact write: feet when the operator works in feet.
    func setAltitude(of item: MissionItem, value: Double) {
        write("plan.missionController.visualItems.\(item.index).altitude", value,
              "this item's altitude")
        reload()
    }

    private func overlayTile(x: Int, y: Int, z: Int, type: String, includeData: Bool) -> [String: Any] {
        let before = (CachedTileOverlay.served, CachedTileOverlay.fromParent,
                      CachedTileOverlay.fromChildren, CachedTileOverlay.missed)
        let overlay = CachedTileOverlay(mapType: type)
        var tileData: Data?
        var answered = false

        overlay.loadTile(at: MKTileOverlayPath(x: x, y: y, z: z, contentScaleFactor: 1)) { data, _ in
            tileData = data
            answered = true
        }

        for _ in 0..<200 where !answered {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }

        let source = CachedTileOverlay.served > before.0 ? "exact"
            : CachedTileOverlay.fromParent > before.1 ? "parent"
            : CachedTileOverlay.fromChildren > before.2 ? "children"
            : "miss"
        return ["ok": answered, "x": x, "y": y, "z": z, "type": type,
                "bytes": tileData?.count ?? 0, "source": source,
                "base64": includeData ? (tileData?.base64EncodedString() ?? "") : ""]
    }

    private final class TileProbe {
        var bytes = 0
        var answered = false
    }

    private func fetchTile(x: Int, y: Int, z: Int, type: String) -> [String: Any] {
        let probe = TileProbe()
        let box = Unmanaged.passRetained(probe).toOpaque()

        qgc_map_tile_fetch(type, Int32(x), Int32(y), Int32(z), { bytes, length, context in
            guard let context else { return }
            let probe = Unmanaged<TileProbe>.fromOpaque(context).takeRetainedValue()
            probe.bytes = Int(length)
            probe.answered = true
        }, box)

        for _ in 0..<100 where !probe.answered {
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }

        return ["ok": probe.answered, "type": type, "x": x, "y": y, "z": z,
                "bytes": probe.bytes, "cached": probe.bytes > 0]
    }

    func probeState() -> [String: Any] {
        ["count": items.count, "syncing": syncing, "status": status,
         "dirty": dirty, "connected": connected,
         "renderers": ["calls": MissionMap.rendererCalls,
                       "kinds": MissionMap.rendererKinds.sorted()],
         "tiles": ["requested": CachedTileOverlay.requested,
                   "secondsSinceRequest": CachedTileOverlay.lastRequest
                       .map { Int(Date().timeIntervalSince($0)) } ?? -1,
                   "exact": CachedTileOverlay.served,
                   "fromParent": CachedTileOverlay.fromParent,
                   "fromChildren": CachedTileOverlay.fromChildren,
                   "missed": CachedTileOverlay.missed],
         "placed": items.filter(\.hasPosition).count,
         "vehiclePlaced": vehiclePosition != nil,
         "map": MissionMap.lastRender["plan"] ?? [:],
         "frame": MissionStore.frameProbe(items),
         "commandCategories": commandCategories,
         "pickerCategory": pickerCategory,
         "pickingCommandFor": pickingCommandFor ?? -1,
         "commandNames": commands.map(\.name),
         "commandsWithSummary": commands.filter { !$0.summary.isEmpty }.count,
         "selected": items.first(where: \.isCurrent)?.sequence ?? -1,
         "itemSpeed": ["available": selectedSpeed.available,
                       "specified": selectedSpeed.specified,
                       "value": selectedSpeed.value ?? -1,
                       "units": selectedSpeed.units],
         "arming": arming ?? "",
         "patterns": patterns,
         "planFile": planFile, "watching": watchPoll != nil, "planName": planName,
         "readyToSave": readyToSave, "notReadyReason": notReadyReason,
         "uploadCheckable": preCheck() != nil,
         "uploadWarning": uploadWarning.map(\.refusal) ?? "",
         "writeFailure": writeFailure ?? "",
         "canUndo": canUndo, "canRedo": canRedo,
         "offersUndo": offersUndo, "offersRedo": offersRedo, "offersDownload": offersDownload,
         "commands": commands.map(\.name),
         "surveys": surveyAreas.map(\.count),
         "corridors": corridorPaths.map(\.count),
         "surveyStats": ["shots": surveyStats.shotsText, "distance": surveyStats.distanceText, "interval": surveyStats.intervalText,
                         "area": surveyStats.areaText, "footprint": surveyStats.footprintText,
                         "warning": surveyStats.warning],
         "distanceMode": AltitudeMode.title(for: distanceMode, in: distanceModes),
         "itemAltitudeMode": AltitudeMode.title(for: itemAltitudeMode, in: itemModes),
         "globalAltitudeMode": AltitudeMode.title(for: globalAltitudeMode, in: missionModes),
         "altitudeModes": ["mission": missionModes.map(\.raw), "item": itemModes.map(\.raw),
                           "distance": distanceModes.map(\.raw),
                           "missionChoosable": AltitudeMode.choosable(missionModes).map(\.raw),
                           "missionRefusals": missionModes.filter { !$0.enabled }.map(\.reason)],
         "defaultAltitude": defaultAltitude,
         "scale": scaleBar.text,
         "polygons": editablePolygons.map { ["path": $0.path, "vertices": $0.points.count,
                                             "canRemove": $0.canRemoveVertex,
                                             "minimumVertices": $0.minimumVertices,
                                             "hint": PolygonEdit.removalHint($0),
                                             "ring": $0.ring, "segments": $0.segments,
                                             "split": $0.splitInvokable] },
         "centre": centreProbe(),
         "launch": ["editable": launch.editable, "altitude": launch.altitudeText,
                    "position": launch.positionText],
         "vehicle": ["firmware": vehicle.firmware, "type": vehicle.type,
                     "cruiseSpeed": vehicle.showsCruiseSpeed ? cruiseSpeed : "",
                     "hoverSpeed": vehicle.showsHoverSpeed ? hoverSpeed : ""],
         "summary": ["distance": summary.distanceText, "duration": summary.durationText,
                     "telemetry": summary.telemetryText, "hasFlight": summary.hasFlight],
         "camera": ["brand": camera.brand, "model": camera.model,
                    "brands": camera.brands.count, "models": camera.models.count,
                    "describes": camera.describes],
         "facts": selectedFacts.map {
             ["name": $0.name, "title": $0.title, "value": $0.value, "units": $0.units,
              "group": $0.group, "path": $0.pathSuffix]
         },
         "terrain": ["points": terrain.points.count, "usable": terrain.usable,
                     "collision": terrain.hasCollision,
                     "unknown": terrain.unknownTerrain,
                     "distance": terrain.distanceText,
                     "min": terrain.minAltitude, "max": terrain.maxAltitude],
         "items": items.prefix(8).map {
             ["seq": $0.sequence, "command": $0.command, "selected": $0.isCurrent,
              "position": $0.positionText, "altitude": $0.altitudeText]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "tile":
            return fetchTile(x: Int(args["x"] ?? "") ?? 0,
                             y: Int(args["y"] ?? "") ?? 0,
                             z: Int(args["z"] ?? "") ?? 0,
                             type: args["type"] ?? CachedTileOverlay.currentMapType())
        case "overlayTile":
            return overlayTile(x: Int(args["x"] ?? "") ?? 0,
                               y: Int(args["y"] ?? "") ?? 0,
                               z: Int(args["z"] ?? "") ?? 0,
                               type: args["type"] ?? CachedTileOverlay.currentMapType(),
                               includeData: args["data"] != nil)
        // No upload action. uploadToVehicle() sends the plan outright whenever the core says
        // canSend, so an action wrapping it was a probe that could fly a plan onto a vehicle.
        // uploadPreCheck answers the readable half - canSend and the refusal - and sends nothing.
        case "addWaypoint":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "addWaypoint needs latitude and longitude"]
            }
            addWaypoint(latitude: latitude, longitude: longitude)
        case "importShape":
            guard let kind = kinds.byId(args["kind"] ?? ""),
                  let path = args["file"] else {
                return ["ok": false, "error": "importShape needs kind and file"]
            }
            if let failure = importShape(kind, from: URL(fileURLWithPath: path)) {
                return ["ok": false, "error": failure]
            }
        case "exportKml":
            guard let path = args["file"] else {
                return ["ok": false, "error": "exportKml needs file"]
            }
            exportKml(to: URL(fileURLWithPath: path))
        case "undo":
            guard offersUndo else {
                return ["ok": false, "error": syncing ? "the plan is syncing" : "nothing to undo"]
            }
            undo()
        case "redo":
            guard offersRedo else {
                return ["ok": false, "error": syncing ? "the plan is syncing" : "nothing to redo"]
            }
            redo()
        case "editing":
            args["on"] == "0" ? stopEditing() : startEditing()
        case "arm":
            arming = args["on"] == "0" ? nil : (args["kind"] ?? "waypoint")
        case "move":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "move needs latitude and longitude"]
            }
            let sequence = Int(args["sequence"] ?? "") ?? -1
            guard let target = items.first(where: { $0.sequence == sequence }) else {
                return ["ok": false, "error": "no item with that sequence"]
            }
            guard target.canRemove else {
                return ["ok": false, "error": "\(target.command) cannot be moved"]
            }
            move(sequence: sequence, latitude: latitude, longitude: longitude)
        case "failWrite":
            write(args["path"] ?? "plan.missionController.visualItems.0.commandName",
                  args["value"] ?? "x", args["what"] ?? "this item")
        case "itemSpeed":
            if let on = args["on"] { setItemSpeedSpecified(on != "0") }
            if let value = Double(args["value"] ?? "") { setItemSpeed(value) }
        case "uploadPreCheck":
            let check = preCheck()
            if args["show"] == "1" { uploadWarning = check?.canSend == true ? nil : check }
            return ["ok": true, "canSend": check?.canSend ?? false,
                    "refusal": check?.refusal ?? "", "state": probeState()]
        case "createPlan":
            let kind = kinds.byId(args["kind"] ?? "")
            if let failure = createPlan(kind) {
                return ["ok": false, "error": failure]
            }
        case "pickCommand":
            guard let target = items.first(where: {
                $0.sequence == Int(args["sequence"] ?? "") ?? -1
            }) else {
                return ["ok": false, "error": "no item at that sequence"]
            }
            pickCommand(for: target)
        case "commandCategory":
            loadCommands()
            showCategory(args["category"] ?? pickerCategory)
        case "setCommand":
            guard let target = items.first(where: { $0.sequence == Int(args["sequence"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no item with that sequence"]
            }
            guard target.canChangeCommand else {
                return ["ok": false, "error": "\(target.command) cannot change its command"]
            }
            setCommand(of: target, to: Int(args["command"] ?? "") ?? 0)
        case "setGlobalAltitudeMode":
            setGlobalAltitudeMode(Int(args["value"] ?? "") ?? AltitudeMode.none)
        case "setDefaultAltitude":
            setDefaultAltitude(args["value"] ?? "")
        case "setLaunchAltitude":
            guard let value = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "setLaunchAltitude needs a value"]
            }
            setLaunchAltitude(value)
        case "moveVertex":
            guard let which = Int(args["which"] ?? ""), let vertex = Int(args["vertex"] ?? ""),
                  let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? ""),
                  editablePolygons.indices.contains(which) else {
                return ["ok": false, "error": "moveVertex needs which, vertex, latitude, longitude"]
            }
            moveVertex(editablePolygons[which], vertex, latitude: latitude, longitude: longitude)
        case "removeVertex":
            guard let which = Int(args["which"] ?? ""), let vertex = Int(args["vertex"] ?? ""),
                  editablePolygons.indices.contains(which) else {
                return ["ok": false, "error": "removeVertex needs which and vertex"]
            }
            let polygon = editablePolygons[which]
            guard polygon.points.indices.contains(vertex) else {
                return ["ok": false, "error": "no corner \(vertex) on \(polygon.path)"]
            }
            guard PolygonEdit.removes(vertex, in: polygon) else {
                return ["ok": false, "error": PolygonEdit.removalHint(polygon)]
            }
            removeVertex(polygon, vertex)
        case "splitSegment":
            guard let which = Int(args["which"] ?? ""), let vertex = Int(args["vertex"] ?? ""),
                  editablePolygons.indices.contains(which) else {
                return ["ok": false, "error": "splitSegment needs which and vertex"]
            }
            splitSegment(editablePolygons[which], after: vertex)
        case "centreMenu":
            centreMenuOpen = args["open"] != "0"
        case "centre":
            guard let choice = MapCentre(rawValue: args["which"] ?? "") else {
                return ["ok": false, "error": "centre needs a known destination"]
            }
            centre(choice, fence: [], rally: [])
        case "centreAt":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "centreAt needs latitude and longitude"]
            }
            centre(latitude: latitude, longitude: longitude)
        case "setLaunchToMapCentre":
            setLaunchToMapCentre()
        case "setCruiseSpeed":
            setCruiseSpeed(args["value"] ?? "")
        case "setHoverSpeed":
            setHoverSpeed(args["value"] ?? "")
        case "setItemAltitudeMode":
            setItemAltitudeMode(Int(args["value"] ?? "") ?? AltitudeMode.none)
        case "setDistanceMode":
            setDistanceMode(Int(args["value"] ?? "") ?? AltitudeMode.none)
        case "setCameraBrand":
            setCamera(brand: args["value"] ?? "")
        case "setCameraModel":
            setCamera(model: args["value"] ?? "")
        case "setFact":
            guard let fact = selectedFacts.first(where: { $0.name == args["name"] }) else {
                return ["ok": false, "error": "the selected item has no fact named \(args["name"] ?? "")"]
            }
            setFact(fact, to: args["value"] ?? "")
        case "save":
            guard let path = args["path"] else { return ["ok": false, "error": "save needs a path"] }
            save(to: URL(fileURLWithPath: path))
        case "load":
            guard let path = args["path"] else { return ["ok": false, "error": "load needs a path"] }
            load(from: URL(fileURLWithPath: path))
        case "removeAll":
            removeAll()
        case "select":
            select(sequence: Int(args["sequence"] ?? "") ?? -1)
        case "remove":
            guard let target = items.first(where: { $0.sequence == Int(args["sequence"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no item with that sequence"]
            }
            guard target.canRemove else {
                return ["ok": false, "error": "\(target.command) cannot be removed"]
            }
            remove(target)
        case "setAltitude":
            guard let target = items.first(where: { $0.index == Int(args["index"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no item at that index"]
            }
            setAltitude(of: target, value: Double(args["value"] ?? "") ?? 0)
        case "download":
            guard offersDownload else {
                return ["ok": false,
                        "error": syncing ? "the plan is syncing" : "no vehicle is connected"]
            }
            downloadFromVehicle()
            for _ in 0..<100 where syncing {
                RunLoop.current.run(until: Date().addingTimeInterval(0.1))
                syncing = (Bridge.group("plan")["syncInProgress"] as? NSNumber)?.boolValue ?? false
            }
            reload()
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
