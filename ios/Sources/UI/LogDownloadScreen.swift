import SwiftUI

private let LOG_ROOT = "logDownload"
private let LOG_MODEL = "logDownload.model"
private let LOGS_VIEW = "view.logs"

let TIME_UNRECEIVED = "unreceived"
let TIME_UNKNOWN = "unknown"

private func dateFormatter(_ configure: (DateFormatter) -> Void) -> DateFormatter {
    let formatter = DateFormatter()
    configure(formatter)
    return formatter
}

private let CLOCK = dateFormatter { $0.dateStyle = .none; $0.timeStyle = .short }
private let DAY_THIS_YEAR = dateFormatter { $0.dateFormat = "MMM d" }
private let DAY_OTHER_YEAR = dateFormatter { $0.dateFormat = "MMM d, yyyy" }

func compactLogTime(_ time: Date, _ today: Date) -> String {
    let calendar = Calendar.current
    if calendar.isDate(time, inSameDayAs: today) { return CLOCK.string(from: time) }
    if let yesterday = calendar.date(byAdding: .day, value: -1, to: today), calendar.isDate(time, inSameDayAs: yesterday) { return "Yesterday" }
    return calendar.component(.year, from: time) == calendar.component(.year, from: today)
        ? DAY_THIS_YEAR.string(from: time)
        : DAY_OTHER_YEAR.string(from: time)
}

private let OFFSET_TIMES: [ISO8601DateFormatter] = ([[.withInternetDateTime], [.withInternetDateTime, .withFractionalSeconds]] as [ISO8601DateFormatter.Options]).map { options in
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = options
    return formatter
}

private let LOCAL_TIMES: [DateFormatter] = ["yyyy-MM-dd'T'HH:mm:ss", "yyyy-MM-dd'T'HH:mm:ss.SSS", "yyyy-MM-dd'T'HH:mm"].map { pattern in
    dateFormatter {
        $0.locale = Locale(identifier: "en_US_POSIX")
        $0.timeZone = .current
        $0.dateFormat = pattern
    }
}

func logLocalTime(_ raw: String) -> Date? {
    OFFSET_TIMES.lazy.compactMap { $0.date(from: raw) }.first ?? LOCAL_TIMES.lazy.compactMap { $0.date(from: raw) }.first
}

func logTimeText(_ raw: String, _ state: String, _ format: (Date) -> String) -> String {
    switch state {
    case TIME_UNRECEIVED: ""
    case TIME_UNKNOWN: "Date unknown"
    default: logLocalTime(raw).map(format) ?? "Date unknown"
    }
}

struct LogEntry: Equatable {
    let index: Int
    let id: Int
    let time: String
    let sizeStr: String
    let received: Bool
    let selected: Bool
    let status: String
    var downloading: Bool = false
    var saved: Bool = false
}

func logSections(_ entries: [LogEntry]) -> [(String, [LogEntry])] {
    [("On the vehicle", entries.filter { !$0.saved }), ("On this phone", entries.filter(\.saved))]
        .filter { !$0.1.isEmpty }
}

func logRows(_ entries: [LogEntry]) -> [(header: String?, entry: LogEntry)] {
    logSections(entries).flatMap { title, section in section.enumerated().map { ($0.offset == 0 ? title : nil, $0.element) } }
}

func downloadCard(_ entries: [LogEntry]) -> (String, String) {
    entries.first(where: \.downloading).map { ("Downloading log \($0.id)", $0.status) } ?? ("Downloading", "")
}

struct LogsView: Equatable {
    let connected: Bool
    let entries: [LogEntry]
    let emptyText: String
    let eraseWarning: String
    let canRefresh: Bool
    let canDownload: Bool
    let canCancel: Bool
    let canErase: Bool
    var eraseSelectedShown: Bool = false
    var canEraseSelected: Bool = false
    var canSort: Bool = false
    var sortText: String = ""
    let busy: Bool
    var downloading: Bool = false
    let anyDownloaded: Bool
    let savePath: String
    let savePathReason: String
}

