import Foundation

final class FenceRallyStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "fenceRally"

    @Published private(set) var shapes: [FenceShape] = []
    @Published private(set) var rallyPoints: [RallyPointRow] = []
    @Published private(set) var fence = FenceSupport.unread
    var fenceSupported: Bool { fence.offers }
    @Published private(set) var rally = FenceSupport.unread
    var rallySupported: Bool { rally.offers }
    @Published private(set) var connected = false
    @Published private(set) var breachReturn: RallyPointRow?
    @Published private(set) var firmwareFence: FirmwareFence?
    @Published private(set) var breachRange = FactRange(control: [:], title: BreachReturn.altitudeSubject)
    @Published private(set) var breachDecimals: Int?
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published var armingRally = false
    @Published private(set) var breachAltitude: Double?
    @Published private(set) var breachAltitudeMetres: Double?
    @Published private(set) var breachAltitudeUnits = "m"
    @Published var writeFailure: String?

    private var watchPoll: Timer?

    private var watchers = 0

    func startWatching() {
        watchers += 1
        guard watchPoll == nil else { return }
        reload()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.reload()
        }
    }

    func stopWatching() {
        watchers = max(0, watchers - 1)
        guard watchers == 0 else { return }
        watchPoll?.invalidate()
        watchPoll = nil
    }

    private(set) var reloads = 0
    private(set) var publishes = 0

    func reload() {
        reloads += 1
        let fences = Bridge.group("view.fences")
        guard (fences["available"] as? NSNumber)?.boolValue == true else {
            set(\.status, "The plan is not available.")
            set(\.shapes, [])
            set(\.rallyPoints, [])
            set(\.fence, .unread)
            set(\.rally, .unread)
            return
        }
        set(\.status, "")

        let planView = Bridge.group("view.plan")
        set(\.connected, FlyState(Bridge.group("view.flyState")).connected)
        set(\.fence, FenceSupport(answer: fences["fenceSupported"]))
        set(\.rally, FenceSupport(answer: fences["rallySupported"]))

        set(\.shapes, FenceRallyStore.readShapes())
        set(\.firmwareFence, FirmwareFence(fences["firmwareFence"]))
        set(\.rallyPoints, FenceRallyStore.readRally())

        set(\.breachReturn, (fences["breachReturnPoint"] as? [String: Any])
            .map { RallyPointRow(coordinate: $0) })
        let breachFact = Bridge.group("view.control(plan.geoFenceController.breachReturnAltitude)")
        set(\.breachAltitude, BreachReturn.shownAltitude(breachFact))
        set(\.breachAltitudeMetres, BreachReturn.altitudeMetres(breachFact))
        set(\.breachAltitudeUnits, BreachReturn.altitudeUnits(breachFact))
        set(\.breachRange, BreachReturn.range(breachFact))
        set(\.breachDecimals, BreachReturn.decimals(breachFact))

        set(\.syncing, PlanSync.busy(planView["sync"]))
    }

    private func set<T: Equatable>(_ key: ReferenceWritableKeyPath<FenceRallyStore, T>, _ value: T) {
        guard self[keyPath: key] != value else { return }
        self[keyPath: key] = value
        publishes += 1
    }

    var mapCentre: GeoPoint? {
        (MissionMap.lastRender["plan"]?["centre"] as? [String: Double])
            .flatMap { GeoPoint(json: ["latitude": $0["lat"] ?? .nan,
                                       "longitude": $0["lon"] ?? .nan]) }
    }

    var mapWindow: MapWindow? {
        let render = MissionMap.lastRender["plan"] ?? [:]
        return MapWindow(centre: mapCentre,
                         latitudeSpan: render["spanLat"] as? Double,
                         longitudeSpan: render["spanLon"] as? Double)
    }

    func addFence(circle: Bool) -> String? {
        if !fence.read { reload() }
        if let refusal = fence.refusal() { return refusal }
        guard let window = mapWindow else {
            return "The map has not settled yet, so there is nowhere to put a fence."
        }
        let corners = [
            ["latitude": window.topLeft.latitude, "longitude": window.topLeft.longitude],
            ["latitude": window.bottomRight.latitude, "longitude": window.bottomRight.longitude],
        ]
        Bridge.invoke("plan.geoFenceController.\(circle ? "addInclusionCircle" : "addInclusionPolygon")",
                      corners)
        reload()

        guard let added = shapes.last else {
            return "The fence could not be added to the plan."
        }
        guard added.usable else {
            remove(added)
            return "The plan is still settling after its download; try the fence again in a moment."
        }
        return nil
    }

    private func circlePath(_ shape: FenceShape) -> String? {
        shape.isCircle ? shape.path : nil
    }

    func setRadius(_ shape: FenceShape, value: Double) {
        if let refused = shape.radiusRefusal(value) {
            writeFailure = refused
            return
        }
        guard let path = circlePath(shape) else { return }
        write("\(path).radius", value, "the fence radius")
        reload()
    }

    func setRallyAltitude(_ point: RallyPointRow, value: Double) {
        guard !point.altitudePath.isEmpty else { return }
        write(point.altitudePath, value, "the rally point height")
        reload()
    }

    func addBreachReturn() -> String? {
        if !fence.read { reload() }
        if let refusal = fence.refusal() { return refusal }
        guard let centre = mapCentre, let altitude = breachAltitudeMetres else {
            return BreachReturn.refusal(haveMap: mapCentre != nil)
        }
        write("plan.geoFenceController.breachReturnPoint",
              ["latitude": centre.latitude, "longitude": centre.longitude,
               "altitude": altitude], "the breach return point")
        reload()
        return BreachReturn.outcome(placed: breachReturn != nil)
    }

    func setBreachAltitude(_ value: Double) {
        if let refused = breachRange.refusal(value) {
            writeFailure = refused
            return
        }
        write("plan.geoFenceController.breachReturnAltitude", value,
              "the breach return altitude")
        reload()
    }

    func setInclusion(_ shape: FenceShape, to inclusion: Bool) {
        write("\(shape.path).inclusion", inclusion, "whether the fence keeps in or out")
        reload()
    }

    func remove(_ shape: FenceShape) {
        if shape.isCircle {
            Bridge.invoke("plan.geoFenceController.deleteCircle", [shape.index])
        } else {
            Bridge.invoke("plan.geoFenceController.deletePolygon", [shape.index])
        }
        reload()
    }

    func addRallyPoint(latitude: Double, longitude: Double) {
        guard rallySupported else { return }
        Bridge.invoke("plan.rallyPointController.addPoint",
                      [["latitude": latitude, "longitude": longitude]])
        armingRally = false
        reload()
    }

    func remove(_ point: RallyPointRow) {
        Bridge.invoke("plan.rallyPointController.removePoint",
                      ["@plan.rallyPointController.points.\(point.id)"])
        reload()
    }

    var offersDownload: Bool { connected && !syncing }

    func downloadFromVehicle() {
        guard offersDownload else { return }
        Bridge.invoke("plan.loadFromVehicle")
        syncing = true
        reload()
    }

    var framingPoints: [GeoPoint] { shapes.flatMap(\.framingPoints) }

    var rallyGeoPoints: [GeoPoint] { FenceRallyStore.geoPoints(rallyPoints) }

    static func geoPoints(_ rows: [RallyPointRow]) -> [GeoPoint] {
        rows.compactMap { row in
            guard let latitude = row.latitude, let longitude = row.longitude else { return nil }
            return GeoPoint(latitude: latitude, longitude: longitude)
        }
    }

    static func readShapes() -> [FenceShape] {
        let view = Bridge.group("view.fences")
        return FenceShape.list(view["polygons"]) + FenceShape.list(view["circles"])
    }

    static func readRally() -> [RallyPointRow] {
        RallyPointRow.list(Bridge.group("view.fences")["rallyPoints"])
    }

    static func planPoints() -> (fence: [GeoPoint], rally: [GeoPoint]) {
        (readShapes().flatMap(\.framingPoints), geoPoints(readRally()))
    }


    func probeState() -> [String: Any] {
        ["shapes": shapes.count, "rallyPoints": rallyPoints.count,
         "firmwareFence": firmwareFence.map { ["radius": $0.radiusText,
                                               "drawable": $0.drawable] } ?? [:],
         "reloads": reloads, "publishes": publishes, "watching": watchPoll != nil,
         "fenceSupported": fenceSupported, "rallySupported": rallySupported,
         "connected": connected, "offersDownload": offersDownload,
         "breachReturn": breachReturn?.positionText ?? "none",
         "breachAltitude": breachAltitude ?? -1,
         "breachAltitudeMetres": breachAltitudeMetres ?? -1,
         "breachAltitudeUnits": breachAltitudeUnits,
         "status": status, "syncing": syncing, "armingRally": armingRally,
         "writeFailure": writeFailure ?? "",
         "map": MissionMap.lastRender["plan"] ?? [:],
         "fence": shapes.prefix(8).map {
             ["kind": $0.kindText, "detail": $0.detailText, "centre": $0.centreText,
              "vertices": $0.vertices.count]
         },
         "rally": rallyPoints.prefix(8).map {
             ["position": $0.positionText, "altitude": $0.altitudeText]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "download":
            guard offersDownload else {
                return ["ok": false,
                        "error": syncing ? "the plan is syncing" : "no vehicle is connected"]
            }
            downloadFromVehicle()
        case "addFence":
            if let failure = addFence(circle: args["circle"] == "1") {
                return ["ok": false, "error": failure]
            }
        case "addRally":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "addRally needs latitude and longitude"]
            }
            addRallyPoint(latitude: latitude, longitude: longitude)
        case "setRadius":
            guard let shape = shapes.first(where: { $0.id == (args["which"] ?? "") }),
                  let value = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "setRadius needs a circle id and a value in the "
                    + "units the fence is shown in, which is not always metres"]
            }
            setRadius(shape, value: value)
        case "setRallyAltitude":
            guard let point = rallyPoints.first(where: { $0.id == Int(args["which"] ?? "") ?? -1 }),
                  let value = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "setRallyAltitude needs a point id and a value in "
                    + "the units the point is shown in, which is not always metres"]
            }
            setRallyAltitude(point, value: value)
        case "addBreachReturn":
            if let failure = addBreachReturn() {
                return ["ok": false, "error": failure]
            }
        case "setBreachAltitude":
            guard let value = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "setBreachAltitude needs a value in the units the "
                    + "breach return is shown in, which is not always metres"]
            }
            setBreachAltitude(value)
        case "setInclusion":
            guard let shape = shapes.first(where: { $0.id == (args["which"] ?? "") }) else {
                return ["ok": false, "error": "no fence shape with that id"]
            }
            setInclusion(shape, to: args["on"] != "0")
        case "removeFence":
            guard let shape = shapes.first(where: { $0.id == (args["which"] ?? "") }) else {
                return ["ok": false, "error": "no fence shape with that id"]
            }
            remove(shape)
        case "removeRally":
            guard let point = rallyPoints.first(where: { $0.id == Int(args["which"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no rally point with that id"]
            }
            remove(point)
        case "armRally":
            armingRally = args["on"] != "0"
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
