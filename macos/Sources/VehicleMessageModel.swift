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

    // A key taken from the message itself, so a row keeps its identity when another arrives at
    // either end of the list; the core's index shifts under every row as soon as one does.
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

struct VehicleMessages: Equatable {
    enum Order: String {
        case oldestFirst
        case newestFirst
    }

    let order: Order
    let all: [VehicleMessage]

    static let empty = VehicleMessages(order: .oldestFirst, all: [])

    init(order: Order, all: [VehicleMessage]) {
        self.order = order
        self.all = all
    }

    init(_ json: [String: Any]) {
        order = Order(rawValue: (json["order"] as? String) ?? "") ?? .oldestFirst
        all = VehicleMessage.list(json["items"])
    }

    var isEmpty: Bool { all.isEmpty }

    // Newest at the top, which is where it was when the core's list ran the other way.
    func newest(_ limit: Int) -> [VehicleMessage] {
        order == .newestFirst ? Array(all.prefix(limit)) : Array(all.suffix(limit).reversed())
    }
}