func logsView(_ view: JSON?) -> LogsView? {
    guard let view else { return nil }
    let today = Date()
    return LogsView(
        connected: view["connected"].bool,
        entries: view["entries"].array.enumerated().filter { $0.element.object != nil }.map { index, entry in
            LogEntry(
                index: entry["index"].int(index),
                id: entry["id"].int(0),
                time: logTimeText(entry["time"].string, entry["timeState"].string) { compactLogTime($0, today) },
                sizeStr: entry["sizeText"].string,
                received: entry["received"].bool,
                selected: entry["selected"].bool,
                status: entry["status"].string,
                downloading: entry["statusId"].string == "downloading",
                saved: entry["statusId"].string == "downloaded"
            )
        },
        emptyText: view["emptyText"].string,
        eraseWarning: view["eraseWarning"].string,
        canRefresh: view["canRefresh"].bool,
        canDownload: view["canDownload"].bool,
        canCancel: view["canCancel"].bool,
        canErase: view["canErase"].bool,
        eraseSelectedShown: view["eraseSelectedShown"].bool,
        canEraseSelected: view["canEraseSelected"].bool,
        canSort: view["canSort"].bool,
        sortText: view["sortText"].string,
        busy: view["busy"].bool,
        downloading: view["downloading"].bool,
        anyDownloaded: view["anyDownloaded"].bool,
        savePath: view["savePath"].string,
        savePathReason: view["savePathReason"].string
    )
}

enum EraseKind {
    case All, Selected

    var action: String { self == .All ? "eraseAll" : "eraseSelected" }
    var title: String { self == .All ? "Delete All Onboard Log Files" : "Delete Selected Onboard Log Files" }
    var text: String {
        self == .All
            ? "All onboard log files will be erased permanently. Is this really what you want?"
            : "The selected onboard log files will be erased permanently. Is this really what you want?"
    }
    var confirm: String { self == .All ? "Erase All" : "Erase Selected" }
}

// A log as a selectable row: the selection circle leading, as in an editing list.
private struct LogRow: View {
    let entry: LogEntry
    let enabled: Bool
    let onToggle: (Bool) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let detail = [entry.time, entry.sizeStr].filter { !$0.isBlank }.joined(separator: " · ")
        Button { onToggle(!entry.selected) } label: {
            HStack(spacing: Space.s3) {
                Image(systemName: entry.selected ? "checkmark.circle.fill" : "circle")
                    .font(.title3)
                    .foregroundStyle(entry.selected ? theme.colors.primary : Color.secondary)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Log \(entry.id)").foregroundStyle(Color.primary)
                    if !detail.isBlank {
                        Text(detail).font(.subheadline).foregroundStyle(Color.secondary)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if !entry.status.isBlank {
                    Text(entry.status).font(.subheadline).foregroundStyle(Color.secondary).lineLimit(1)
                }
            }
            .contentShape(Rectangle())
        }
        .listRowButton()
        .disabled(!(enabled && entry.received))
        .accessibilityAddTraits(entry.selected ? .isSelected : [])
    }
}

private struct IndeterminateBar: View {
    @Environment(\.theme) private var theme

    var body: some View {
        TimelineView(.animation) { timeline in
            let phase = timeline.date.timeIntervalSinceReferenceDate.truncatingRemainder(dividingBy: 1.5) / 1.5
            GeometryReader { geometry in
                let width = geometry.size.width
                Capsule()
                    .fill(theme.colors.primary)
                    .frame(width: width * 0.4)
                    .offset(x: width * 1.4 * phase - width * 0.4)
            }
        }
        .frame(height: 4)
        .background(theme.colors.secondaryContainer)
        .clipShape(Capsule())
        .accessibilityElement()
        .accessibilityLabel("In progress")
    }
}

func savedToText(_ logs: LogsView?) -> String? {
    guard let reading = logs, reading.anyDownloaded else { return nil }
    if !reading.savePath.isBlank { return "Saved to \(reading.savePath)" }
    if !reading.savePathReason.isBlank { return reading.savePathReason }
    return nil
}

func shouldAutoRefreshLogs(_ hasVehicle: Bool, _ busy: Bool) -> Bool { hasVehicle && !busy }

