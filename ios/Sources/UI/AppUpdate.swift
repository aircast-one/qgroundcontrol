#if os(macOS)
import SwiftUI

// App updates from the core's updater (appUpdate.*, view.appUpdate). The iOS app updates through the App Store, so
// this exists on the native macOS app only; its install step (open the .dmg and quit) is in AppInstall.swift.

let APP_UPDATE_VIEW = "view.appUpdate"
let DISCONNECT_FIRST = "Disconnect from the vehicle first"
let INSTALL_PRIMER = "Aircast quits now and opens the installer. Drag Aircast onto Applications and choose Replace."
let RELEASE_TAG_URL = "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v"
private let UPDATE_SUBJECT = "The updater"
private let NOTES_MAX_LINES = 12
private let UPDATE_BANNER_MAX_WIDTH: CGFloat = 560
private let UPDATE_BANNER_CORNER: CGFloat = 12

enum UpdatePhase: String {
    case Idle = "idle"
    case Checking = "checking"
    case UpToDate = "upToDate"
    case Available = "available"
    case Downloading = "downloading"
    case Ready = "ready"
    case Failed = "failed"

    static func of(_ token: String) -> UpdatePhase { UpdatePhase(rawValue: token) ?? .Idle }
}

struct AppUpdateReading: Equatable {
    let current: String
    let channel: String
    let phase: UpdatePhase
    let latest: String
    let notes: String
    let releaseUrl: String
    let assetName: String
    let assetUrl: String
    let sizeBytes: Int64
    let progress: Double
    let file: String
    let error: String
    let checkedAt: String
    let autoCheck: Bool
    let skipped: String
    let later: Bool
    let updatedFrom: String
    let canInstall: Bool
}

func appUpdateReading(_ view: JSON?) -> AppUpdateReading? {
    guard let view, view.object != nil else { return nil }
    return AppUpdateReading(
        current: view["current"].string,
        channel: view["channel"].string,
        phase: UpdatePhase.of(view["phase"].string),
        latest: view["latest"].string,
        notes: view["notes"].string,
        releaseUrl: view["releaseUrl"].string,
        assetName: view["assetName"].string,
        assetUrl: view["assetUrl"].string,
        sizeBytes: view["sizeBytes"].int64 ?? 0,
        progress: view["progress"].double(0),
        file: view["file"].string,
        error: view["error"].string,
        checkedAt: view["checkedAt"].string,
        autoCheck: view["autoCheck"].bool(true),
        skipped: view["skipped"].string,
        later: view["later"].bool(false),
        updatedFrom: view["updatedFrom"].string,
        canInstall: view["canInstall"].bool
    )
}

func updateStatus(_ reading: AppUpdateReading, failure: InstallFailure? = nil) -> String {
    switch reading.phase {
    case .Checking: "Checking for updates…"
    case .UpToDate: "Aircast is up to date"
    case .Available: "Aircast \(reading.latest) is available"
    case .Downloading: "Downloading Aircast \(reading.latest)… \(Int((reading.progress * 100).rounded()))%"
    case .Ready: failure?.text ?? "Aircast \(reading.latest) is ready to install"
    case .Failed: reading.error.ifBlank("The update check failed.")
    case .Idle: ""
    }
}

func skippedThis(_ reading: AppUpdateReading) -> Bool { !reading.skipped.isEmpty && reading.skipped == reading.latest }

func bannerShown(_ reading: AppUpdateReading) -> Bool {
    reading.canInstall && !reading.later && !skippedThis(reading)
        && [UpdatePhase.Available, .Downloading, .Ready].contains(reading.phase)
}

func updatedShown(_ reading: AppUpdateReading) -> Bool { !reading.updatedFrom.isEmpty && reading.canInstall }

func releaseTagUrl(_ version: String) -> String { RELEASE_TAG_URL + version }

func installRefusal(_ reading: AppUpdateReading) -> String? { reading.canInstall ? nil : DISCONNECT_FIRST }

func installFailureFor(_ outcome: InstallOutcome?, _ reading: AppUpdateReading) -> InstallFailure? {
    guard let outcome, outcome.file == reading.file, reading.phase == .Ready, reading.canInstall else { return nil }
    return outcome.failure
}

