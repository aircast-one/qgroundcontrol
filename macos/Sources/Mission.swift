import Foundation

final class MissionStore: ObservableObject, Probeable {
    static let probeID = "mission"

    @Published private(set) var items: [MissionItem] = []
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published private(set) var vehiclePosition: (latitude: Double, longitude: Double)?

    func reload() {
        let controller = Bridge.group("plan.missionController")
        guard controller["kind"] as? String == "object" else {
            status = "No vehicle is connected."
            items = []
            return
        }

        let model = Bridge.group("plan.missionController.visualItems")
        items = ((model["elements"] as? [[String: Any]]) ?? []).map(MissionItem.init(json:))
        syncing = (Bridge.group("plan")["syncInProgress"] as? NSNumber)?.boolValue ?? false
        status = items.isEmpty ? "This plan has no items." : ""

        let coordinate = Bridge.group("vehicle")["coordinate"] as? [String: Any]
        if let latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue,
           let longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue {
            vehiclePosition = (latitude, longitude)
        } else {
            vehiclePosition = nil
        }
    }

    // Reading the mission back from the vehicle is the only way to be sure what it is
    // actually going to fly, as opposed to what was last edited here.
    func downloadFromVehicle() {
        Bridge.invoke("plan.loadFromVehicle")
        syncing = true
        reload()
    }

    func probeState() -> [String: Any] {
        ["count": items.count, "syncing": syncing, "status": status,
         "placed": items.filter(\.hasPosition).count,
         "vehiclePlaced": vehiclePosition != nil,
         "map": MissionMap.lastRender,
         "items": items.prefix(8).map {
             ["seq": $0.sequence, "command": $0.command,
              "position": $0.positionText, "altitude": $0.altitudeText]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
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
