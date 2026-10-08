import SwiftUI

let MESSAGES = "view.messages"

enum MessageSeverity: String, CaseIterable {
    case Error, Warning, Normal
}

struct VehicleMessage: Equatable, Hashable {
    let index: Int
    let time: String
    let severity: String
    let level: MessageSeverity
    let text: String
}

private let PREARM_PREFIX = "PreArm:"

func chipBlocker(_ blocker: String) -> String {
    let reason = blocker.trimmed.removingPrefix(PREARM_PREFIX).trimmed
    let sentence = reason.range(of: ". ").map { String(reason[..<$0.lowerBound]) } ?? reason
    return sentence.trimmingTrailing(".")
}

func bannerMessages(_ unread: [VehicleMessage]) -> [VehicleMessage] {
    unread.filter { ($0.level == .Error || $0.level == .Warning) && !$0.text.trimmingLeading(" ", "\t", "\n").hasPrefix(PREARM_PREFIX) }
}

func bannerText(_ messages: [VehicleMessage]) -> String? {
    guard !messages.isEmpty else { return nil }
    let worst = messages.last { $0.level == .Error } ?? messages.last { $0.level == .Warning }
    let count = messageCountText(messages.count)
    return worst.flatMap { $0.text.isBlank ? nil : $0.text } ?? count
}

func severitySummary(_ messages: [VehicleMessage]) -> String {
    let counted = { (n: Int, one: String) in "\(n) \(one)\(n == 1 ? "" : "s")" }
    let errors = messages.filter { $0.level == .Error }.count
    let warnings = messages.filter { $0.level == .Warning }.count
    let rest = messages.count - errors - warnings
    return [
        errors > 0 ? counted(errors, "error") : nil,
        warnings > 0 ? counted(warnings, "warning") : nil,
        rest > 0 ? counted(rest, "message") : nil,
    ].compactMap { $0 }.joined(separator: " \u{00b7} ").ifBlank(messageCountText(0))
}

func messageCountText(_ count: Int) -> String { "\(count) message\(count == 1 ? "" : "s") from the vehicle" }

func unreadMessages(_ messages: [VehicleMessage], _ unread: Int) -> [VehicleMessage] {
    Array(messages.suffix(max(unread, 0)))
}

func unreadCount(_ view: JSON?) -> Int { view?["unread"].int(0) ?? 0 }

func levelOf(_ name: String) -> MessageSeverity {
    switch name {
    case "error": .Error
    case "warning": .Warning
    default: .Normal
    }
}

let OLDEST_FIRST = "oldestFirst"

func vehicleMessages(_ view: JSON?) -> [VehicleMessage] {
    guard let view, let items = view["items"].arrayOrNil else { return [] }
    let read = items.enumerated().compactMap { at, item -> VehicleMessage? in
        let text = item["text"].string
        guard item.object != nil, !text.isBlank else { return nil }
        return VehicleMessage(
            index: item["index"].int(at),
            time: messageTime(item["time"].string),
            severity: item["severity"].string,
            level: levelOf(item["level"].string),
            text: text
        )
    }
    return view["order"].string == OLDEST_FIRST ? read : read.reversed()
}

let WARNINGS = "view.warnings"

func armingBlocker(_ view: JSON?) -> String? {
    guard let view, !view["armingBlocker"].isNull else { return nil }
    let blocker = view["armingBlocker"].string
    return blocker.isBlank ? nil : blocker
}

struct ArmingCheck: Equatable, Hashable {
    let message: String
    let description: String
    let severity: String
}

func armingChecks(_ view: JSON?) -> [ArmingCheck]? {
    guard let view, !view["armingChecks"].isNull, let listed = view["armingChecks"].arrayOrNil else { return nil }
    return listed.filter { $0.object != nil }.map { problem in
        ArmingCheck(message: problem["message"].string, description: problem["description"].string, severity: problem["severity"].string)
    }.filter { !$0.message.isBlank }
}

private let CLOCK_WITH_MILLIS = #/^(\d{1,2}:\d{2}:\d{2})\.\d+$/#

func messageTime(_ served: String) -> String {
    (try? CLOCK_WITH_MILLIS.wholeMatch(in: served.trimmed)).map { String($0.1) } ?? served
}

private let ALERT_CORNER: CGFloat = 12

struct VehicleMessageBanner: View {
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @QgcPath(MESSAGES) private var messagesJson
    @State private var showing = false

    var body: some View {
        let shown = bannerMessages(unreadMessages(vehicleMessages(messagesJson), unreadCount(messagesJson)))
        ZStack {
            if !shown.isEmpty { banner(shown) }
            if showing { VehicleMessagesSheet { showing = false } }
        }
    }