func sizedLabel(_ label: String, _ size: String?) -> String { size.map { "\(label) (\($0))" } ?? label }

func meteredQuestion(_ size: String?) -> String {
    size.map { "This update is \($0) and you're on a metered connection. Download now?" }
        ?? "You're on a metered connection. Download the update now?"
}

enum ReadyAction { case Install, OpenReleasePage, None }

// A .dmg Finder would not open can be tried again; one that has gone can only be fetched again, from the release page.
func readyAction(_ failure: InstallFailure?) -> ReadyAction {
    guard let failure else { return .Install }
    switch failure {
    case .Missing: return .OpenReleasePage
    case .NotOpened: return .Install
    }
}

func installLabel(_ failure: InstallFailure?) -> String { failure == nil ? "Install" : "Try again" }

private let MARKDOWN_LINK = #"\[([^\]]+)\]\([^)]*\)"#
private let MARKDOWN_MARKS = #"(?m)^[ \t]{0,3}(#{1,6}[ \t]+|[-*+][ \t]+|>[ \t]*)|[*_`]+"#

func plainNotes(_ markdown: String) -> String {
    let stripped = markdown
        .replacingOccurrences(of: "\r\n", with: "\n")
        .replacingOccurrences(of: MARKDOWN_LINK, with: "$1", options: .regularExpression)
        .replacingOccurrences(of: MARKDOWN_MARKS, with: "", options: .regularExpression)
    var lines = stripped.components(separatedBy: "\n").map(trimmedEnd)
    while let first = lines.first, first.isBlank { lines.removeFirst() }
    while let last = lines.last, last.isBlank { lines.removeLast() }
    return lines.joined(separator: "\n").replacingOccurrences(of: #"\n{3,}"#, with: "\n\n", options: .regularExpression)
}

private func trimmedEnd(_ line: String) -> String {
    var trimmed = Substring(line)
    while let last = trimmed.last, last.isWhitespace { trimmed = trimmed.dropLast() }
    return String(trimmed)
}

func checkedAtMillis(_ checkedAt: String) -> Int64? {
    ISO8601DateFormatter().date(from: checkedAt).map { Int64(($0.timeIntervalSince1970 * 1000).rounded()) }
}

func updateSizeText(_ bytes: Int64) -> String? {
    bytes > 0 ? ByteCountFormatter.string(fromByteCount: bytes, countStyle: .file) : nil
}

private func lastCheckedText(_ checkedAt: String) -> String? {
    checkedAtMillis(checkedAt).map { millis in
        let when = Date(timeIntervalSince1970: Double(millis) / 1000)
        return "Last checked \(RelativeDateTimeFormatter().localizedString(for: when, relativeTo: Date()))"
    }
}

// The questions an update asks before it acts: a download over a metered connection, and the install, which quits the
// app (Android's equivalents are the metered dialog and the install-permission primer).
private enum UpdatePrompt: Equatable {
    case Metered
    case Install

    var title: String { self == .Install ? "Install the update" : "Metered connection" }
    var confirm: String { self == .Install ? "Open installer and quit" : "Download" }
    var decline: String { self == .Install ? "Cancel" : "Not now" }
}

private struct UpdatePrompts: ViewModifier {
    let reading: AppUpdateReading
    @Binding var prompt: UpdatePrompt?
    let download: () -> Void

    func body(content: Content) -> some View {
        content
            .alert(prompt?.title ?? "", isPresented: Binding(get: { prompt != nil }, set: { if !$0 { prompt = nil } }), presenting: prompt) { asked in
                Button(asked.confirm) {
                    prompt = nil
                    switch asked {
                    case .Metered: download()
                    case .Install: MacInstaller.shared.start(reading.file)
                    }
                }
                Button(asked.decline, role: .cancel) { prompt = nil }
            } message: { asked in
                Text(asked == .Install ? INSTALL_PRIMER : meteredQuestion(updateSizeText(reading.sizeBytes)))
            }
            .onAppear { UpdateNetwork.warm() }
    }
}

// Keeps an install failure only while it still describes the update on screen, as Android's rememberInstallFailure does.
private struct ForgetsStaleInstall: ViewModifier {
    let reading: AppUpdateReading

    func body(content: Content) -> some View {
        content.onChange(of: "\(reading.phase.rawValue)|\(reading.file)|\(reading.canInstall)", initial: true) {
            MacInstaller.shared.forget { installFailureFor($0, reading) != nil }
        }
    }
}

private func askToDownload(_ prompt: Binding<UpdatePrompt?>, _ download: () -> Void) {
    if UpdateNetwork.metered { prompt.wrappedValue = .Metered } else { download() }
}

struct UpdatesSection: View {
    @QgcPath(APP_UPDATE_VIEW) private var view
    @State private var refusal: String?
    @State private var prompt: UpdatePrompt?
    @Environment(\.theme) private var theme
    @Environment(\.openURL) private var openURL
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if let reading = appUpdateReading(view) {
            if nativeList {
                nativeSection(reading, installFailureFor(MacInstaller.shared.outcome, reading))
            } else {
                section(reading, installFailureFor(MacInstaller.shared.outcome, reading))
                    .modifier(UpdatePrompts(reading: reading, prompt: $prompt, download: { act("appUpdate.download") }))
                    .modifier(ForgetsStaleInstall(reading: reading))
            }
        }
    }

    // Inside a NativeForm: an Updates section — the version row with Check now (which carries the prompts), the
    // automatic check switch, the state with its detail, and each action the state offers as a row of its own.
    @ViewBuilder private func nativeSection(_ reading: AppUpdateReading, _ failure: InstallFailure?) -> some View {
        let status = refusal ?? updateStatus(reading, failure: failure)
        let alarming = refusal != nil || failure != nil || reading.phase == .Failed
        let offersRelease = reading.phase == .Ready && readyAction(failure) == .OpenReleasePage
        let detail = updateDetail(reading)
        Section {
            LabeledContent {
                Button("Check now") { act("appUpdate.check") }
                    .disabled(reading.phase == .Checking || reading.phase == .Downloading)
            } label: {
                Text("Aircast version")
                Text("\(reading.current) · \(reading.channel) channel")
            }
            .modifier(UpdatePrompts(reading: reading, prompt: $prompt, download: { act("appUpdate.download") }))
            .modifier(ForgetsStaleInstall(reading: reading))
            Toggle("Check for updates automatically", isOn: Binding(get: { reading.autoCheck }, set: { on in
                offMain { _ = Qgc.invoke("appUpdate.setAutoCheck", on) }
            }))
            if !status.isBlank {
                VStack(alignment: .leading, spacing: 2) {
                    Text(status).foregroundStyle(alarming ? theme.colors.error : Color.primary)
                    if !detail.isBlank {
                        Text(detail).font(.subheadline).foregroundStyle(Color.secondary)
                    }
                }
            }
            if reading.phase == .Downloading {
                ProgressView(value: min(max(reading.progress, 0), 1))
            }
            if (reading.phase == .Available || reading.phase == .Ready) && !reading.notes.isBlank {
                Text(plainNotes(reading.notes))
                    .font(.footnote)
                    .foregroundStyle(Color.secondary)
                    .lineLimit(NOTES_MAX_LINES)
                    .truncationMode(.tail)
            }
            switch reading.phase {
            case .Available:
                Button(sizedLabel("Download", updateSizeText(reading.sizeBytes))) {
                    askToDownload($prompt) { act("appUpdate.download") }
                }
            case .Downloading:
                Button("Cancel download") { act("appUpdate.cancel") }
            case .Ready:
                switch readyAction(failure) {
                case .OpenReleasePage:
                    Button("Open release page") { open(reading.releaseUrl.ifBlank(releaseTagUrl(reading.latest))) }
                case .Install:
                    Button(installLabel(failure)) { prompt = .Install }.disabled(!reading.canInstall)
                case .None:
                    EmptyView()
                }
            case .Failed:
                if !reading.assetUrl.isBlank {
                    Button("Download in browser") { open(reading.assetUrl) }
                }
                Button("Try again") { act("appUpdate.check") }
            default:
                EmptyView()
            }
            if !reading.releaseUrl.isBlank && reading.phase != .UpToDate && !offersRelease {
                Button("Release notes") { open(reading.releaseUrl) }
            }
            if (reading.phase == .Available || reading.phase == .Ready) && !skippedThis(reading) {
                Button("Skip this version") { act("appUpdate.skip") }
            }
        } header: {
            SectionHeader(text: "Updates")
        }
    }

    private func act(_ path: String) {
        refusal = nil
        Task { refusal = await offMain { Qgc.refusalOf(path, subject: UPDATE_SUBJECT) } }
    }

    private func open(_ link: String) {
        if let url = URL(string: link) { openURL(url) }
    }

    private func section(_ reading: AppUpdateReading, _ failure: InstallFailure?) -> some View {
        let status = refusal ?? updateStatus(reading, failure: failure)
        let alarming = refusal != nil || failure != nil || reading.phase == .Failed
        let offersRelease = reading.phase == .Ready && readyAction(failure) == .OpenReleasePage
        return VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: "Updates")
            UpdateRow(headline: "Aircast version", supporting: "\(reading.current) · \(reading.channel) channel") {
                Button("Check now") { act("appUpdate.check") }
                    .buttonStyle(.text)
                    .disabled(reading.phase == .Checking || reading.phase == .Downloading)
            }
            UpdateRow(headline: "Check for updates automatically") {
                Toggle("Check for updates automatically", isOn: Binding(get: { reading.autoCheck }, set: { on in
                    offMain { _ = Qgc.invoke("appUpdate.setAutoCheck", on) }
                }))
                .labelsHidden()
                .toggleStyle(.switch)
            }
            if !status.isBlank {
                UpdateRow(headline: status, supporting: updateDetail(reading), headlineColor: alarming ? theme.colors.error : theme.colors.onSurface) {
                    EmptyView()
                }
            }
            if reading.phase == .Downloading {
                ProgressView(value: min(max(reading.progress, 0), 1))
                    .padding(.horizontal, Space.s4)
            }
            if (reading.phase == .Available || reading.phase == .Ready) && !reading.notes.isBlank {
                Text(plainNotes(reading.notes))
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .lineLimit(NOTES_MAX_LINES)
                    .truncationMode(.tail)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, Space.s2)
            }
            HStack(spacing: Space.s2) {
                Spacer(minLength: 0)
                if !reading.releaseUrl.isBlank && reading.phase != .UpToDate && !offersRelease {
                    Button("Release notes") { open(reading.releaseUrl) }.buttonStyle(.text)
                }
                if (reading.phase == .Available || reading.phase == .Ready) && !skippedThis(reading) {
                    Button("Skip this version") { act("appUpdate.skip") }.buttonStyle(.text)
                }
                switch reading.phase {
                case .Available:
                    Button(sizedLabel("Download", updateSizeText(reading.sizeBytes))) {
                        askToDownload($prompt) { act("appUpdate.download") }
                    }
                    .buttonStyle(.filled)
                case .Downloading:
                    Button("Cancel") { act("appUpdate.cancel") }.buttonStyle(.text)
                case .Ready:
                    UpdateReadyButton(reading: reading, failure: failure, install: { prompt = .Install })
                case .Failed:
                    if !reading.assetUrl.isBlank {
                        Button("Download in browser") { open(reading.assetUrl) }.buttonStyle(.text)
                    }
                    Button("Try again") { act("appUpdate.check") }.buttonStyle(.text)
                default:
                    EmptyView()
                }
            }
            .padding(.horizontal, Space.s2)
        }
    }

    private func updateDetail(_ reading: AppUpdateReading) -> String {
        let named = reading.phase == .Ready && !reading.assetName.isBlank
            ? updateSizeText(reading.sizeBytes).map { "\(reading.assetName) · \($0)" } ?? reading.assetName
            : nil
        let lines: [String?] = [
            reading.phase == .Ready ? installRefusal(reading) : nil,
            named,
            lastCheckedText(reading.checkedAt),
        ]
        return lines.compactMap { $0 }.joined(separator: "\n")
    }
}

