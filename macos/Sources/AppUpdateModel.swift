import Foundation

struct AppUpdate: Equatable {
    enum Phase: String {
        case idle, checking, upToDate, available, downloading, ready, failed
    }

    enum Action: Hashable {
        case check, download, update, cancel, openInstaller, later, skip, dismissUpdated

        var path: String? {
            switch self {
            case .check: return "appUpdate.check"
            case .download, .update: return "appUpdate.download"
            case .cancel: return "appUpdate.cancel"
            case .later: return "appUpdate.later"
            case .skip: return "appUpdate.skip"
            case .dismissUpdated: return "appUpdate.dismissUpdated"
            case .openInstaller: return nil
            }
        }

        var downloads: Bool { self == .download || self == .update }
    }

    enum Place: Equatable {
        case fly, settings
    }

    struct Prompt: Equatable {
        let action: Action
        let place: Place
        let message: String

        var title: String { action == .openInstaller ? "Install the update" : "Metered connection" }

        var confirm: String { action == .openInstaller ? "Open installer and quit" : "Download" }

        var decline: String { action == .openInstaller ? "Cancel" : "Not now" }
    }

    static let application = "Aircast QGC"
    static let releasesUrl = "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v"
    static let disconnectFirst = "Disconnect from the vehicle first"
    static let unanswered = "The updater did not answer."
    static let notOpened = "macOS could not open the installer."
    static let tryAgain = "Try again"
    static let whatsNew = "What's new"
    static let dismiss = "Dismiss"
    static let autoCheckTitle = "Check for updates automatically"
    static let autoCheckNote = "Looks for a new release shortly after start and every six hours."
    static let skippedNote = "Skipped. Fly no longer offers this version."
    static let replaceInstructions =
        "Drag Aircast onto Applications and choose Replace. Aircast quits now so the old copy can be replaced."

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
    let skipped: String
    let later: Bool
    let updatedFrom: String
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
        skipped = text("skipped")
        later = (view["later"] as? NSNumber)?.boolValue ?? false
        updatedFrom = text("updatedFrom")
        canInstall = (view["canInstall"] as? NSNumber)?.boolValue ?? false
    }

    var offered: Bool { [.available, .downloading, .ready].contains(phase) }

    var busy: Bool { phase == .checking || phase == .downloading }

    var isSkipped: Bool { !skipped.isEmpty && skipped == latest }

    var bannerShown: Bool { offered && canInstall && !later && !isSkipped }

    var updatedShown: Bool { !updatedFrom.isEmpty }

    var updatedTitle: String { "Updated to \(Self.application) \(current)" }

    var whatsNewUrl: String { Self.releasesUrl + current }

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

    var offerActions: [Action] {
        let skip: [Action] = isSkipped ? [] : [.skip]
        switch phase {
        case .available: return [.download] + skip
        case .downloading: return [.cancel]
        case .ready: return [.openInstaller] + skip
        default: return []
        }
    }

    var offerDetail: String {
        [phase == .ready ? installRefusal ?? "" : "", isSkipped ? Self.skippedNote : ""]
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }

    var bannerActions: [Action] {
        switch phase {
        case .downloading: return [.cancel]
        case .ready: return [.openInstaller, .later]
        default: return [.update, .later]
        }
    }

    var installRefusal: String? { canInstall ? nil : Self.disconnectFirst }

    var retry: Action { assetName.isEmpty ? .check : .download }

    func label(_ action: Action) -> String {
        let sized = { (word: String) in size.isEmpty ? word : "\(word) (\(size))" }
        switch action {
        case .check: return phase == .checking ? "Checking…" : "Check now"
        case .download: return sized("Download")
        case .update: return sized("Update")
        case .cancel: return "Cancel"
        case .openInstaller: return "Open installer"
        case .later: return "Later"
        case .skip: return "Skip this version"
        case .dismissUpdated: return Self.dismiss
        }
    }

    func enabled(_ action: Action) -> Bool {
        switch action {
        case .check: return !busy
        case .openInstaller: return canInstall && !file.isEmpty
        default: return true
        }
    }

    func meteredQuestion(expensive: Bool, constrained: Bool) -> String? {
        guard expensive || constrained else { return nil }
        return size.isEmpty
            ? "This connection is metered. Download the update now?"
            : "This update is \(size) and this connection is metered. Download now?"
    }

    func prompt(for action: Action, from place: Place, expensive: Bool, constrained: Bool) -> Prompt? {
        switch action {
        case .openInstaller:
            return Prompt(action: action, place: place, message: Self.replaceInstructions)
        case .download, .update:
            return meteredQuestion(expensive: expensive, constrained: constrained)
                .map { Prompt(action: action, place: place, message: $0) }
        default:
            return nil
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
