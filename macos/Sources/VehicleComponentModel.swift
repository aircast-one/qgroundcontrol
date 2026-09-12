import Foundation

struct VehicleComponentInfo: Identifiable, Equatable {
    let name: String
    let className: String
    let needsAttention: Bool
    let openable: Bool
    let blockedReason: String?
    let known: String?

    var id: String { className.isEmpty ? name : className }

    static let sensorClasses = ["SensorsComponent", "APMSensorsComponent"]

    var isSensors: Bool { VehicleComponentInfo.sensorClasses.contains(className) }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        className = (json["className"] as? String) ?? ""
        needsAttention = (json["needsAttention"] as? NSNumber)?.boolValue ?? false
        openable = (json["openable"] as? NSNumber)?.boolValue ?? true
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        known = (json["known"] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    static let blockedWithoutReason = "Disabled"

    var blockedSentence: String? {
        guard !openable else { return nil }
        guard let blockedReason else { return VehicleComponentInfo.blockedWithoutReason }
        return "Disabled while the vehicle is \(blockedReason)"
    }

    static func identity(ofPage page: String) -> String {
        let words = page.split(separator: " ").map(String.init)
        guard let first = words.first else { return "" }
        return ([first.lowercased()] + words.dropFirst()).joined()
    }

    static func blockedSentence(for page: String, in listed: [VehicleComponentInfo]) -> String? {
        let wanted = identity(ofPage: page)
        let matched = listed.first { $0.known == wanted }
            ?? listed.first { $0.known == nil && $0.name == page }
        return matched?.blockedSentence
    }

    static func list(_ json: Any?) -> [VehicleComponentInfo] {
        ((json as? [Any]) ?? []).compactMap(VehicleComponentInfo.init)
    }
}

struct VehicleReadiness: Equatable {
    let connected: Bool
    let ready: Bool?
    let headline: String
    let detail: String

    static let unknown = VehicleReadiness(connected: false, ready: nil, headline: "", detail: "")

    var verdict: (text: String, good: Bool)? {
        guard let ready else { return nil }
        return ready ? ("Ready", true) : ("Check", false)
    }

    init(connected: Bool, ready: Bool?, headline: String, detail: String) {
        self.connected = connected
        self.ready = ready
        self.headline = headline
        self.detail = detail
    }

    init(_ json: [String: Any]) {
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        ready = (json["ready"] as? NSNumber)?.boolValue
        headline = (json["headline"] as? String) ?? ""
        detail = (json["detail"] as? String) ?? ""
    }
}
