import Foundation

struct FenceShape: Identifiable {
    enum Form {
        case polygon(vertices: Int, area: Double)
        case circle(radius: Double)
    }

    let id: Int
    let inclusion: Bool
    let form: Form
    let latitude: Double?
    let longitude: Double?

    init(json: [String: Any], id: Int, circle: Bool) {
        self.id = id
        inclusion = (json["inclusion"] as? NSNumber)?.boolValue ?? true

        let center = json["center"] as? [String: Any]
        latitude = (center?["latitude"] as? NSNumber)?.doubleValue
        longitude = (center?["longitude"] as? NSNumber)?.doubleValue

        form = circle
            ? .circle(radius: ((json["facts"] as? [[String: Any]]) ?? [])
                .first { ($0["name"] as? String) == "Radius" }
                .flatMap { ($0["value"] as? NSNumber)?.doubleValue } ?? 0)
            : .polygon(vertices: (json["count"] as? NSNumber)?.intValue ?? 0,
                       area: (json["area"] as? NSNumber)?.doubleValue ?? 0)
    }

    var kindText: String {
        switch form {
        case .circle: return inclusion ? "Keep-in circle" : "Keep-out circle"
        case .polygon: return inclusion ? "Keep-in polygon" : "Keep-out polygon"
        }
    }

    var detailText: String {
        switch form {
        case let .circle(radius):
            return String(format: "%.0f m radius", radius)
        case let .polygon(vertices, area):
            let vertexText = "\(vertices) vertice\(vertices == 1 ? "" : "s")"
            return area > 0
                ? "\(vertexText) · \(FenceShape.areaText(area))"
                : vertexText
        }
    }

    var centreText: String {
        guard let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    static func areaText(_ area: Double) -> String {
        area >= 10000
            ? String(format: "%.2f km²", area / 1_000_000)
            : String(format: "%.0f m²", area)
    }
}

struct RallyPointRow: Identifiable {
    let id: Int
    let latitude: Double?
    let longitude: Double?
    let altitude: Double?

    init(json: [String: Any], id: Int) {
        self.id = id
        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        altitude = (coordinate?["altitude"] as? NSNumber)?.doubleValue
    }

    var positionText: String {
        guard let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    var altitudeText: String {
        guard let altitude, altitude.isFinite else { return "—" }
        return String(format: "%.1f m", altitude)
    }
}

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
         "status": status, "syncing": syncing,
         "fence": shapes.prefix(8).map {
             ["kind": $0.kindText, "detail": $0.detailText, "centre": $0.centreText]
         },
         "rally": rallyPoints.prefix(8).map {
             ["position": $0.positionText, "altitude": $0.altitudeText]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "download": downloadFromVehicle()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
