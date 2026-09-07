import Foundation

struct VehicleComponentInfo: Identifiable {
    let name: String
    let setupComplete: Bool
    let requiresSetup: Bool

    var id: String { name }

    var needsAttention: Bool { requiresSetup && !setupComplete }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        setupComplete = (object["setupComplete"] as? NSNumber)?.boolValue ?? true
        requiresSetup = (object["requiresSetup"] as? NSNumber)?.boolValue ?? false
    }

    static func from(_ elements: [Any]) -> [VehicleComponentInfo] {
        elements.compactMap(VehicleComponentInfo.init(json:))
    }

    static func incomplete(_ components: [VehicleComponentInfo]) -> [VehicleComponentInfo] {
        components.filter(\.needsAttention)
    }

}

struct VehicleReadiness {
    let ready: Bool
    let headline: String
    let detail: String

    init(connected: Bool, components: [VehicleComponentInfo], sensorFaults: [String]) {
        let outstanding = VehicleComponentInfo.incomplete(components)

        guard connected else {
            ready = false
            headline = "No vehicle connected"
            detail = "Connect a vehicle to check what it needs."
            return
        }

        ready = outstanding.isEmpty && sensorFaults.isEmpty && !components.isEmpty

        if !outstanding.isEmpty {
            headline = outstanding.count == 1
                ? "1 component needs setup"
                : "\(outstanding.count) components need setup"
        } else if !sensorFaults.isEmpty {
            headline = "\(sensorFaults.count) sensor\(sensorFaults.count == 1 ? "" : "s") reporting a fault"
        } else if components.isEmpty {
            headline = "This vehicle reports no setup components"
        } else {
            headline = "Ready to fly"
        }

        if !sensorFaults.isEmpty {
            detail = sensorFaults.joined(separator: ", ")
        } else if !outstanding.isEmpty {
            detail = outstanding.map(\.name).joined(separator: ", ")
        } else if components.isEmpty {
            detail = "Nothing to check."
        } else {
            detail = "Setup complete and all enabled sensors are healthy."
        }
    }
}
