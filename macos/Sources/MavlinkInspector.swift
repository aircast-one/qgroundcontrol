import Foundation

final class MavlinkInspectorStore: ObservableObject, Probeable {
    static let probeID = "mavlinkInspector"

    @Published private(set) var messages: [MavlinkMessage] = []
    @Published private(set) var fields: [MavlinkField] = []
    @Published private(set) var systemId = 0
    @Published private(set) var listening = false

    private var poll: Timer?

    var selected: MavlinkMessage? { messages.first(where: \.selected) }

    func start() {
        refresh()
        poll?.invalidate()
        poll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stop() {
        poll?.invalidate()
        poll = nil
    }

    func refresh() {
        let systems = (Bridge.group("mavlinkInspector.systems")["elements"] as? [[String: Any]]) ?? []
        guard let system = systems.first else {
            if listening { listening = false }
            if !messages.isEmpty { messages = [] }
            if !fields.isEmpty { fields = [] }
            return
        }

        if !listening { listening = true }
        let id = (system["id"] as? NSNumber)?.intValue ?? 0
        if id != systemId { systemId = id }

        let listed = MavlinkMessage.from(
            (Bridge.group("mavlinkInspector.systems.0.messages")["elements"] as? [Any]) ?? [])
        if listed != messages { messages = listed }

        guard let current = listed.first(where: \.selected) else {
            if !fields.isEmpty { fields = [] }
            return
        }
        let read = MavlinkField.from(
            (Bridge.group("mavlinkInspector.systems.0.messages.\(current.index).fields")["elements"] as? [Any]) ?? [])
        if read != fields { fields = read }
    }

    func select(_ message: MavlinkMessage) {
        _ = Bridge.set("mavlinkInspector.systems.0.selected", message.index)
        refresh()
    }

    func probeState() -> [String: Any] {
        ["systemId": systemId, "listening": listening, "count": messages.count,
         "selected": selected?.name ?? "",
         "messages": messages.prefix(6).map {
             ["name": $0.name, "id": $0.id, "rate": $0.rateText,
              "count": $0.count, "selected": $0.selected]
         },
         "fields": fields.prefix(8).map { ["name": $0.name, "type": $0.type, "value": $0.value] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": refresh()
        case "select":
            guard let message = messages.first(where: { $0.name == args["message"] ?? "" }) else {
                return ["ok": false, "error": "no message named \(args["message"] ?? "")"]
            }
            select(message)
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
