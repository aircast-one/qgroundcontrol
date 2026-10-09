import SwiftUI

private let MAVLINK_LOG_VIEW = "view.mavlinkLog"
private let MAVLINK_LOG_POLL_MS = 1000
private let TEXT_SAVE_DELAY_MS = 600

let WIND_SPEEDS: [(String, String)] = [("Not Set", "-1"), ("Calm", "0"), ("Breeze", "5"), ("Gale", "8"), ("Storm", "10")]
let FLIGHT_RATINGS: [(String, String)] = [
    ("Not Set", "notset"),
    ("Crashed (Pilot Error)", "crash_pilot"),
    ("Crashed (Software or Hardware Issue)", "crash_sw_hw"),
    ("Unsatisfactory", "unsatisfactory"),
    ("Good", "good"),
    ("Great", "great"),
]

struct LogFile: Equatable {
    let name: String
    let size: Int64
    let uploaded: Bool
    var writing: Bool = false
}

struct MavlinkLog: Equatable {
    let px4: Bool
    let running: Bool
    let canStart: Bool
    let persistence: Bool
    let settings: JSON
    let files: [LogFile]
    let uploading: Bool
    let uploadingFile: String
    let uploadProgress: Float
    let message: String
}

func mavlinkLog(_ view: JSON?) -> MavlinkLog? {
    guard let it = view else { return nil }
    return MavlinkLog(
        px4: it["vehiclePx4"].bool,
        running: it["logRunning"].bool,
        canStart: it["canStartLog"].bool,
        persistence: it["persistence"].bool(true),
        settings: it["settings"].object != nil ? it["settings"] : .object([:]),
        files: it["files"].objects.map { file in
            LogFile(name: file["name"].string, size: file["size"].int64 ?? 0, uploaded: file["uploaded"].bool, writing: file["writing"].bool)
        },
        uploading: it["uploading"].bool,
        uploadingFile: it["uploadingFile"].string,
        uploadProgress: min(max(Float(it["uploadProgress"].double(0)), 0), 1),
        message: it["message"].string
    )
}

func logSizeText(_ size: Int64) -> String {
    NumberFormatter.localizedString(from: NSNumber(value: size), number: .decimal)
}

func logListIdle(_ log: MavlinkLog) -> Bool { !log.uploading && !log.running }

func uploadEmail(_ settings: JSON, _ drafts: [String: String]) -> String {
    drafts["emailAddress"] ?? settings["emailAddress"].string
}

func unsavedTexts(_ settings: JSON, _ drafts: [String: String]) -> [(String, String)] {
    drafts.filter { field, typed in typed != settings[field].string }.sorted { $0.key < $1.key }.map { ($0.key, $0.value) }
}

struct LogConfirm {
    let title: String
    let question: String
    let run: () -> Void
}

let MAVLINK_LOGGING_TITLE = "MAVLink Logging"
let NO_PX4_VEHICLE = "Connect a PX4 vehicle to manage logs"

struct Px4LogTransferPage: View {
    @Environment(\.theme) private var theme
    @State private var read: MavlinkLog?
    @State private var polls = 0
    @State private var selected: Set<String> = []
    @State private var confirming: LogConfirm?
    @State private var refused: String?
    @State private var drafts: [String: String] = [:]

