import Foundation

final class VehicleComponentsStore: ObservableObject, Probeable {
    static let probeID = "vehicleComponents"

    @Published private(set) var components: [VehicleComponentInfo] = []
    @Published private(set) var connected = false

    func reload() {
        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        let plugin = Bridge.group("vehicle.autopilotPlugin.vehicleComponents")
        components = VehicleComponentInfo.from((plugin["value"] as? [Any]) ?? [])
    }

    var outstanding: [VehicleComponentInfo] { VehicleComponentInfo.incomplete(components) }

    func readiness(sensorFaults: [String]) -> VehicleReadiness {
        VehicleReadiness(connected: connected, components: components, sensorFaults: sensorFaults)
    }

    func probeState() -> [String: Any] {
        ["connected": connected, "count": components.count,
         "outstanding": outstanding.map(\.name),
         "components": components.map {
             ["name": $0.name, "complete": $0.setupComplete, "requires": $0.requiresSetup]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "reload" else { return ["ok": false, "error": "unknown action \(action)"] }
        reload()
        return ["ok": true, "state": probeState()]
    }
}
