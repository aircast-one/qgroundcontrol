import Foundation

final class FenceRallyStore: ObservableObject, Probeable {
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

        let polygons = elements("plan.geoFenceController.polygons")
        let circles = elements("plan.geoFenceController.circles")
        shapes = polygons.enumerated().map { FenceShape(json: $0.element, id: $0.offset, circle: false) }
            + circles.enumerated().map {
                FenceShape(json: $0.element, id: polygons.count + $0.offset, circle: true)
            }

        rallyPoints = elements("plan.rallyPointController.points")
            .enumerated().map { RallyPointRow(json: $0.element, id: $0.offset) }

        breachReturn = (fence["breachReturnPoint"] as? [String: Any])
            .map { RallyPointRow(json: ["coordinate": $0], id: -1) }

        syncing = (Bridge.group("plan")["syncInProgress"] as? NSNumber)?.boolValue ?? false
    }

    var mapWindow: MapWindow? {
        let render = MissionMap.lastRender["plan"] ?? [:]
        let centre = (render["centre"] as? [String: Double])
            .flatMap { GeoPoint(json: ["latitude": $0["lat"] ?? .nan,
                                       "longitude": $0["lon"] ?? .nan]) }
        return MapWindow(centre: centre,
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

    func setInclusion(_ shape: FenceShape, to inclusion: Bool) {
        let polygons = elements("plan.geoFenceController.polygons").count
        let path = shape.radius != nil
            ? "plan.geoFenceController.circles.\(shape.id - polygons)"
            : "plan.geoFenceController.polygons.\(shape.id)"
        _ = Bridge.set("\(path).inclusion", inclusion)
        reload()
    }

    func remove(_ shape: FenceShape) {
        let polygons = elements("plan.geoFenceController.polygons").count
        if shape.radius != nil {
            Bridge.invoke("plan.geoFenceController.deleteCircle", [shape.id - polygons])
        } else {
            Bridge.invoke("plan.geoFenceController.deletePolygon", [shape.id])
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

    private func elements(_ path: String) -> [[String: Any]] {
        (Bridge.group(path)["elements"] as? [[String: Any]]) ?? []
    }

    func probeState() -> [String: Any] {
        ["shapes": shapes.count, "rallyPoints": rallyPoints.count,
         "fenceSupported": fenceSupported, "rallySupported": rallySupported,
         "connected": connected,
         "breachReturn": breachReturn?.positionText ?? "none",
         "status": status, "syncing": syncing, "armingRally": armingRally,
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
        case "setInclusion":
            guard let shape = shapes.first(where: { $0.id == Int(args["which"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no fence shape with that id"]
            }
            setInclusion(shape, to: args["on"] != "0")
        case "removeFence":
            guard let shape = shapes.first(where: { $0.id == Int(args["which"] ?? "") ?? -1 }) else {
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
