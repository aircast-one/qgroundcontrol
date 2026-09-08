import Foundation

struct VehicleWarning: Identifiable, Equatable {
    let id: String
    let text: String
    let detail: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String, !id.isEmpty,
              let text = json["text"] as? String, !text.isEmpty else { return nil }
        self.id = id
        self.text = text
        detail = (json["detail"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [VehicleWarning] {
        ((json as? [Any]) ?? []).compactMap(VehicleWarning.init)
    }
}
