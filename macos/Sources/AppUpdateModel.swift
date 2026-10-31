import Foundation

struct AppUpdate: Equatable {
    enum Phase: String {
        case idle, checking, upToDate, available, downloading, ready, failed
    }

    enum Action: Hashable {
        case check, download, update, cancel, openInstaller, snooze

        var path: String? {
            switch self {
            case .check: return "appUpdate.check"
            case .download, .update: return "appUpdate.download"
            case .cancel: return "appUpdate.cancel"
            case .snooze: return "appUpdate.snooze"
            case .openInstaller: return nil
            }
        }
    }

    static let application = "Aircast QGC"
    static let disconnectFirst = "Disconnect from the vehicle first"
    static let unanswered = "The updater did not answer."
    static let notOpened = "macOS could not open the installer."
    static let tryAgain = "Try again"
    static let autoCheckTitle = "Check for updates automatically"
    static let autoCheckNote = "Looks for a new release shortly after start and every six hours."

    let current: String
    let channel: String
    let phase: Phase
    let latest: String
    let notes: String
    let releaseUrl: String
    let assetName: String
    let sizeBytes: Int64
    let progress: Double
    let file: String
    let error: String
    let checkedAt: Date?
    let autoCheck: Bool
    let snoozed: Bool
    let canInstall: Bool

    static let none = AppUpdate([:])

    init(_ view: [String: Any]) {
        let text = { (key: String) in (view[key] as? String) ?? "" }
        current = text("current")
        channel = text("channel")
        phase = Phase(rawValue: text("phase")) ?? .idle
        latest = text("latest")
        notes = text("notes")
        releaseUrl = text("releaseUrl")
        assetName = text("assetName")
        sizeBytes = (view["sizeBytes"] as? NSNumber)?.int64Value ?? 0
        progress = (view["progress"] as? NSNumber)?.doubleValue ?? 0
        file = text("file")
        error = text("error")
        checkedAt = ISO8601DateFormatter().date(from: text("checkedAt"))
        autoCheck = (view["autoCheck"] as? NSNumber)?.boolValue ?? true
        snoozed = !text("snoozed").isEmpty
        canInstall = (view["canInstall"] as? NSNumber)?.boolValue ?? false
    }

    var offered: Bool { [.available, .downloading, .ready].contains(phase) }

    var busy: Bool { phase == .checking || phase == .downloading }

    var bannerShown: Bool { offered && canInstall && !snoozed }

    var percent: Int { Int((progress * 100).rounded()) }

    var title: String {
        switch phase {
        case .downloading: return "Downloading \(Self.application) \(latest)… \(percent)%"
        case .ready: return "\(Self.application) \(latest) is ready to install"
        default: return "\(Self.application) \(latest) is available"
        }
    }

    var channelName: String { channel == "beta" ? "Beta" : "Stable" }

    var size: String {
        sizeBytes > 0 ? String(format: "%.0f MB", Double(sizeBytes) / 1_048_576) : ""
    }

    var checkedText: String {
        checkedAt.map { "Last checked \($0.formatted(date: .abbreviated, time: .shortened))" }
            ?? "Not checked yet"
    }

    var caption: String {
        ["\(channelName) channel", phase == .upToDate ? "Up to date" : "", checkedText]
            .filter { !$0.isEmpty }
            .joined(separator: " \u{00B7} ")
    }

    var offerAction: Action? {
        switch phase {
        case .available: return .download
        case .downloading: return .cancel
        case .ready: return .openInstaller
        default: return nil
        }
    }

    var offerDetail: String { phase == .ready ? installRefusal ?? "" : "" }

    var bannerActions: [Action] {
        switch phase {
        case .downloading: return [.cancel]
        case .ready: return [.openInstaller, .snooze]
        default: return [.update, .snooze]
        }
    }

    var installRefusal: String? { canInstall ? nil : Self.disconnectFirst }

    var retry: Action { assetName.isEmpty ? .check : .download }

    func label(_ action: Action) -> String {
        switch action {
        case .check: return phase == .checking ? "Checking…" : "Check now"
        case .download: return size.isEmpty ? "Download" : "Download (\(size))"
        case .update: return "Update"
        case .cancel: return "Cancel"
        case .openInstaller: return "Open installer"
        case .snooze: return "Later"
        }
    }

    func enabled(_ action: Action) -> Bool {
        switch action {
        case .check: return !busy
        case .openInstaller: return canInstall && !file.isEmpty
        default: return true
        }
    }

    var plainNotes: String {
        notes.replacingOccurrences(of: "\r", with: "")
            .components(separatedBy: "\n")
            .map { line in
                line.trimmingCharacters(in: .whitespaces)
                    .replacingOccurrences(of: #"\[([^\]]+)\]\([^)]*\)"#, with: "$1",
                                          options: .regularExpression)
                    .replacingOccurrences(of: #"^#+\s*|\*\*|`"#, with: "", options: .regularExpression)
                    .replacingOccurrences(of: #"^[-*]\s+"#, with: "• ", options: .regularExpression)
            }
            .filter { !$0.isEmpty }
            .joined(separator: "\n")
    }

    static func refusal(_ answer: [String: Any]) -> String? {
        guard (answer["ok"] as? NSNumber)?.boolValue != true else { return nil }
        return (answer["reason"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? unanswered
    }
}
