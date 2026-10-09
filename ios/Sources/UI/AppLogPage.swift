import SwiftUI
import UniformTypeIdentifiers

let APP_LOG_VIEW = "view.appLog"
let APP_LOG_CLEAR = "appLog.clear"
let APP_LOG_SAVE = "appLog.save"
let APP_LOG_SAVE_FORMAT = "settings.logManagerSettings.saveFormat.rawValue"
private let SAVE_FORMAT_CSV = 1
private let APP_LOG_POLL_MS = 500
private let FILTER_DEBOUNCE_MS = 200
private let LEVEL_WARNING = 2
private let LEVEL_CRITICAL = 3

struct AppLogFilter: Equatable {
    var levelIndex: Int = 0
    var category: String = ""
    var text: String = ""
    var regex: Bool = false
}

struct AppLogEntry: Equatable {
    let sequence: Int64
    let level: Int
    let message: String
    let category: String
    let timestamp: String
    let source: String
}

struct AppLogRead: Equatable {
    let levels: [String]
    let categories: [String]
    let regexValid: Bool
    let first: Int64?
    let entries: [AppLogEntry]
}

private let formSafe = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-*_ ")

private func encoded(_ text: String) -> String {
    (text.addingPercentEncoding(withAllowedCharacters: formSafe) ?? text).replacingOccurrences(of: " ", with: "+")
}

func appLogPath(_ filter: AppLogFilter, _ after: Int64?) -> String {
    "\(APP_LOG_VIEW)(\(max(filter.levelIndex - 1, 0)),\(encoded(filter.category)),\(encoded(filter.text)),\(filter.regex ? 1 : 0),\(after.map(String.init) ?? ""))"
}

func appLogRead(_ view: JSON?) -> AppLogRead? {
    guard let view, view["class"].string == "AppLog" else { return nil }
    return AppLogRead(
        levels: view["levels"].strings,
        categories: view["categories"].strings,
        regexValid: view["regexValid"].bool(true),
        first: view["first"].isNull ? nil : view["first"].int64 ?? 0,
        entries: view["entries"].array.filter { $0.object != nil }.map { entry in
            AppLogEntry(
                sequence: entry["sequence"].int64 ?? 0,
                level: entry["level"].int(0),
                message: entry["message"].string,
                category: entry["category"].string,
                timestamp: entry["timestamp"].string,
                source: entry["source"].string
            )
        }
    )
}

func mergedEntries(_ held: [AppLogEntry], _ read: AppLogRead) -> [AppLogEntry] {
    held.filter { entry in read.first.map { entry.sequence >= $0 } ?? false } + read.entries
}

func appLogFileName(_ saveFormat: JSON?) -> String {
    saveFormat?["value"].int == SAVE_FORMAT_CSV ? "QGCConsole.csv" : "QGCConsole.txt"
}

func appLogMime(_ fileName: String) -> String {
    fileName.lowercased().hasSuffix(".csv") ? "text/csv" : "text/plain"
}

private struct AppLogDocument: FileDocument {
    static let readableContentTypes: [UTType] = [.plainText, .commaSeparatedText]
    let text: String

    init(text: String) { self.text = text }

    init(configuration: ReadConfiguration) throws {
        text = configuration.file.regularFileContents.flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}

private func textOf(_ json: JSON) -> String? {
    if case .string(let text) = json { return text }
    return nil
}

private func levelColor(_ level: Int, _ theme: Theme) -> Color {
    switch level {
    case 0: theme.colors.onSurfaceVariant
    case LEVEL_WARNING: theme.aircast.warning
    case LEVEL_CRITICAL...: theme.aircast.alert
    default: theme.colors.onSurface
    }
}

private struct PollKey: Equatable {
    let filter: AppLogFilter
    let cleared: Int
}

private struct FollowKey: Equatable {
    let count: Int
    let following: Bool
}

private let LOG_END = "log-end"

struct AppLogPage: View {
    @Environment(\.theme) private var theme
    @State private var filter = AppLogFilter()
    @State private var entries: [AppLogEntry] = []
    @State private var read: AppLogRead?
    @State private var cleared = 0
    @State private var notice: String?
    @State private var atBottom = true
    @State private var following = true
    @State private var showCategories = false
    @State private var saveName = appLogFileName(nil)
    @State private var saving: AppLogDocument?

