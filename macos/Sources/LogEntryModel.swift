import Foundation

struct LogEntry: Identifiable, Equatable {
    let id: Int
    let sizeBytes: Int
    let sizeText: String
    let status: String
    let received: Bool
    let time: String

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let id = (object["id"] as? NSNumber)?.intValue else { return nil }
        self.id = id
        sizeBytes = (object["size"] as? NSNumber)?.intValue ?? 0
        sizeText = LogEntry.humanSize(sizeBytes)
        status = (object["status"] as? String) ?? ""
        received = (object["received"] as? NSNumber)?.boolValue ?? false
        time = LogEntry.humanTime((object["time"] as? String) ?? "")
    }

    static func from(_ elements: [Any]) -> [LogEntry] {
        elements.compactMap(LogEntry.init(json:))
    }

    // QGC's own sizeStr is locale-formatted with a comma decimal separator, which reads
    // as a thousands separator here.
    static func humanSize(_ bytes: Int) -> String {
        let units = ["bytes", "KB", "MB", "GB"]
        let step = bytes <= 0 ? 0 : min(Int(log(Double(bytes)) / log(1024)), units.count - 1)
        guard step > 0 else { return "\(bytes) bytes" }
        let value = Double(bytes) / pow(1024, Double(step))
        return String(format: "%.1f %@", value, units[step])
    }

    static func humanTime(_ raw: String) -> String {
        guard let date = LogEntry.parser.date(from: raw) else { return raw }
        return LogEntry.display.string(from: date)
    }

    private static let parser: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss.SSS"
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter
    }()

    private static let display: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()
}
