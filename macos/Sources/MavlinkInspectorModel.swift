import Foundation

struct MavlinkMessage: Identifiable, Equatable {
    let index: Int
    let id: Int
    let name: String
    let compId: Int
    let count: Int
    let rateHz: Double
    let targetRateHz: Int
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
        targetRateHz = (object["targetRateHz"] as? NSNumber)?.intValue ?? MessageRate.useDefault
        selected = (object["selected"] as? NSNumber)?.boolValue ?? false
    }

    static func from(_ elements: [Any]) -> [MavlinkMessage] {
        elements.enumerated().compactMap { MavlinkMessage(json: $0.element, index: $0.offset) }
    }
}

enum MessageRate {
    static let disabled = -1
    static let useDefault = 0

    static let choices = [disabled, useDefault, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 25, 50, 100]

    static func title(_ rate: Int) -> String {
        switch rate {
        case disabled: return "Off"
        case useDefault: return "Default"
        default: return "\(rate) Hz"
        }
    }

    static func offered(_ rate: Int) -> Bool { choices.contains(rate) }

    static func shown(_ rate: Int) -> Int { offered(rate) ? rate : useDefault }
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
