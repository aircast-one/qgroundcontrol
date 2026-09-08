import Foundation

struct VehicleComponentInfo: Identifiable, Equatable {
    let name: String
    let needsAttention: Bool

    var id: String { name }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        needsAttention = (json["needsAttention"] as? NSNumber)?.boolValue ?? false
    }

    static func list(_ json: Any?) -> [VehicleComponentInfo] {
        ((json as? [Any]) ?? []).compactMap(VehicleComponentInfo.init)
    }
}

struct VehicleReadiness: Equatable {
    let ready: Bool
    let headline: String
    let detail: String

    static let unknown = VehicleReadiness(ready: false, headline: "", detail: "")

    init(ready: Bool, headline: String, detail: String) {
        self.ready = ready
        self.headline = headline
        self.detail = detail
    }

    init(_ json: [String: Any]) {
        ready = (json["ready"] as? NSNumber)?.boolValue ?? false
        headline = (json["headline"] as? String) ?? ""
        detail = (json["detail"] as? String) ?? ""
    }
}
