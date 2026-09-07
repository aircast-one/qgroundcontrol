import Foundation

struct VehicleMessage: Identifiable, Equatable {
    enum Level: String {
        case error
        case warning
        case normal
    }

    let time: String
    let text: String
    let level: Level

    var id: String { "\(time)|\(text)" }

    static let errorToken = "<#E>"
    static let warningToken = "<#I>"

    static func level(of chunk: String) -> Level {
        if chunk.contains(errorToken) { return .error }
        if chunk.contains(warningToken) { return .warning }
        return .normal
    }

    static func parse(_ formatted: String) -> [VehicleMessage] {
        formatted
            .components(separatedBy: "</font>")
            .compactMap { chunk in
                guard let opened = chunk.range(of: "\">") else { return nil }
                let body = String(chunk[opened.upperBound...])
                    .replacingOccurrences(of: "<br/>", with: "")
                    .trimmingCharacters(in: .whitespaces)
                guard !body.isEmpty else { return nil }
                return split(body, level: level(of: chunk))
            }
    }

    static let severityWords: Set<String> = [
        "EMERGENCY", "ALERT", "Critical", "Error", "Warning", "Notice", "Info", "Debug",
    ]

    static func withoutSeverity(_ text: String) -> String {
        guard let colon = text.firstIndex(of: ":"),
              severityWords.contains(String(text[text.startIndex..<colon])) else { return text }
        return String(text[text.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
    }

    static func split(_ body: String, level: Level) -> VehicleMessage {
        guard body.hasPrefix("["), let close = body.firstIndex(of: "]") else {
            return VehicleMessage(time: "", text: withoutSeverity(body), level: level)
        }
        let stamp = body[body.index(after: body.startIndex)..<close]
            .trimmingCharacters(in: .whitespaces)
        let rest = String(body[body.index(after: close)...]).trimmingCharacters(in: .whitespaces)
        return VehicleMessage(time: String(stamp.prefix(8)), text: withoutSeverity(rest), level: level)
    }

    static func worst(_ messages: [VehicleMessage]) -> Level {
        if messages.contains(where: { $0.level == .error }) { return .error }
        if messages.contains(where: { $0.level == .warning }) { return .warning }
        return .normal
    }
}
