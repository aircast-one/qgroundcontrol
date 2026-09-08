import Foundation

final class VehicleComponentsStore: ObservableObject, Probeable {
    static let probeID = "vehicleComponents"

    @Published private(set) var components: [VehicleComponentInfo] = []
    @Published private(set) var connected = false
    @Published private(set) var readiness = VehicleReadiness.unknown

    func reload() {
        let view = Bridge.group("view.setup")
        let live = (view["connected"] as? NSNumber)?.boolValue ?? false
        if live != connected { connected = live }
        let listed = VehicleComponentInfo.list(view["components"])
        if listed != components { components = listed }
        let read = VehicleReadiness(view)
        if read != readiness { readiness = read }
    }

    var outstanding: [VehicleComponentInfo] { components.filter(\.needsAttention) }

    func probeState() -> [String: Any] {
        ["connected": connected, "count": components.count,
         "outstanding": outstanding.map(\.name),
         "ready": readiness.ready, "headline": readiness.headline, "detail": readiness.detail,
         "components": components.map {
             ["name": $0.name, "needsAttention": $0.needsAttention]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "reload" else { return ["ok": false, "error": "unknown action \(action)"] }
        reload()
        return ["ok": true, "state": probeState()]
    }
}