private struct UpdateRow<Trailing: View>: View {
    let headline: String
    var supporting: String = ""
    var headlineColor: Color? = nil
    @ViewBuilder let trailing: () -> Trailing
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s4) {
            VStack(alignment: .leading, spacing: 2) {
                Text(headline).font(.bodyLarge).foregroundStyle(headlineColor ?? theme.colors.onSurface)
                if !supporting.isBlank {
                    Text(supporting).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            trailing()
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }
}

private struct UpdateReadyButton: View {
    let reading: AppUpdateReading
    let failure: InstallFailure?
    let install: () -> Void
    @Environment(\.openURL) private var openURL

    var body: some View {
        switch readyAction(failure) {
        case .OpenReleasePage:
            Button("Open release page") {
                if let url = URL(string: reading.releaseUrl.ifBlank(releaseTagUrl(reading.latest))) { openURL(url) }
            }
            .buttonStyle(.filled)
        case .Install:
            Button(installLabel(failure), action: install)
                .buttonStyle(.filled)
                .disabled(!reading.canInstall)
        case .None:
            EmptyView()
        }
    }
}

// The Fly view's update banners: "Updated to …" once after an upgrade, and the offer while an update is available,
// downloading or ready. Both stay hidden while a vehicle is connected.
struct UpdateBanner: View {
    @QgcPath(APP_UPDATE_VIEW) private var view

    var body: some View {
        if let reading = appUpdateReading(view) {
            if updatedShown(reading) { UpdatedBanner(reading: reading) }
            if bannerShown(reading) {
                UpdateOfferBanner(reading: reading, failure: installFailureFor(MacInstaller.shared.outcome, reading))
                    .modifier(ForgetsStaleInstall(reading: reading))
            }
        }
    }
}

private struct UpdatedBanner: View {
    let reading: AppUpdateReading
    @Environment(\.openURL) private var openURL

    var body: some View {
        UpdateBannerCard(icon: .checkCircle, text: "Updated to Aircast \(reading.current)") {
            Button("Dismiss") { offMain { _ = Qgc.invoke("appUpdate.dismissUpdated") } }
                .buttonStyle(.text)
            Button("What's new") {
                if let url = URL(string: releaseTagUrl(reading.current)) { openURL(url) }
            }
            .buttonStyle(.filled)
        }
    }
}

private struct UpdateOfferBanner: View {
    let reading: AppUpdateReading
    let failure: InstallFailure?
    @State private var prompt: UpdatePrompt?

    var body: some View {
        UpdateBannerCard(
            icon: .download,
            text: updateStatus(reading, failure: failure),
            progress: reading.phase == .Downloading ? reading.progress : nil
        ) {
            switch reading.phase {
            case .Downloading:
                Button("Cancel") { act("appUpdate.cancel") }.buttonStyle(.text)
            case .Ready:
                Button("Later") { act("appUpdate.later") }.buttonStyle(.text)
                UpdateReadyButton(reading: reading, failure: failure, install: { prompt = .Install })
            default:
                Button("Later") { act("appUpdate.later") }.buttonStyle(.text)
                Button(sizedLabel("Update", updateSizeText(reading.sizeBytes))) {
                    askToDownload($prompt) { act("appUpdate.download") }
                }
                .buttonStyle(.filled)
            }
        }
        .modifier(UpdatePrompts(reading: reading, prompt: $prompt, download: { act("appUpdate.download") }))
    }

    private func act(_ path: String) {
        offMain { _ = Qgc.invoke(path) }
    }
}

private struct UpdateBannerCard<Actions: View>: View {
    let icon: Icon
    let text: String
    var progress: Double? = nil
    @ViewBuilder let actions: () -> Actions
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            HStack(spacing: Space.s3) {
                Image(icon)
                    .resizable()
                    .scaledToFit()
                    .frame(width: 24, height: 24)
                    .accessibilityHidden(true)
                Text(text).font(.titleSmall)
            }
            if let progress {
                ProgressView(value: min(max(progress, 0), 1))
                    .padding(.leading, 36)
            }
            HStack(spacing: Space.s2) {
                Spacer(minLength: 0)
                actions()
            }
        }
        .padding(EdgeInsets(top: Space.s3, leading: Space.s3, bottom: Space.s1, trailing: Space.s2))
        .frame(maxWidth: .infinity, alignment: .leading)
        .foregroundStyle(theme.colors.onSecondaryContainer)
        .background(theme.colors.secondaryContainer, in: RoundedRectangle(cornerRadius: UPDATE_BANNER_CORNER))
        .frame(maxWidth: UPDATE_BANNER_MAX_WIDTH)
    }
}
#endif
