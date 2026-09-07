import Foundation
import QGCMapTileC

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
        case "tile":
            // MapKit never calls loadTile while it cannot render, so the cache path is
            // exercised directly against a tile known to be in QGC's database.
            return fetchTile(x: Int(args["x"] ?? "") ?? 0,
                             y: Int(args["y"] ?? "") ?? 0,
                             z: Int(args["z"] ?? "") ?? 0,
                             type: args["type"] ?? CachedTileOverlay.currentMapType())
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
