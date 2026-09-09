import Foundation

struct VehicleMessage: Identifiable, Equatable {
    enum Level: String {
        case error
        case warning
        case normal
    }

    let time: String
    let component: Int?
    let level: Level
    let text: String

    // The list only ever grows at the front, so a key taken from the message itself keeps a
    // row's identity when one arrives; the core's index would shift every row down by one.
    var id: String { "\(time)|\(component.map(String.init) ?? "")|\(text)" }

    var stamp: String { String(time.prefix(8)) }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let text = json["text"] as? String, !text.isEmpty else { return nil }
        self.text = text
        time = (json["time"] as? String) ?? ""
        component = (json["component"] as? NSNumber)?.intValue
        level = Level(rawValue: (json["level"] as? String) ?? "") ?? .normal
    }

    static func list(_ items: Any?) -> [VehicleMessage] {
        ((items as? [Any]) ?? []).compactMap(VehicleMessage.init)
    }

    static func worst(_ messages: [VehicleMessage]) -> Level {
        if messages.contains(where: { $0.level == .error }) { return .error }
        if messages.contains(where: { $0.level == .warning }) { return .warning }
        return .normal
    }
}
