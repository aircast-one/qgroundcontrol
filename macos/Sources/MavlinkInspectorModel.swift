import Foundation

struct MavlinkMessage: Identifiable, Equatable {
    let index: Int
    let id: Int
    let name: String
    let compId: Int
    let count: Int
    let rateHz: Double
    let selected: Bool

    var rateText: String {
        guard rateHz > 0 else { return "—" }
        return rateHz < 0.05 ? "<0.1 Hz" : String(format: "%.1f Hz", rateHz)
    }

    var countText: String { "\(count)" }

    init?(json: Any?, index: Int) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.index = index
        self.name = name
        id = (object["id"] as? NSNumber)?.intValue ?? 0
        compId = (object["compId"] as? NSNumber)?.intValue ?? 0
        count = (object["count"] as? NSNumber)?.intValue ?? 0
        rateHz = (object["actualRateHz"] as? NSNumber)?.doubleValue ?? 0
        selected = (object["selected"] as? NSNumber)?.boolValue ?? false
    }

    static func from(_ elements: [Any]) -> [MavlinkMessage] {
        elements.enumerated().compactMap { MavlinkMessage(json: $0.element, index: $0.offset) }
    }
}

struct MavlinkField: Identifiable, Equatable {
    let name: String
    let type: String
    let value: String

    var id: String { name }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        type = (object["type"] as? String) ?? ""
        value = (object["value"] as? String) ?? ""
    }

    static func from(_ elements: [Any]) -> [MavlinkField] {
        elements.compactMap(MavlinkField.init(json:))
    }
}
