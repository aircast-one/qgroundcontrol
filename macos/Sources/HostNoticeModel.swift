import Foundation

struct HostNotice: Identifiable, Equatable {
    enum Kind: Equatable {
        case message
        case vehicleError
        case navigation
        case unknown

        init(_ token: String?) {
            switch token {
            case "message": self = .message
            case "vehicleError": self = .vehicleError
            case "navigation": self = .navigation
            default: self = .unknown
            }
        }
    }

    let id: Int64
    let kind: Kind
    let title: String
    let text: String
    let at: Date?

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = (json["id"] as? NSNumber)?.int64Value else { return nil }
        self.id = id
        kind = Kind(json["kind"] as? String)
        title = (json["title"] as? String) ?? ""
        text = (json["text"] as? String) ?? ""
        at = (json["at"] as? NSNumber).map {
            Date(timeIntervalSince1970: $0.doubleValue / 1000)
        }
    }

    // A navigation notice carries its destination in the title, and QGCApplication has exactly
    // one: showVehicleConfig posts "setup" from AutoPilotPlugin.cc:72.
    var destination: String? { kind == .navigation ? title : nil }

    // showCriticalVehicleMessage is fed by StatusTextHandler::newErrorMessage, which fires on
    // the same STATUSTEXT that reaches vehicle.formattedMessages, so the Fly view's message
    // panel already draws it. An unknown kind is drawn: the core meant to say something and
    // this head not recognising the word is no reason to lose the sentence.
    var shows: Bool {
        switch kind {
        case .message, .unknown: return true
        case .vehicleError, .navigation: return false
        }
    }

    var line: String {
        guard !title.isEmpty, !text.isEmpty else { return text.isEmpty ? title : text }
        return "\(title): \(text)"
    }
}

struct HostNotices: Equatable {
    let all: [HostNotice]
    let dropped: Int
    let reported: Int

    static let none = HostNotices(all: [], dropped: 0, reported: 0)

    init(all: [HostNotice], dropped: Int, reported: Int) {
        self.all = all
        self.dropped = dropped
        self.reported = reported
    }

    init(_ json: [String: Any]) {
        let listed = (json["notices"] as? [Any]) ?? []
        all = listed.compactMap(HostNotice.init)
        dropped = (json["dropped"] as? NSNumber)?.intValue ?? 0
        reported = (json["count"] as? NSNumber)?.intValue ?? 0
    }

    // The producer counts its own queue, so a count this head cannot turn into notices is a
    // message it has lost rather than one that was never sent. Measured on the running app:
    // host.count answered 2 while host.notices answered [null, null], because a QVariantList
    // property is a leaf the bridge cannot walk and its maps serialise as null. compactMap
    // swallowed both and drew an empty panel, which is the silence this channel exists to end.
    var lost: Int { max(0, reported - all.count) }

    var lostNotice: String? {
        guard lost > 0 else { return nil }
        return lost == 1
            ? "1 notice could not be read."
            : "\(lost) notices could not be read."
    }

    var shown: [HostNotice] { all.filter(\.shows) }

    var newest: HostNotice? { shown.last }

    // The same call that posts this also posts the sentence explaining it, and that sentence is
    // already drawn, so the destination is offered as somewhere to go rather than as another
    // row saying the same thing. It is offered and not taken: raising a window over whatever
    // the operator is watching is not the same act as QML switching a view inside one window.
    var navigation: HostNotice? { all.last { $0.kind == .navigation } }

    static let setup = "setup"

    var offersSetup: Bool { navigation?.destination == HostNotices.setup }

    // A destination this head cannot reach is still a request that was made. Saying so beats
    // dropping it, and names the word so the next person knows what to wire.
    var unreachable: String? {
        guard let asked = navigation?.destination, asked != HostNotices.setup else { return nil }
        return "The app asked to open \(asked), which this window cannot reach."
    }

    // The queue keeps its oldest eight and drops from behind them, so what is missing is from
    // the middle of a burst. Saying nothing would let the count silently disagree with the list.
    var dropNotice: String? {
        guard dropped > 0 else { return nil }
        return dropped == 1
            ? "1 earlier notice was dropped."
            : "\(dropped) earlier notices were dropped."
    }
}
