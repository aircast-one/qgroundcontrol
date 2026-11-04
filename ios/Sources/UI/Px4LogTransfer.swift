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
                EmptyState(icon: .description, title: MAVLINK_LOGGING_TITLE, text: NO_PX4_VEHICLE)
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

    // The logging controls, the upload details and the saved files as sections of a native form; Delete, the
    // destructive action, in a section of its own with its confirmation.
    private func page(_ log: MavlinkLog) -> some View {
        let editable = log.persistence
        let idle = logListIdle(log)
        let uploadedSelected = log.files.contains { selected.contains($0.name) && $0.uploaded }
        return NativeForm {
            Section {
                LabeledContent("Logging") {
                    HStack(spacing: Space.s3) {
                        Button("Start") { act("mavlinkLog.start") }
                            .disabled(!(!log.running && log.canStart && editable))
                        Button("Stop") { act("mavlinkLog.stop") }
                            .disabled(!(log.running && editable))
                    }
                }
                LogFlag(label: "Start logging automatically", checked: log.settings["enableAutoStart"].bool, enabled: editable) { act("mavlinkLog.set", "enableAutoStart", $0) }
            } header: {
                SectionHeader(text: "MAVLink 2.0 Logging")
            } footer: {
                Text("PX4 Pro only")
            }

            Section {
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
            } header: {
                SectionHeader(text: "Log Upload")
            }

            Section {
                if log.files.isEmpty {
                    Text("No log files").foregroundStyle(Color.secondary)
                }
                ForEach(log.files, id: \.name) { file in
                    fileRow(log, file)
                }
            } header: {
                HStack {
                    SectionHeader(text: "Saved Log Files")
                    Spacer(minLength: 0)
                    if !log.files.isEmpty {
                        let all = Set(log.files.filter { !$0.writing }.map(\.name))
                        Button(selected.isEmpty || selected != all ? "Select All" : "Select None") {
                            selected = selected.isEmpty || selected != all ? all : []
                        }
                        .disabled(!idle)
                    }
                }
            } footer: {
                if !log.message.isBlank {
                    Text(log.message).foregroundStyle(theme.colors.error)
                }
            }

            Section {
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

            Section {
                Button("Delete…", role: .destructive) {
                    let chosen = selected.sorted().map { $0 as Any? }
                    confirming = LogConfirm(title: "Delete Selected Log Files", question: "Delete the selected log files?") {
                        act([("mavlinkLog.delete", chosen)])
                        selected = []
                    }
                }
                .disabled(!(!selected.isEmpty && idle))
            }
        }
    }

    private func fileRow(_ log: MavlinkLog, _ file: LogFile) -> some View {
        let uploadingThis = log.uploading && log.uploadingFile == file.name
        let checked = selected.contains(file.name)
        return Button {
            selected = checked ? selected.subtracting([file.name]) : selected.union([file.name])
        } label: {
            HStack(spacing: Space.s3) {
                Image(systemName: checked ? "checkmark.circle.fill" : "circle")
                    .font(.title3)
                    .foregroundStyle(checked ? theme.colors.primary : Color.secondary)
                    .accessibilityHidden(true)
                Text(file.name)
                    .foregroundStyle(file.writing ? theme.aircast.warning : Color.primary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if uploadingThis && !file.uploaded {
                    ProgressView(value: log.uploadProgress).frame(width: 96)
                } else {
                    Text(file.uploaded ? "Uploaded" : logSizeText(file.size)).foregroundStyle(Color.secondary)
                }
            }
            .contentShape(Rectangle())
        }
        .listRowButton()
        .disabled(uploadingThis || file.writing)
        .accessibilityAddTraits(checked ? .isSelected : [])
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
        FieldRow(label: label, value: shown, enabled: enabled, onChange: { typed = $0 })
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
        ChoiceField(
            label: label,
            value: choices.first { $0.1 == value }?.0 ?? choices.first?.0 ?? "",
            options: choices.map { $0.0 },
            enabled: enabled
        ) { index in
            if choices.indices.contains(index) { onPick(choices[index].1) }
        }
    }
}
