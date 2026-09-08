import Foundation
import MapKit
import QGCMapTileC

final class MissionStore: ObservableObject, Probeable {
    static let probeID = "mission"

    @Published private(set) var items: [MissionItem] = []
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published private(set) var vehiclePosition: VehicleMarker?
    @Published private(set) var dirty = false
    @Published private(set) var connected = false
    @Published var arming: MissionItemKind?
    @Published private(set) var canUndo = false
    @Published private(set) var canRedo = false
    @Published private(set) var commands: [MissionCommand] = []
    @Published private(set) var commandCategories: [String] = []
    @Published var pickingCommandFor: Int?

    @Published private(set) var pickerCategory = ""
    @Published private(set) var selectedFacts: [ItemFact] = []
    @Published private(set) var surveyStats = SurveyStats.none
    @Published private(set) var camera = CameraChoice.empty
    @Published private(set) var distanceMode = ""
    @Published private(set) var itemAltitudeMode = ""
    @Published private(set) var globalAltitudeMode = ""
    @Published private(set) var defaultAltitude = ""
    @Published private(set) var summary = PlanSummary.empty
    @Published private(set) var vehicle = MissionVehicle.unknown
    @Published private(set) var cruiseSpeed = ""
    @Published private(set) var hoverSpeed = ""
    @Published private(set) var planFile = ""

    private var undoPoll: Timer?

    func reload() {
        let controller = Bridge.group("plan.missionController")
        guard controller["kind"] as? String == "object" else {
            status = "No vehicle is connected."
            items = []
            return
        }

        let model = Bridge.group("plan.missionController.visualItems")
        items = ((model["elements"] as? [[String: Any]]) ?? [])
            .enumerated().map { MissionItem(json: $0.element, index: $0.offset) }
        let plan = Bridge.group("plan")
        syncing = (plan["syncInProgress"] as? NSNumber)?.boolValue ?? false
        dirty = (plan["dirty"] as? NSNumber)?.boolValue ?? false
        planFile = (plan["currentPlanFile"] as? String) ?? ""
        globalAltitudeMode = (controller["globalAltitudeMode"] as? String) ?? ""

        let hover = (controller["missionHoverDistance"] as? NSNumber)?.doubleValue ?? 0
        let cruise = (controller["missionCruiseDistance"] as? NSNumber)?.doubleValue ?? 0
        summary = PlanSummary(
            distanceMetres: hover + cruise,
            seconds: (controller["missionTime"] as? NSNumber)?.doubleValue ?? 0,
            maxTelemetryMetres: (controller["missionMaxTelemetry"] as? NSNumber)?.doubleValue ?? 0)
        defaultAltitude = (Bridge.group("settings.appSettings.defaultMissionItemAltitude")["valueString"] as? String) ?? ""

        let controllerVehicle = Bridge.group("plan.controllerVehicle")
        vehicle = MissionVehicle(
            firmware: (controllerVehicle["firmwareTypeString"] as? String) ?? "",
            type: (controllerVehicle["vehicleTypeString"] as? String) ?? "",
            multiRotor: (controllerVehicle["multiRotor"] as? NSNumber)?.boolValue ?? false,
            vtol: (controllerVehicle["vtol"] as? NSNumber)?.boolValue ?? false)
        cruiseSpeed = (Bridge.group("settings.appSettings.offlineEditingCruiseSpeed")["valueString"] as? String) ?? ""
        hoverSpeed = (Bridge.group("settings.appSettings.offlineEditingHoverSpeed")["valueString"] as? String) ?? ""
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
        Bridge.invoke("plan.loadFromVehicle")
        syncing = true
        reload()
    }