    var body: some View {
        VStack(spacing: 0) {
            ZStack(alignment: .top) {
                if entries.isEmpty {
                    Text("No log entries")
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(Array(entries.enumerated()), id: \.element.sequence) { index, entry in
                                AppLogRow(entry: entry, index: index)
                            }
                            Color.clear
                                .frame(height: 1)
                                .id(LOG_END)
                                .onAppear { atBottom = true }
                                .onDisappear { atBottom = false }
                        }
                    }
                    .onChange(of: FollowKey(count: entries.count, following: following)) {
                        if following && !entries.isEmpty { proxy.scrollTo(LOG_END, anchor: .bottom) }
                    }
                }
                if !atBottom && !entries.isEmpty {
                    Button("Show latest") { following = true }
                        .buttonStyle(.borderedProminent)
                        .padding(.top, Space.s2)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .onChange(of: atBottom) { if !atBottom { following = false } }
            Divider()
            AppLogFilterBar(
                filter: filter,
                read: read,
                onFilter: { filter = $0 },
                onCategories: { showCategories = true },
                onSave: save,
                onClear: {
                    Task {
                        _ = await offMain { Qgc.invoke(APP_LOG_CLEAR) }
                        cleared += 1
                    }
                }
            )
            if let notice {
                Text(notice)
                    .foregroundStyle(theme.aircast.alert)
                    .padding(.horizontal, Space.s3)
                    .padding(.vertical, Space.s1)
            }
        }
        .task(id: PollKey(filter: filter, cleared: cleared)) {
            try? await Task.sleep(for: .milliseconds(FILTER_DEBOUNCE_MS))
            guard !Task.isCancelled else { return }
            entries = []
            while !Task.isCancelled {
                let path = appLogPath(filter, entries.last?.sequence)
                let next = await offMain { appLogRead(Qgc.get(path)) }
                if let next, !Task.isCancelled {
                    read = next
                    entries = mergedEntries(entries, next)
                }
                try? await Task.sleep(for: .milliseconds(APP_LOG_POLL_MS))
            }
        }
        .sheet(isPresented: $showCategories) {
            LoggingCategoriesDialog(onDismiss: { showCategories = false })
        }
        .fileExporter(
            isPresented: Binding(get: { saving != nil }, set: { if !$0 { saving = nil } }),
            document: saving,
            contentType: UTType(mimeType: appLogMime(saveName)) ?? .plainText,
            defaultFilename: saveName
        ) { result in
            switch result {
            case .success: notice = nil
            case .failure(let error): notice = (error as? CocoaError)?.code == .userCancelled ? notice : "The file could not be written."
            }
        }
    }

    private func save() {
        Task {
            let (name, saved) = await offMain { () -> (String, String?) in
                let name = appLogFileName(Qgc.get(APP_LOG_SAVE_FORMAT))
                return (name, textOf(Qgc.invokeResult(APP_LOG_SAVE, name)))
            }
            saveName = name
            if let saved {
                saving = AppLogDocument(text: saved)
            } else {
                notice = "The log could not be read."
            }
        }
    }
}

private struct AppLogRow: View {
    let entry: AppLogEntry
    let index: Int
    @Environment(\.theme) private var theme

    var body: some View {
        let colour = levelColor(entry.level, theme)
        VStack(alignment: .leading, spacing: 0) {
            Text(entry.message)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(colour)
                .lineLimit(2)
                .truncationMode(.tail)
            Text([entry.timestamp, entry.category, entry.source].filter { !$0.isEmpty }.joined(separator: "  "))
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .lineLimit(1)
                .truncationMode(.tail)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, Space.s1)
        .padding(.leading, 3 + Space.s2)
        .padding(.trailing, Space.s2)
        .overlay(alignment: .leading) {
            Rectangle().fill(entry.level >= LEVEL_WARNING ? colour : .clear).frame(width: 3)
        }
        .background(index % 2 == 0 ? theme.colors.surface : theme.colors.surfaceContainer)
    }
}

private struct AppLogFilterBar: View {
    let filter: AppLogFilter
    let read: AppLogRead?
    let onFilter: (AppLogFilter) -> Void
    let onCategories: () -> Void
    let onSave: () -> Void
    let onClear: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: Space.s2) {
                    let levels = read?.levels ?? []
                    OptionMenu(
                        label: sentenceCase(levels.indices.contains(filter.levelIndex) ? levels[filter.levelIndex] : "All Levels"),
                        options: levels.map(sentenceCase)
                    ) { index in onFilter(with(filter) { $0.levelIndex = index }) }
                    let categories = read?.categories ?? []
                    OptionMenu(
                        label: sentenceCase(filter.category.ifEmpty("All Categories")),
                        options: categories.map(sentenceCase)
                    ) { index in onFilter(with(filter) { $0.category = index == 0 ? "" : categories[index] }) }
                    Button("Categories", action: onCategories).buttonStyle(.bordered)
                    Button("Save", action: onSave).buttonStyle(.bordered)
                    Button("Clear", action: onClear).buttonStyle(.bordered)
                }
            }
            HStack(spacing: Space.s2) {
                TextField("Search…", text: Binding(get: { filter.text }, set: { text in onFilter(with(filter) { $0.text = text }) }))
                    .textFieldStyle(.roundedBorder)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                    .overlay {
                        if read?.regexValid == false {
                            RoundedRectangle(cornerRadius: 6).stroke(theme.colors.error, lineWidth: 1)
                        }
                    }
                Toggle(isOn: Binding(get: { filter.regex }, set: { regex in onFilter(with(filter) { $0.regex = regex }) })) {
                    Text(".*").monospaced()
                }
                .toggleStyle(.button)
            }
        }
        .padding(Space.s2)
        .background(theme.colors.surfaceContainer)
    }
}

private func with(_ filter: AppLogFilter, _ change: (inout AppLogFilter) -> Void) -> AppLogFilter {
    var copy = filter
    change(&copy)
    return copy
}

private struct OptionMenu: View {
    let label: String
    let options: [String]
    let onPick: (Int) -> Void

    var body: some View {
        Menu {
            ForEach(Array(options.enumerated()), id: \.offset) { index, option in
                Button(option) { onPick(index) }
            }
        } label: {
            Text(label)
        }
        .disabled(options.isEmpty)
    }
}