struct LogDownloadScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(LOGS_VIEW) private var json
    @State private var confirmErase: EraseKind?

    var body: some View {
        let logs = logsView(json)
        Group {
            if let logs, logs.connected {
                connected(logs)
            } else {
                EmptyState(icon: .description, title: "No vehicle", text: (logs?.emptyText ?? "").ifBlank("Connect a vehicle to download its flight logs."))
                    .frame(maxHeight: .infinity, alignment: .top)
            }
        }
        .background(BlocksNavigation(blocking: logs?.downloading == true, reason: LOG_DOWNLOAD_BLOCK))
        .alert(confirmErase?.title ?? "", isPresented: Binding(get: { confirmErase != nil }, set: { if !$0 { confirmErase = nil } }), presenting: confirmErase) { erase in
            Button(erase.confirm, role: .destructive) { offMain { Qgc.invoke("\(LOG_ROOT).\(erase.action)") } }
            Button("Cancel", role: .cancel) { confirmErase = nil }
        } message: { erase in
            Text(erase.text)
        }
    }

    private func invoke(_ action: String) {
        offMain { Qgc.invoke("\(LOG_ROOT).\(action)") }
    }

    // The logs as a native list: a section per place a log is kept, Download as the page's primary action, Refresh
    // and the sort order in the navigation bar's menu, and the erase actions as destructive rows of their own.
    private func connected(_ logs: LogsView) -> some View {
        let entries = logs.entries
        let selectedCount = entries.filter(\.selected).count
        let selectable = entries.filter(\.received)
        let busy = logs.busy
        return NativeForm {
            if busy {
                let (title, progress) = downloadCard(entries)
                Section {
                    VStack(alignment: .leading, spacing: Space.s2) {
                        LabeledContent(title, value: progress)
                        IndeterminateBar()
                    }
                    .padding(.vertical, Space.s1)
                    if logs.canCancel {
                        Button("Cancel download") { invoke("cancel") }
                    }
                }
            }
            if entries.isEmpty {
                Section {
                    EmptyState(icon: .description, title: "No logs yet", text: logs.emptyText.ifBlank("This vehicle reports no flight logs."))
                        .listRowBackground(Color.clear)
                }
            } else {
                if !selectable.isEmpty && !busy {
                    Section {
                        Button(selectedCount < selectable.count ? "Select all" : "Deselect all") {
                            let selectAll = selectedCount < selectable.count
                            let indices = selectable.map(\.index)
                            offMain { indices.forEach { Qgc.set("\(LOG_MODEL).\($0).selected", selectAll) } }
                        }
                    }
                }
                ForEach(logSections(entries), id: \.0) { title, section in
                    Section {
                        ForEach(section, id: \.index) { entry in
                            LogRow(entry: entry, enabled: !busy) { checked in
                                offMain { Qgc.set("\(LOG_MODEL).\(entry.index).selected", checked) }
                            }
                        }
                    } header: {
                        SectionHeader(text: title)
                    }
                }
                if !busy {
                    Section {
                        if logs.eraseSelectedShown {
                            Button("Erase selected…", role: .destructive) { confirmErase = .Selected }.disabled(!logs.canEraseSelected)
                        }
                        Button("Erase all…", role: .destructive) { confirmErase = .All }.disabled(!logs.canErase)
                    }
                }
            }
            if let line = savedToText(logs) {
                FormNote(text: line)
            }
        }
        .pagePrimaryAction(selectedCount > 0 ? "Download (\(selectedCount))" : "Download", enabled: logs.canDownload && selectedCount > 0) {
            invoke("download")
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button { invoke("refresh") } label: { Label("Refresh", systemImage: "arrow.clockwise") }
                        .disabled(!logs.canRefresh)
                    if !logs.sortText.isBlank {
                        Button { invoke("toggleSortByDate") } label: { Label(sentenceCase(logs.sortText), systemImage: "arrow.up.arrow.down") }
                            .disabled(!logs.canSort)
                    }
                } label: {
                    Label("More", systemImage: "ellipsis.circle")
                }
            }
        }
        .task(id: logs.connected) {
            if shouldAutoRefreshLogs(logs.connected, busy) { invoke("refresh") }
        }
    }
}