    func uploadToVehicle() {
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

    func undo() {
        Bridge.invoke("plan.undo")
        reload()
    }

    func redo() {
        Bridge.invoke("plan.redo")
        reload()
    }

    func addWaypoint(latitude: Double, longitude: Double) {
        let kind = arming ?? .waypoint
        let index = items.count

        if let complex = kind.complexName {
            Bridge.invoke("plan.missionController.\(kind.invokable)",
                          [complex, ["latitude": latitude, "longitude": longitude], index, true])
            seed(kind, at: index, latitude: latitude, longitude: longitude)
        } else {
            Bridge.invoke("plan.missionController.\(kind.invokable)",
                          [["latitude": latitude, "longitude": longitude], index, true])
        }

        arming = nil
        reload()
    }

    private func seed(_ kind: MissionItemKind, at index: Int, latitude: Double, longitude: Double) {
        guard let plan = MissionItemKind.seed(for: kind, latitude: latitude, longitude: longitude)
        else { return }
        let path = "plan.missionController.visualItems.\(index).\(plan.property)"
        plan.points.forEach { point in
            Bridge.invoke("\(path).appendVertex",
                          [["latitude": point.latitude, "longitude": point.longitude]])
        }
    }

    private func loadSurveyStats(for item: MissionItem, calc: [String: Any]) {
        guard item.isSurveyItem else {
            if surveyStats != .none { surveyStats = .none }
            return
        }

        let survey = Bridge.group("plan.missionController.visualItems.\(item.index)")
        func number(_ json: [String: Any], _ key: String) -> Double {
            (json[key] as? NSNumber)?.doubleValue ?? 0
        }
        func factValue(_ json: [String: Any], _ property: String) -> Double {
            let facts = (json["facts"] as? [[String: Any]]) ?? []
            let match = facts.first { $0["property"] as? String == property }
            return (match?["value"] as? NSNumber)?.doubleValue ?? 0
        }

        let read = SurveyStats(
            shots: (survey["cameraShots"] as? NSNumber)?.intValue ?? 0,
            secondsBetweenShots: number(survey, "timeBetweenShots"),
            areaSquareMetres: number(survey, "coveredArea"),
            footprintSide: factValue(calc, "adjustedFootprintSide"),
            footprintFrontal: factValue(calc, "adjustedFootprintFrontal"),
            minimumInterval: factValue(calc, "minTriggerInterval"))
        if read != surveyStats { surveyStats = read }
    }

    private func surveyPolygon(of item: MissionItem) -> [GeoPoint] {
        guard let property = MissionItemKind.areaProperty(forCommand: item.command)
        else { return [] }
        let polygon = Bridge.group("plan.missionController.visualItems.\(item.index).\(property)")
        return ((polygon["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
    }

    private func corridorPath(of item: MissionItem) -> [GeoPoint] {
        guard let property = MissionItemKind.lineProperty(forCommand: item.command)
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

    var terrain: TerrainProfile {
        TerrainProfile(points: items.filter(\.hasPosition).map {
            TerrainPoint(distance: $0.distanceFromStart,
                         missionAltitude: $0.amslAltitude ?? 0,
                         terrainAltitude: $0.terrainAltitude,
                         collision: $0.terrainCollision)
        })
    }

    func move(sequence: Int, latitude: Double, longitude: Double) {
        guard let item = items.first(where: { $0.sequence == sequence }), item.canRemove else { return }
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).coordinate",
                       ["latitude": latitude, "longitude": longitude])
        reload()
    }

    private func loadSelectedFacts() {
        guard let item = items.first(where: \.isCurrent) else {
            selectedFacts = []
            if surveyStats != .none { surveyStats = .none }
            return
        }
        let listed = ItemFact.lists.flatMap { list in
            ItemFact.from(
                (Bridge.group("plan.missionController.visualItems.\(item.index).\(list)")["elements"] as? [Any]) ?? [],
                list: list)
        }

        let calc = item.isSimpleItem
            ? [:]
            : Bridge.group("plan.missionController.visualItems.\(item.index).cameraCalc")
        camera = CameraChoice(json: calc)
        loadSurveyStats(for: item, calc: calc)
        distanceMode = (calc["distanceMode"] as? String) ?? ""
        itemAltitudeMode = item.specifiesAltitude
            ? (Bridge.group("plan.missionController.visualItems.\(item.index)")["altitudeMode"] as? String) ?? ""
            : ""

        let cameraFacts = item.isSimpleItem ? [] : ItemFact.camera(
            (calc["facts"] as? [Any]) ?? [])

        selectedFacts = cameraFacts + (listed.isEmpty && !item.isSimpleItem
            ? ItemFact.owned((Bridge.group("plan.missionController.visualItems.\(item.index)")["facts"] as? [Any]) ?? [])
            : listed)
    }

    func setCamera(brand: String? = nil, model: String? = nil) {
        guard let item = items.first(where: \.isCurrent) else { return }
        let path = "plan.missionController.visualItems.\(item.index).cameraCalc"
        if let brand { _ = Bridge.set("\(path).cameraBrand", brand) }
        if let model { _ = Bridge.set("\(path).cameraModel", model) }
        reload()
    }

    func setGlobalAltitudeMode(_ raw: String) {
        guard AltitudeMode.isMissionChoice(raw) else { return }
        _ = Bridge.set("plan.missionController.globalAltitudeMode", raw)
        reload()
    }

    func setDefaultAltitude(_ value: String) {
        guard let metres = Double(value), metres.isFinite else { return }
        _ = Bridge.set("settings.appSettings.defaultMissionItemAltitude", metres)
        reload()
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
        _ = Bridge.set(path, max(speed, slowest))
        reload()
    }

    func setItemAltitudeMode(_ raw: String) {
        guard let item = items.first(where: \.isCurrent), AltitudeMode.isChoice(raw) else { return }
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).altitudeMode", raw)
        reload()
    }

    func setDistanceMode(_ raw: String) {
        guard let item = items.first(where: \.isCurrent), AltitudeMode.isChoice(raw) else { return }
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).cameraCalc.distanceMode", raw)
        reload()
    }

    func setFact(_ fact: ItemFact, to value: String) {
        guard let item = items.first(where: \.isCurrent) else { return }
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).\(fact.pathSuffix)",
                       Double(value) ?? value)
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
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).command", command)
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

    func setAltitude(of item: MissionItem, metres: Double) {
        _ = Bridge.set("plan.missionController.visualItems.\(item.index).altitude", metres)
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
         "commandCategories": commandCategories,
         "pickerCategory": pickerCategory,
         "pickingCommandFor": pickingCommandFor ?? -1,
         "commandNames": commands.map(\.name),
         "commandsWithSummary": commands.filter { !$0.summary.isEmpty }.count,
         "selected": items.first(where: \.isCurrent)?.sequence ?? -1,
         "arming": arming?.rawValue ?? "",
         "planFile": planFile, "planName": planName,
         "canUndo": canUndo, "canRedo": canRedo,
         "commands": commands.map(\.name),
         "surveys": surveyAreas.map(\.count),
         "corridors": corridorPaths.map(\.count),
         "surveyStats": ["shots": surveyStats.shotsText, "interval": surveyStats.intervalText,
                         "area": surveyStats.areaText, "footprint": surveyStats.footprintText,
                         "warning": surveyStats.warning],
         "distanceMode": distanceMode, "itemAltitudeMode": itemAltitudeMode,
         "globalAltitudeMode": globalAltitudeMode, "defaultAltitude": defaultAltitude,
         "vehicle": ["firmware": vehicle.firmware, "type": vehicle.type,
                     "cruiseSpeed": vehicle.showsCruiseSpeed ? cruiseSpeed : "",
                     "hoverSpeed": vehicle.showsHoverSpeed ? hoverSpeed : ""],
         "summary": ["distance": summary.distanceText, "duration": summary.durationText,
                     "telemetry": summary.telemetryText, "hasFlight": summary.hasFlight],
         "camera": ["brand": camera.brand, "model": camera.model,
                    "brands": camera.brands.count, "models": camera.models.count,
                    "describes": camera.describes],
         "facts": selectedFacts.map {
             ["name": $0.name, "value": $0.value, "units": $0.units, "group": $0.group, "path": $0.pathSuffix]
         },
         "terrain": ["points": terrain.points.count, "usable": terrain.usable,
                     "collision": terrain.hasCollision,
                     "unknown": terrain.unknownTerrain,
                     "distance": terrain.totalDistance,
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
        case "upload":
            uploadToVehicle()
            for _ in 0..<150 where syncing {
                RunLoop.current.run(until: Date().addingTimeInterval(0.1))
                syncing = (Bridge.group("plan")["syncInProgress"] as? NSNumber)?.boolValue ?? false
            }
            reload()
        case "addWaypoint":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "addWaypoint needs latitude and longitude"]
            }
            addWaypoint(latitude: latitude, longitude: longitude)
        case "importShape":
            guard let kind = MissionItemKind(rawValue: args["kind"] ?? ""),
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
            undo()
        case "redo":
            redo()
        case "editing":
            args["on"] == "0" ? stopEditing() : startEditing()
        case "arm":
            arming = args["on"] == "0" ? nil : MissionItemKind(rawValue: args["kind"] ?? "waypoint")
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
        case "createPlan":
            let kind = MissionItemKind(rawValue: args["kind"] ?? "")
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
            setGlobalAltitudeMode(args["value"] ?? "")
        case "setDefaultAltitude":
            setDefaultAltitude(args["value"] ?? "")
        case "setCruiseSpeed":
            setCruiseSpeed(args["value"] ?? "")
        case "setHoverSpeed":
            setHoverSpeed(args["value"] ?? "")
        case "setItemAltitudeMode":
            setItemAltitudeMode(args["value"] ?? "")
        case "setDistanceMode":
            setDistanceMode(args["value"] ?? "")
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
            setAltitude(of: target, metres: Double(args["metres"] ?? "") ?? 0)
        case "download":
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
