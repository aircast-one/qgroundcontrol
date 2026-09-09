import Foundation

final class FenceRallyStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "fenceRally"

    @Published private(set) var shapes: [FenceShape] = []
    @Published private(set) var rallyPoints: [RallyPointRow] = []
    @Published private(set) var fenceSupported = false
    @Published private(set) var rallySupported = false
    @Published private(set) var connected = false
    @Published private(set) var breachReturn: RallyPointRow?
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published var armingRally = false
    @Published private(set) var breachAltitude: Double?
    @Published private(set) var breachAltitudeUnits = "m"
    @Published var writeFailure: String?

    func reload() {
        let fence = Bridge.group("plan.geoFenceController")
        guard fence["kind"] as? String == "object" else {
            status = "The plan is not available."
            shapes = []
            rallyPoints = []
            fenceSupported = false
            rallySupported = false
            return
        }
        status = ""

        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        fenceSupported = (fence["supported"] as? NSNumber)?.boolValue ?? false
        rallySupported = (Bridge.group("plan.rallyPointController")["supported"] as? NSNumber)?.boolValue ?? false

        shapes = FenceRallyStore.readShapes()
        rallyPoints = FenceRallyStore.readRally()

        breachReturn = (fence["breachReturnPoint"] as? [String: Any])
            .map { RallyPointRow(coordinate: $0) }
        let breachFact = Bridge.group("plan.geoFenceController.breachReturnAltitude")
        breachAltitude = (breachFact["value"] as? NSNumber)?.doubleValue
        breachAltitudeUnits = (breachFact["units"] as? String) ?? "m"

        syncing = (Bridge.group("plan")["syncInProgress"] as? NSNumber)?.boolValue ?? false
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
        guard fenceSupported else {
            return "This vehicle does not accept a geofence."
        }
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

    // A Fact write is COOKED, so this is whatever unit the operator is working in, not
    // metres. Naming it metres invites someone to convert a value that is already right.
    func setRadius(_ shape: FenceShape, value: Double) {
        guard let path = circlePath(shape), value > 0 else { return }
        write("\(path).radius", value, "the fence radius")
        reload()
    }

    func setRallyAltitude(_ point: RallyPointRow, value: Double) {
        guard !point.altitudePath.isEmpty else { return }
        write(point.altitudePath, value, "the rally point height")
        reload()
    }

    func addBreachReturn() -> String? {
        guard fenceSupported else { return "This vehicle does not accept a geofence." }
        guard let centre = mapCentre else {
            return "The map has not settled yet, so there is nowhere to put it."
        }

        write("plan.geoFenceController.breachReturnPoint",
              ["latitude": centre.latitude, "longitude": centre.longitude,
               "altitude": breachAltitude ?? 0], "the breach return point")
        reload()
        return breachReturn == nil ? "The breach return point was not accepted." : nil
    }

    func setBreachAltitude(_ value: Double) {
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

    func downloadFromVehicle() {
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

    // The mission probe reports the centre menu but cannot see this store, and a probe that
    // computes a field differently from the screen can never disagree with it. It reads through
    // the same two builders the view's state comes from, so a new shape kind reaches both.
    static func planPoints() -> (fence: [GeoPoint], rally: [GeoPoint]) {
        (readShapes().flatMap(\.framingPoints), geoPoints(readRally()))
    }


    func probeState() -> [String: Any] {
        ["shapes": shapes.count, "rallyPoints": rallyPoints.count,
         "fenceSupported": fenceSupported, "rallySupported": rallySupported,
         "connected": connected,
         "breachReturn": breachReturn?.positionText ?? "none",
         "breachAltitude": breachAltitude ?? -1,
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
        case "download": downloadFromVehicle()
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
