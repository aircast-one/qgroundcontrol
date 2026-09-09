import Foundation

struct LogEntry: Identifiable, Equatable {
    let index: Int
    let id: Int
    let sizeText: String
    let status: String
    let received: Bool
    let timeState: TimeState
    let time: String

    enum TimeState: String {
        case unreceived
        case unknown
        case known
        case unrecognised

        init(_ reported: String?) {
            self = TimeState(rawValue: reported ?? "") ?? .unrecognised
        }
    }

    // The core says which of QGC's three branches applies (LogDownloadPage.qml:83-91); only
    // the real time is rendered, and in the reader's locale, which is why that part is here.
    var timeText: String {
        switch timeState {
        case .unreceived, .unrecognised: return ""
        case .unknown: return "Date Unknown"
        case .known:
            guard let parsed = LogEntry.parser.date(from: time) else { return time }
            return LogEntry.display.string(from: parsed)
        }
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = (json["id"] as? NSNumber)?.intValue else { return nil }
        self.id = id
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        sizeText = (json["sizeText"] as? String) ?? ""
        status = (json["status"] as? String) ?? ""
        received = (json["received"] as? NSNumber)?.boolValue ?? false
        timeState = TimeState(json["timeState"] as? String)
        time = (json["time"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [LogEntry] {
        ((json as? [Any]) ?? []).compactMap(LogEntry.init)
    }

    private static let parser: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss.SSS"
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        return formatter
    }()

    private static let display: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()
}
