import Foundation

struct VehicleComponentInfo: Identifiable, Equatable {
    let name: String
    let className: String
    let needsAttention: Bool

    // The name is QGC's and its constructor runs tr(), so it is translated in the five locales
    // that translate "Sensors" and is no basis for identity or for recognising a component. The
    // class is the same in every build. Falls back to the name only so a component the bridge
    // reports without a class still gets a distinct id rather than colliding on "".
    var id: String { className.isEmpty ? name : className }

    // PX4 and ArduPilot each have their own sensors component and both are the one this window
    // badges when a sensor is failing.
    static let sensorClasses = ["SensorsComponent", "APMSensorsComponent"]

    var isSensors: Bool { VehicleComponentInfo.sensorClasses.contains(className) }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        className = (json["className"] as? String) ?? ""
        needsAttention = (json["needsAttention"] as? NSNumber)?.boolValue ?? false
    }

    static func list(_ json: Any?) -> [VehicleComponentInfo] {
        ((json as? [Any]) ?? []).compactMap(VehicleComponentInfo.init)
    }
}

struct VehicleReadiness: Equatable {
    let connected: Bool
    let ready: Bool
    let headline: String
    let detail: String

    static let unknown = VehicleReadiness(connected: false, ready: false, headline: "", detail: "")

    var verdict: (text: String, good: Bool)? {
        guard connected else { return nil }
        return ready ? ("Ready", true) : ("Check", false)
    }

    init(connected: Bool, ready: Bool, headline: String, detail: String) {
        self.connected = connected
        self.ready = ready
        self.headline = headline
        self.detail = detail
    }

    init(_ json: [String: Any]) {
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        ready = (json["ready"] as? NSNumber)?.boolValue ?? false
        headline = (json["headline"] as? String) ?? ""
        detail = (json["detail"] as? String) ?? ""
    }
}
