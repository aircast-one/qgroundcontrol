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
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