    private func banner(_ shown: [VehicleMessage]) -> some View {
        let urgent = shown.contains { $0.level == .Error }
        let shape = urgent ? AnyShape(RoundedRectangle(cornerRadius: ALERT_CORNER)) : AnyShape(Capsule())
        let content = urgent ? osdTint(theme.colors.onErrorContainer, theme.colors.error, flyOsd) : osdTint(theme.colors.onSurface, theme.aircast.warning, flyOsd)
        return HStack(spacing: 10) {
            Image(urgent ? .error : .warning)
                .font(.system(size: 20))
                .foregroundStyle(urgent ? content : theme.aircast.warning)
                .frame(width: 24, height: 24)
            Text(bannerText(shown) ?? messageCountText(shown.count))
                .font(.labelLarge)
                .lineLimit(2)
                .truncationMode(.tail)
            Button { offMain { VehicleCommands.resetAllMessages() } } label: {
                Image(.close).font(.system(size: 16)).frame(width: 32, height: 32)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Dismiss messages")
        }
        .foregroundStyle(content)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .frame(minHeight: 40)
        .background(osdBackdrop(urgent ? theme.colors.errorContainer : theme.aircast.warningContainer, flyOsd), in: shape)
        .contentShape(shape)
        .onTapGesture {
            showing = true
            offMain { VehicleCommands.resetAllMessages() }
        }
        .accessibilityAddTraits(.isButton)
    }
}

struct VehicleMessagesSheet: View {
    let onDismiss: () -> Void
    @QgcPath(WARNINGS) private var warnings
    @QgcPath(MESSAGES) private var messagesJson

    var body: some View {
        VehicleMessageLog(
            messages: vehicleMessages(messagesJson),
            checks: armingChecks(warnings) ?? [],
            blocker: armingBlocker(warnings),
            onDismiss: onDismiss
        )
    }
}

private struct VehicleMessageLog: View {
    let messages: [VehicleMessage]
    let checks: [ArmingCheck]
    let blocker: String?
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme
    @State private var editing: String?

    var body: some View {
        let blocking = checks.isEmpty ? [blocker.map { ArmingCheck(message: $0, description: "", severity: "error") }].compactMap { $0 } : checks
        let lines = Array(messages.reversed())
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(alignment: .leading, spacing: 0) {
                Text(blocking.isEmpty ? "Messages" : "Why it will not arm").font(.headlineSmall)
                Text(severitySummary(lines)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                if lines.isEmpty && blocking.isEmpty {
                    Text("The vehicle has not said anything yet.").padding(.vertical, 16)
                } else {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            ForEach(Array(blocking.enumerated()), id: \.offset) { _, check in
                                MessageLine(level: check.severity == "error" ? .Error : .Warning, time: "") {
                                    Text(check.message).font(.bodyMedium)
                                    if !check.description.isBlank {
                                        LinkedText(html: check.description, style: .bodySmall, color: theme.colors.onSurfaceVariant, onParameter: { editing = $0 })
                                    }
                                }
                            }
                            ForEach(Array(lines.enumerated()), id: \.offset) { _, message in
                                MessageLine(level: message.level, time: message.time) {
                                    Text(message.text).font(.bodyMedium)
                                }
                            }
                        }
                    }
                    .frame(maxHeight: 420)
                    .padding(.top, 12)
                }
                HStack {
                    Spacer()
                    Button("Clear") {
                        offMain { VehicleCommands.clearMessages() }
                        onDismiss()
                    }
                    Button("Close", action: onDismiss)
                }
                .buttonStyle(.borderless)
                .padding(.top, 8)
                if let name = editing {
                    ParameterEditDialog(name: name, title: EDIT_PARAMETER_TITLE, onDismiss: { editing = nil })
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 24)
        }
    }
}

struct VehicleMessagesPage: View {
    @QgcPath(MESSAGES) private var messagesJson

    var body: some View {
        let lines = Array(vehicleMessages(messagesJson).reversed())
        Group {
            if lines.isEmpty {
                EmptyState(icon: .description, title: "No messages", text: "The vehicle has not said anything yet.")
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(Array(lines.enumerated()), id: \.offset) { _, message in
                            MessageLine(level: message.level, time: message.time) {
                                Text(message.text).font(.bodyMedium)
                            }
                        }
                        HStack {
                            Spacer()
                            Button("Clear") { offMain { VehicleCommands.clearMessages() } }.buttonStyle(.borderless)
                        }
                    }
                    .padding(.horizontal, 16)
                }
            }
        }
        .onAppear { offMain { VehicleCommands.resetAllMessages() } }
    }
}

struct MessageLine<Content: View>: View {
    let level: MessageSeverity
    let time: String
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            Image(icon)
                .font(.system(size: 20))
                .foregroundStyle(tint)
                .frame(width: 24, height: 24)
            VStack(alignment: .leading, spacing: 2) {
                if !time.isBlank {
                    Text(time).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                content()
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 10)
    }

