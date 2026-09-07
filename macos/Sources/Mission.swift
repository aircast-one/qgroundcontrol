import Foundation
import MapKit
import QGCMapTileC

final class MissionStore: ObservableObject, Probeable {
    static let probeID = "mission"

    @Published private(set) var items: [MissionItem] = []
    @Published private(set) var status = ""
    @Published private(set) var syncing = false
    @Published private(set) var vehiclePosition: (latitude: Double, longitude: Double)?
    @Published private(set) var dirty = false
    @Published private(set) var connected = false
    @Published var addingWaypoint = false
    @Published private(set) var canUndo = false
    @Published private(set) var canRedo = false

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
        canUndo = (plan["canUndo"] as? NSNumber)?.boolValue ?? false
        canRedo = (plan["canRedo"] as? NSNumber)?.boolValue ?? false
        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        status = items.isEmpty ? "This plan has no items." : ""

        let coordinate = Bridge.group("vehicle")["coordinate"] as? [String: Any]
        if let latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue,
           let longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue {
            vehiclePosition = (latitude, longitude)
        } else {
            vehiclePosition = nil
        }
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

    // Snapshots are taken by a timer in the controller rather than per edit, so the
    // stacks only catch up a beat after a change.
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

    // Assigning a @Published value republishes even when it has not changed, and this
    // runs every second: the plan view re-rendered on every tick, which tore down and
    // rebuilt every annotation and overlay on the map before any of them could draw.
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
        Bridge.invoke("plan.missionController.insertSimpleMissionItem",
                      [["latitude": latitude, "longitude": longitude], items.count, true])
        addingWaypoint = false
        reload()
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
         "tiles": ["exact": CachedTileOverlay.served,
                   "fromParent": CachedTileOverlay.fromParent,
                   "fromChildren": CachedTileOverlay.fromChildren,
                   "missed": CachedTileOverlay.missed],
         "placed": items.filter(\.hasPosition).count,
         "vehiclePlaced": vehiclePosition != nil,
         "map": MissionMap.lastRender["plan"] ?? [:],
         "selected": items.first(where: \.isCurrent)?.sequence ?? -1,
         "addingWaypoint": addingWaypoint,
         "canUndo": canUndo, "canRedo": canRedo,
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
            // MapKit never calls loadTile while it cannot render, so the cache path is
            // exercised directly against a tile known to be in QGC's database.
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
        case "undo":
            undo()
        case "redo":
            redo()
        case "editing":
            args["on"] == "0" ? stopEditing() : startEditing()
        case "arm":
            addingWaypoint = args["on"] != "0"
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
