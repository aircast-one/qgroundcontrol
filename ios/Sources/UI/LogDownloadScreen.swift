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

private struct LogRow: View {
    let entry: LogEntry
    let enabled: Bool
    let onToggle: (Bool) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button { onToggle(!entry.selected) } label: {
            HStack(spacing: 16) {
                Image(entry.selected ? .checkCircle : .description)
                    .font(.system(size: 20))
                    .foregroundStyle(entry.selected ? theme.colors.onPrimary : theme.colors.onSecondaryContainer)
                    .frame(width: 40, height: 40)
                    .background(entry.selected ? theme.colors.primary : theme.colors.secondaryContainer, in: Circle())
                VStack(alignment: .leading, spacing: 2) {
                    Text("Log \(entry.id)").font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                    Text([entry.time, entry.sizeStr].filter { !$0.isBlank }.joined(separator: " · "))
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Text(entry.status).font(.labelMedium).foregroundStyle(theme.colors.primary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .frame(maxWidth: .infinity, minHeight: 72)
            .background(entry.selected ? theme.colors.secondaryContainer : .clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!(enabled && entry.received))
        .accessibilityAddTraits(entry.selected ? .isSelected : [])
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
                EmptyState(icon: .description, title: "No vehicle", text: logs.map { $0.emptyText.ifBlank("Connect a vehicle to download its flight logs.") } ?? "Connect a vehicle to download its flight logs.")
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

    private func connected(_ logs: LogsView) -> some View {
        let entries = logs.entries
        let selectedCount = entries.filter(\.selected).count
        let selectable = entries.filter(\.received)
        let busy = logs.busy
        return ZStack(alignment: .bottomTrailing) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 8) {
                    Button("Refresh") { invoke("refresh") }.disabled(!logs.canRefresh)
                    if !logs.sortText.isBlank {
                        Button(sentenceCase(logs.sortText)) { invoke("toggleSortByDate") }.disabled(!logs.canSort)
                    }
                }
                .buttonStyle(.bordered)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)

                if !selectable.isEmpty && !busy {
                    Toggle(isOn: Binding(get: { selectedCount == selectable.count }, set: { _ in
                        let selectAll = selectedCount < selectable.count
                        let indices = selectable.map(\.index)
                        offMain { indices.forEach { Qgc.set("\(LOG_MODEL).\($0).selected", selectAll) } }
                    })) {
                        Text(selectedCount < selectable.count ? "Select all" : "Deselect all").font(.labelLarge)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 4)
                }

                if busy {
                    let (title, progress) = downloadCard(entries)
                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Text(title).font(.titleSmall).frame(maxWidth: .infinity, alignment: .leading)
                            if !progress.isBlank {
                                Text(progress).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                            if logs.canCancel {
                                Button("Cancel") { invoke("cancel") }.buttonStyle(.borderless)
                            }
                        }
                        ProgressView().progressViewStyle(.linear)
                    }
                    .padding(16)
                    .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                }

                if entries.isEmpty {
                    EmptyState(icon: .description, title: "No logs yet", text: logs.emptyText.ifBlank("This vehicle reports no flight logs."))
                    Spacer(minLength: 0)
                } else {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            ForEach(logSections(entries), id: \.0) { title, section in
                                SectionHeader(text: title)
                                ForEach(section, id: \.index) { entry in
                                    LogRow(entry: entry, enabled: !busy) { checked in
                                        offMain { Qgc.set("\(LOG_MODEL).\(entry.index).selected", checked) }
                                    }
                                }
                            }
                            if !busy {
                                HStack(spacing: 8) {
                                    if logs.eraseSelectedShown {
                                        Button("Erase selected") { confirmErase = .Selected }.disabled(!logs.canEraseSelected)
                                    }
                                    Button("Erase all") { confirmErase = .All }.disabled(!logs.canErase)
                                }
                                .buttonStyle(.bordered)
                                .tint(theme.colors.error)
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                            }
                            Color.clear.frame(height: 88)
                        }
                    }
                }

                if let line = savedToText(logs) {
                    Text(line).font(.bodySmall).frame(maxWidth: .infinity, alignment: .leading).padding(16)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)

            if logs.canDownload && selectedCount > 0 {
                Button { invoke("download") } label: {
                    HStack(spacing: 12) {
                        Image(.download)
                        Text("Download (\(selectedCount))").font(.labelLarge)
                    }
                    .foregroundStyle(theme.colors.onPrimaryContainer)
                    .padding(.horizontal, 20)
                    .frame(height: 56)
                    .background(theme.colors.primaryContainer, in: RoundedRectangle(cornerRadius: Corner.large))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Download (\(selectedCount))")
                .padding(16)
            }
        }
        .task(id: logs.connected) {
            if shouldAutoRefreshLogs(logs.connected, busy) { invoke("refresh") }
        }
    }
}