    private var icon: Icon {
        switch level {
        case .Error: .error
        case .Warning: .warning
        case .Normal: .checkCircle
        }
    }

    private var tint: Color {
        switch level {
        case .Error: theme.colors.error
        case .Warning: theme.aircast.warning
        case .Normal: theme.aircast.success
        }
    }
}

enum SnackbarDuration: Equatable {
    case Short, Long, Indefinite

    var millis: Int? {
        switch self {
        case .Short: 4000
        case .Long: 10000
        case .Indefinite: nil
        }
    }
}

enum SnackbarResult: Equatable {
    case Dismissed, ActionPerformed
}

@MainActor
final class SnackbarData: Identifiable {
    let id = UUID()
    let message: String
    let actionLabel: String?
    let withDismissAction: Bool
    let duration: SnackbarDuration
    private var finish: ((SnackbarResult) -> Void)?

    init(message: String, actionLabel: String?, withDismissAction: Bool, duration: SnackbarDuration, finish: @escaping (SnackbarResult) -> Void) {
        self.message = message
        self.actionLabel = actionLabel
        self.withDismissAction = withDismissAction
        self.duration = duration
        self.finish = finish
    }

    func dismiss() { resolve(.Dismissed) }

    func performAction() { resolve(.ActionPerformed) }

    private func resolve(_ result: SnackbarResult) {
        let done = finish
        finish = nil
        done?(result)
    }
}

@MainActor
@Observable
final class SnackbarHostState {
    private(set) var currentSnackbarData: SnackbarData?
    @ObservationIgnored private var waiters: [CheckedContinuation<Void, Never>] = []
    @ObservationIgnored private var busy = false

    @discardableResult
    func showSnackbar(_ message: String, actionLabel: String? = nil, withDismissAction: Bool = false, duration: SnackbarDuration? = nil) async -> SnackbarResult {
        await acquire()
        let shownFor = duration ?? (actionLabel == nil ? .Short : .Indefinite)
        let result = await withCheckedContinuation { (done: CheckedContinuation<SnackbarResult, Never>) in
            let data = SnackbarData(message: message, actionLabel: actionLabel, withDismissAction: withDismissAction, duration: shownFor) { done.resume(returning: $0) }
            currentSnackbarData = data
            if let millis = shownFor.millis {
                Task { @MainActor in
                    try? await Task.sleep(for: .milliseconds(millis))
                    data.dismiss()
                }
            }
        }
        currentSnackbarData = nil
        release()
        return result
    }

    private func acquire() async {
        guard busy else {
            busy = true
            return
        }
        await withCheckedContinuation { waiters.append($0) }
    }

    private func release() {
        guard !waiters.isEmpty else {
            busy = false
            return
        }
        waiters.removeFirst().resume()
    }
}

struct SnackbarHost<Content: View>: View {
    let hostState: SnackbarHostState
    @ViewBuilder let snackbar: (SnackbarData) -> Content

    var body: some View {
        ZStack {
            if let data = hostState.currentSnackbarData {
                snackbar(data)
                    .id(data.id)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.2), value: hostState.currentSnackbarData?.id)
    }
}

private struct Snackbar: View {
    let data: SnackbarData
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s2) {
            Text(data.message)
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.inverseOnSurface)
                .frame(maxWidth: .infinity, alignment: .leading)
            if let action = data.actionLabel {
                Button(action) { data.performAction() }
                    .font(.labelLarge)
                    .foregroundStyle(theme.dark ? Theme.lightTheme.colors.primary : Theme.darkTheme.colors.primary)
                    .buttonStyle(.borderless)
            }
            if data.withDismissAction {
                Button { data.dismiss() } label: { Image(.close) }
                    .foregroundStyle(theme.colors.inverseOnSurface)
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Dismiss")
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(theme.colors.inverseSurface, in: RoundedRectangle(cornerRadius: Corner.extraSmall))
        .frame(maxWidth: 600)
        .padding(12)
    }
}

struct AppSnackbar: View {
    let data: SnackbarData
    @Environment(\.theme) private var theme

    var body: some View {
        if data.duration != .Indefinite {
            Snackbar(data: data)
        } else {
            HStack(spacing: 12) {
                Image(.error).font(.system(size: 20)).frame(width: 24, height: 24)
                Text(data.message)
                    .font(.labelLarge)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Button { data.dismiss() } label: { Image(.close).frame(width: 40, height: 40) }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Dismiss")
            }
            .foregroundStyle(theme.colors.onErrorContainer)
            .padding(.leading, 16)
            .padding(.trailing, 4)
            .padding(.vertical, 6)
            .background(theme.colors.errorContainer, in: RoundedRectangle(cornerRadius: ALERT_CORNER))
            .frame(maxWidth: 600)
            .padding(12)
        }
    }
}