    var body: some View {
        VStack(spacing: 0) { content }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .task(id: polls) {
                while !Task.isCancelled {
                    read = await offMain { mavlinkLog(Qgc.get(MAVLINK_LOG_VIEW)) }
                    try? await Task.sleep(for: .milliseconds(MAVLINK_LOG_POLL_MS))
                }
            }
            .alert(confirming?.title ?? "", isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }), presenting: confirming) { confirm in
                Button("Ok") {
                    confirming = nil
                    confirm.run()
                }
                Button("Cancel", role: .cancel) { confirming = nil }
            } message: { confirm in
                Text(confirm.question)
            }
            .alertWhenPresenterFree($refused, title: { _ in MAVLINK_LOGGING_TITLE }) { _ in
                Button("Close", role: .cancel) { refused = nil }
            } message: { text in
                Text(text)
            }
    }

    @ViewBuilder private var content: some View {
        if let log = read {
            if !log.px4 {
                Text(NO_PX4_VEHICLE)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, 16)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                page(log)
            }
        }
    }

    private func act(_ calls: [(String, [Any?])]) {
        let refusalSlot = $refused
        let pollSlot = $polls
        offMainInOrder {
            let first = calls.compactMap { path, args in Qgc.refusalOf(path, arguments: args) }.first
            onMain {
                if let first { refusalSlot.wrappedValue = first }
                pollSlot.wrappedValue += 1
            }
        }
    }

    private func act(_ path: String, _ args: Any?...) { act([(path, args)]) }

    private func draft(_ field: String) -> (String) -> Void {
        { typed in drafts[field] = typed }
    }

    private func save(_ field: String) -> (String) -> Void {
        { typed in act("mavlinkLog.set", field, typed) }
    }

    private func unsavedCalls(_ log: MavlinkLog) -> [(String, [Any?])] {
        unsavedTexts(log.settings, drafts).map { field, typed in ("mavlinkLog.set", [field, typed] as [Any?]) }
    }

    private func saveItemsThen(_ log: MavlinkLog, _ path: String, _ args: Any?...) {
        act(unsavedCalls(log) + [(path, args)])
    }

    private func page(_ log: MavlinkLog) -> some View {
        let editable = log.persistence
        let idle = logListIdle(log)
        let uploadedSelected = log.files.contains { selected.contains($0.name) && $0.uploaded }
        return ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                SectionHeader(text: "MAVLink 2.0 Logging")
                FootNote(text: "PX4 Pro only")
                HStack(spacing: 8) {
                    Text("Logging").frame(maxWidth: .infinity, alignment: .leading)
                    Button("Start") { act("mavlinkLog.start") }
                        .buttonStyle(.bordered)
                        .disabled(!(!log.running && log.canStart && editable))
                    Button("Stop") { act("mavlinkLog.stop") }
                        .buttonStyle(.bordered)
                        .disabled(!(log.running && editable))
                }
                LogFlag(label: "Start logging automatically", checked: log.settings["enableAutoStart"].bool, enabled: editable) { act("mavlinkLog.set", "enableAutoStart", $0) }

                SectionHeader(text: "Log Upload")
                LogText(label: "Email Address", value: log.settings["emailAddress"].string, enabled: editable, onType: draft("emailAddress"), onSave: save("emailAddress"))
                LogText(label: "Default Description", value: log.settings["description"].string, enabled: editable, onType: draft("description"), onSave: save("description"))
                LogText(label: "Upload URL", value: log.settings["uploadURL"].string, enabled: editable, onType: draft("uploadURL"), onSave: save("uploadURL"))
                LogText(label: "Video URL", value: log.settings["videoURL"].string, enabled: editable, onType: draft("videoURL"), onSave: save("videoURL"))
                LogChoice(label: "Wind Speed", choices: WIND_SPEEDS, value: log.settings["windSpeed"].string, enabled: editable) { saveItemsThen(log, "mavlinkLog.set", "windSpeed", $0) }
                LogChoice(label: "Flight Rating", choices: FLIGHT_RATINGS, value: log.settings["rating"].string, enabled: editable) { saveItemsThen(log, "mavlinkLog.set", "rating", $0) }
                LogText(label: "Additional Feedback", value: log.settings["feedback"].string, enabled: editable, onType: draft("feedback"), onSave: save("feedback"))
                LogFlag(label: "Make logs public", checked: log.settings["publicLog"].bool, enabled: editable) { act("mavlinkLog.set", "publicLog", $0) }
                LogFlag(label: "Upload logs automatically", checked: log.settings["enableAutoUpload"].bool, enabled: editable) { saveItemsThen(log, "mavlinkLog.set", "enableAutoUpload", $0) }
                LogFlag(label: "Delete logs after upload", checked: log.settings["deleteAfterUpload"].bool, enabled: editable && log.settings["enableAutoUpload"].bool) {
                    act("mavlinkLog.set", "deleteAfterUpload", $0)
                }

                SectionHeader(text: "Saved Log Files")
                if log.files.isEmpty { Text("No log files").font(.bodySmall) }
                ForEach(log.files, id: \.name) { file in
                    fileRow(log, file)
                }
                HStack(spacing: 4) {
                    Button("Select All") { selected = Set(log.files.filter { !$0.writing }.map(\.name)) }.disabled(!idle)
                    Button("Select None") { selected = [] }.disabled(!idle)
                    Button("Delete…") {
                        let chosen = selected.sorted().map { $0 as Any? }
                        confirming = LogConfirm(title: "Delete Selected Log Files", question: "Delete the selected log files?") {
                            act([("mavlinkLog.delete", chosen)])
                            selected = []
                        }
                    }
                    .disabled(!(!selected.isEmpty && idle))
                    if log.uploading {
                        Button("Cancel Upload…") {
                            confirming = LogConfirm(title: "Cancel Upload", question: "Cancel the upload in progress?") { act("mavlinkLog.cancelUpload") }
                        }
                    } else {
                        Button("Upload…") {
                            act(unsavedCalls(log))
                            if uploadEmail(log.settings, drafts).isBlank {
                                refused = "Please enter an email address before uploading MAVLink log files."
                            } else {
                                let chosen = selected.sorted().map { $0 as Any? }
                                confirming = LogConfirm(title: "Upload Selected Log Files", question: "Upload the selected log files?") {
                                    act([("mavlinkLog.upload", chosen)])
                                    selected = []
                                }
                            }
                        }
                        .disabled(!(!selected.isEmpty && idle && !uploadedSelected))
                    }
                }
                .buttonStyle(.text)
                .font(.labelLarge)
                if !log.message.isBlank {
                    Text(log.message).font(.bodySmall).foregroundStyle(theme.colors.error)
                }
            }
            .padding(.horizontal, 16)
        }
    }

    private func fileRow(_ log: MavlinkLog, _ file: LogFile) -> some View {
        let uploadingThis = log.uploading && log.uploadingFile == file.name
        return HStack {
            Toggle("", isOn: Binding(
                get: { selected.contains(file.name) },
                set: { selected = $0 ? selected.union([file.name]) : selected.subtracting([file.name]) }
            ))
            .labelsHidden()
            .disabled(uploadingThis || file.writing)
            Text(file.name)
                .font(.bodySmall)
                .foregroundStyle(file.writing ? theme.aircast.warning : theme.colors.onSurface)
                .frame(maxWidth: .infinity, alignment: .leading)
            if uploadingThis && !file.uploaded {
                ProgressView(value: log.uploadProgress).frame(width: 96)
            } else {
                Text(file.uploaded ? "Uploaded" : logSizeText(file.size))
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
    }
}

private struct LogFlag: View {
    let label: String
    let checked: Bool
    let enabled: Bool
    let onChange: (Bool) -> Void

    var body: some View {
        Toggle(label, isOn: Binding(get: { checked }, set: onChange))
            .disabled(!enabled)
    }
}

private struct LogText: View {
    let label: String
    let value: String
    let enabled: Bool
    let onType: (String) -> Void
    let onSave: (String) -> Void
    @State private var typed: String?
    @Environment(\.theme) private var theme

    var body: some View {
        let shown = typed ?? value
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(label, text: Binding(get: { shown }, set: { typed = $0 }))
                .textFieldStyle(.roundedBorder)
        }
        .disabled(!enabled)
        .onChange(of: value) { typed = nil }
        .task(id: shown) {
            onType(shown)
            guard shown != value, (try? await Task.sleep(for: .milliseconds(TEXT_SAVE_DELAY_MS))) != nil else { return }
            onSave(shown)
        }
    }
}

private struct LogChoice: View {
    let label: String
    let choices: [(String, String)]
    let value: String
    let enabled: Bool
    let onPick: (String) -> Void

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(choices, id: \.1) { shown, raw in
                    Button(shown) { onPick(raw) }
                }
            } label: {
                Text(choices.first { $0.1 == value }?.0 ?? choices.first?.0 ?? "")
            }
            .buttonStyle(.bordered)
            .disabled(!enabled)
        }
    }
}
