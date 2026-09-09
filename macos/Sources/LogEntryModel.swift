import Foundation

struct LogEntry: Identifiable, Equatable {
    let index: Int
    let id: Int
    let sizeText: String
    let status: String
    let received: Bool
    let selected: Bool
    let time: String

    // QGC's own three branches (LogDownloadPage.qml:83-91): an entry the vehicle has not
    // sent shows nothing at all, a clock that never got set shows that it did not, and
    // only a real time is formatted — in the reader's locale, which is why this stays here
    // rather than coming from the core as a fixed string.
    var timeText: String {
        guard received else { return "" }
        guard let parsed = LogEntry.parser.date(from: time) else { return time }
        guard Calendar(identifier: .gregorian).component(.year, from: parsed) >= 2010 else {
            return "Date Unknown"
        }
        return LogEntry.display.string(from: parsed)
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = (json["id"] as? NSNumber)?.intValue else { return nil }
        self.id = id
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        sizeText = (json["sizeText"] as? String) ?? ""
        status = (json["status"] as? String) ?? ""
        received = (json["received"] as? NSNumber)?.boolValue ?? false
        selected = (json["selected"] as? NSNumber)?.boolValue ?? false
        time = (json["time"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [LogEntry] {
        ((json as? [Any]) ?? []).compactMap(LogEntry.init)
    }

    // QGC enables Download on busy-ness alone (LogDownloadPage.qml:128). The core's
    // canDownload also wants a selected entry, which fits a single button over a selection
    // rather than this head's per-row buttons that already name the log they fetch.
    static func canDownload(requestingList: Bool, downloading: Bool) -> Bool {
        !requestingList && !downloading
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
