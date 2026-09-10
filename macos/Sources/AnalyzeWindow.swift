import AppKit
import SwiftUI

struct VibrationBar: View {
    static let captionHeight = 42.0

    let axis: VibrationAxis
    let reading: VibrationReading

    var body: some View {
        VStack(spacing: 6) {
            GeometryReader { geometry in
                let height = geometry.size.height

                ZStack(alignment: .bottom) {
                    Rectangle()
                        .stroke(Color.secondary.opacity(0.5), lineWidth: 1)
                    Rectangle()
                        .fill(colour)
                        .frame(height: (axis.fraction ?? 0) * height)
                    threshold(reading.dangerLevel, in: height)
                    threshold(reading.warningLevel, in: height)
                }
            }
            .frame(width: 54)

            Text(axis.value.map { String(format: "%.1f", $0) } ?? "\u{2014}")
                .font(.body.monospacedDigit())
            Text(axis.label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private var colour: Color {
        switch axis.severity {
        case .danger: return .red
        case .warning: return .orange
        case .normal: return .accentColor
        case nil: return .secondary
        }
    }

    private func threshold(_ level: Double, in height: Double) -> some View {
        Rectangle()
            .fill(Color.red)
            .frame(height: 1)
            .offset(y: -(level / reading.scaleMaximum) * height)
            .frame(maxHeight: .infinity, alignment: .bottom)
    }
}

struct VibrationScale: View {
    let reading: VibrationReading

    private var marks: [Double] {
        [reading.scaleMaximum, reading.dangerLevel, reading.warningLevel, 0]
    }

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .bottom) {
                ForEach(marks, id: \.self) { mark in
                    Text(String(Int(mark)))
                        .font(.caption2.monospacedDigit())
                        .foregroundColor(mark == reading.dangerLevel || mark == reading.warningLevel
                                         ? .red : .secondary)
                        .offset(y: -(mark / reading.scaleMaximum) * geometry.size.height + 6)
                        .frame(maxHeight: .infinity, alignment: .bottom)
                }
            }
        }
        .frame(width: 22)
        .padding(.bottom, VibrationBar.captionHeight)
    }
}

struct VibrationView: View {
    @ObservedObject var store: VibrationStore

    var body: some View {
        SetupPageBody(title: "Vibration",
                      note: "Live vibration on each axis, and whether it is safe to fly.") {
            if store.reading.available {
                GroupCard {
                    VStack(alignment: .leading, spacing: Overlay.unit * 0.75) {
                        HStack(alignment: .bottom, spacing: 28) {
                            VibrationScale(reading: store.reading)
                            ForEach(store.reading.axes) { axis in
                                VibrationBar(axis: axis, reading: store.reading)
                            }
                        }
                        .frame(height: 190)
                        HStack(spacing: 6) {
                            Image(systemName: adviceSymbol)
                            Text(advice)
                        }
                        .font(.callout)
                        .foregroundColor(adviceColour)
                    }
                    .padding(Overlay.unit)
                }

                VStack(alignment: .leading, spacing: 0) {
                    SectionLabel(text: "Accelerometer clipping")
                    GroupCard {
                        ForEach(Array(store.reading.clipCounts.enumerated()), id: \.offset) { index, count in
                            GroupRow(title: "Accelerometer \(index + 1)",
                                     value: "\(count) \(count == 1 ? "clip" : "clips")",
                                     showSeparator: index > 0,
                                     trailing: {
                                         Image(systemName: count == 0
                                             ? "checkmark.circle.fill" : "exclamationmark.circle.fill")
                                             .foregroundColor(count == 0 ? .green : .orange)
                                     })
                        }
                    }
                    Text("Counted since the vehicle booted. Any clipping means the accelerometer saturated.")
                        .font(.caption).foregroundColor(.secondary)
                        .padding(.horizontal, Overlay.horizontalPadding)
                        .padding(.top, Overlay.unit * 0.35)
                }
            } else {
                GroupCard { EmptyStateRow(text: "This vehicle is not reporting vibration.") }
            }
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
    }

    private var adviceSymbol: String {
        switch store.reading.worst {
        case .danger: return "exclamationmark.triangle.fill"
        case .warning: return "exclamationmark.circle.fill"
        case .normal: return "checkmark.circle.fill"
        case nil: return "questionmark.circle"
        }
    }

    private var advice: String {
        switch store.reading.worst {
        case .danger: return "Above \(Int(store.reading.dangerLevel)) — do not fly until this is fixed."
        case .warning: return "Above \(Int(store.reading.warningLevel)) — attitude estimation may degrade."
        case .normal: return "Within normal range."
        case nil: return "No axis is reporting a level yet."
        }
    }

    private var adviceColour: Color {
        switch store.reading.worst {
        case .danger: return .red
        case .warning: return .orange
        case .normal: return .secondary
        case nil: return .secondary
        }
    }
}

struct LogDownloadView: View {
    @ObservedObject var store: LogDownloadStore

    var body: some View {
        SetupPageBody(title: "Log Download",
                      note: store.savePath.isEmpty
                          ? "Flight logs stored on the vehicle."
                          : "Flight logs stored on the vehicle. Downloads are saved to \(store.savePath).") {
            GroupCard {
                if !store.status.isEmpty {
                    EmptyStateRow(text: store.status)
                } else if store.logs.isEmpty {
                    EmptyStateRow(text: store.emptyText)
                } else {
                    ForEach(Array(store.logs.enumerated()), id: \.element.id) { index, entry in
                        GroupRow(title: "Log \(entry.id)",
                                 description: entry.timeText,
                                 value: entry.status == "Available" ? entry.sizeText
                                     : "\(entry.sizeText) \u{00B7} \(entry.status)",
                                 showSeparator: index > 0,
                                 leading: {
                                     Tile(symbol: "doc.text.fill",
                                          colour: entry.received ? .gray : .accentColor)
                                 },
                                 trailing: {
                                     Button("Download") { store.download(entry) }
                                         .disabled(!store.canDownload)
                                 })
                    }
                }
            }

            HStack(spacing: Overlay.step) {
                Button("Refresh", action: store.refresh)
                    .disabled(!store.canRefresh)
                    .help(store.connected
                        ? "Ask the vehicle for its logs"
                        : "Connect a vehicle to list its logs")
                if store.downloading || store.requestingList {
                    ProgressView().controlSize(.small)
                    Button("Cancel", action: store.cancel)
                }
                Spacer()
                Button("Erase All\u{2026}", action: store.askToEraseAll)
                    .disabled(!store.canErase)
            }
        }
        .onAppear(perform: store.startWatching)
        .onDisappear(perform: store.stopWatching)
        .alert("Erase every log on the vehicle?", isPresented: $store.confirmingErase) {
            Button("Cancel", role: .cancel) { }
            Button("Erase All", role: .destructive, action: store.eraseAll)
        } message: {
            Text(store.eraseWarning)
        }
    }
}

struct GeoTagView: View {
    @ObservedObject var store: GeoTagStore

    private var home: String { NSHomeDirectory() }

    var body: some View {
        SetupPageBody(title: "Geotag Images",
                      note: "Writes the position from a flight log into the images a survey took, so they carry where they were shot.") {
            GroupCard {
                pathRow("Flight log", value: store.job.logFile,
                        placeholder: "No log chosen", separator: false,
                        choose: store.chooseLogFile)
                pathRow("Image folder", value: store.job.imageDirectory,
                        placeholder: "No folder chosen", separator: true,
                        choose: store.chooseImageDirectory)
                pathRow("Save to", value: store.job.saveDirectory,
                        placeholder: store.job.imageDirectory.isEmpty
                            ? "A TAGGED folder beside your images"
                            : shorten(store.job.destination),
                        separator: true,
                        choose: store.chooseSaveDirectory)
            }

            if !store.job.errorMessage.isEmpty {
                Label(store.job.errorMessage, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout)
                    .foregroundColor(store.job.failed ? .red : .orange)
                    .fixedSize(horizontal: false, vertical: true)
            }

            HStack(spacing: Overlay.unit * 0.75) {
                if store.job.busy {
                    Button("Cancel", action: store.cancel)
                    ProgressView(value: store.job.progress, total: 100)
                        .frame(maxWidth: 220)
                    Text(store.job.progressText)
                        .font(.callout.monospacedDigit())
                        .foregroundColor(.secondary)
                } else {
                    Button(store.job.failed ? "Try Again" : "Start Tagging", action: store.start)
                        .keyboardShortcut(.defaultAction)
                        .disabled(!store.job.canStart)
                    if store.job.finished {
                        Label("Tagged", systemImage: "checkmark.circle.fill")
                            .font(.callout)
                            .foregroundColor(.green)
                    }
                }
                Spacer()
            }
        }
        .onAppear(perform: store.reload)
        .writeFailureAlert($store.writeFailure)
    }

    private func shorten(_ path: String) -> String {
        GeoTagJob.shortPath(path, home: home)
    }

    private func pathRow(_ title: String, value: String, placeholder: String,
                         separator: Bool, choose: @escaping () -> Void) -> some View {
        GroupRow(title: title, showSeparator: separator, trailing: {
            HStack(spacing: Overlay.step) {
                Text(value.isEmpty ? placeholder : shorten(value))
                    .font(.callout)
                    .foregroundColor(value.isEmpty ? .secondary : Overlay.value)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .frame(maxWidth: 320, alignment: .trailing)
                    .help(value.isEmpty ? placeholder : value)
                Button("Choose\u{2026}", action: choose)
                    .disabled(store.job.busy)
            }
        })
    }
}

struct MavlinkInspectorView: View {
    @ObservedObject var store: MavlinkInspectorStore

    static let listHeight: CGFloat = 300

    var body: some View {
        SetupPageBody(title: "MAVLink Inspector",
                      note: store.listening
                          ? "Every message system \(store.systemId) is sending, how often it arrives, and what is inside the one you pick."
                          : "Every message the vehicle sends, how often it arrives, and what is inside the one you pick.") {
            if !store.listening {
                GroupCard { EmptyStateRow(text: "No vehicle is talking yet.") }
            } else {
                HStack(alignment: .top, spacing: Overlay.unit) {
                    GroupCard {
                        ScrollView {
                            VStack(spacing: 0) {
                                ForEach(Array(store.messages.enumerated()), id: \.element.id) { row, message in
                                    Button {
                                        store.select(message)
                                    } label: {
                                        GroupRow(title: message.title,
                                                 description: "#\(message.messageId)",
                                                 value: message.rateText,
                                                 showSeparator: row > 0,
                                                 current: message.selected)
                                    }
                                    .buttonStyle(.plain)
                                }
                            }
                        }
                    }
                    .frame(width: 260, height: MavlinkInspectorView.listHeight)

                    fields
                        .frame(height: MavlinkInspectorView.listHeight, alignment: .top)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
    }

    @ViewBuilder private var fields: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
            if let message = store.selected {
                SectionLabel(text: "\(message.title) \u{00B7} \(message.countText) received")
                GroupCard {
                    GroupRow(title: "Arriving at", value: message.rateText,
                             showSeparator: false)
                    GroupRow(title: "Ask for", trailing: {
                        Picker("", selection: Binding(
                            get: { MessageRateChoice.shown(message.targetRateHz, in: store.rateChoices) },
                            set: { store.setRate($0) })
                        ) {
                            ForEach(store.rateChoices) {
                                Text($0.title).tag($0.rate)
                            }
                        }
                        .labelsHidden()
                        .frame(width: 110)
                    })
                }
                GroupCard {
                    ScrollView {
                        VStack(spacing: 0) {
                            if store.fields.isEmpty {
                                EmptyStateRow(text: "This message carries no fields.")
                            } else {
                                ForEach(Array(store.fields.enumerated()), id: \.element.id) { row, field in
                                    GroupRow(title: field.name,
                                             description: field.type,
                                             value: field.value.isEmpty ? "\u{2014}" : field.value,
                                             showSeparator: row > 0)
                                }
                            }
                        }
                    }
                }
            } else {
                GroupCard { EmptyStateRow(text: "Pick a message to watch its fields.") }
            }
        }
    }
}

struct MavlinkConsoleView: View {
    @ObservedObject var store: MavlinkConsoleStore

    // QML gates its own scroll on flickable.atYEnd, so scrolling up freezes the view until you
    // come back and the operator manages nothing. SwiftUI cannot read scroll offset at this
    // deployment target - onScrollGeometryChange is macOS 15 - so this asks the operator instead.
    // A checkbox is the compromise, not the design; delete it when the target moves.
    @State private var following = true

    var body: some View {
        // Not passing connected: SetupPageBody's own disconnected state says "Connect a vehicle
        // to set this up", which is setup language for a page that sets nothing up. The two
        // sentences this page needs are its own, and they tell a silent vehicle apart from none.
        SetupPageBody(title: "MAVLink Console",
                      note: MavlinkConsole.readOnlyNotice) {
            GroupCard {
                if store.console.describes {
                    HStack(spacing: Overlay.unit * 0.5) {
                        Toggle("Follow new output", isOn: $following)
                            .toggleStyle(.checkbox)
                        Spacer()
                        Button("Copy All") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(store.console.copyable,
                                                           forType: .string)
                        }
                    }
                    .padding(.horizontal, Overlay.unit * 0.4)
                    Divider()
                    ScrollViewReader { scroll in
                        ScrollView {
                            VStack(alignment: .leading, spacing: 1) {
                                ForEach(Array(store.console.lines.enumerated()), id: \.offset) {
                                    index, line in
                                    Text(line)
                                        .font(.system(.body, design: .monospaced))
                                        .textSelection(.enabled)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .id(index)
                                }
                            }
                            .padding(Overlay.unit * 0.4)
                        }
                        .frame(minHeight: 240)
                        .onChange(of: store.console.lines.count) { count in
                            guard following, count > 0 else { return }
                            scroll.scrollTo(count - 1, anchor: .bottom)
                        }
                    }
                } else {
                    EmptyStateRow(text: store.console.emptyText)
                }
            }
        }
        .onAppear(perform: store.startWatching)
        .onDisappear(perform: store.stopWatching)
    }
}

struct AnalyzeView: View {
    @ObservedObject var vibration: VibrationStore
    @ObservedObject var logs: LogDownloadStore
    @ObservedObject var geoTag: GeoTagStore
    @ObservedObject var inspector: MavlinkInspectorStore
    @ObservedObject var console: MavlinkConsoleStore
    @ObservedObject var selection: PageSelection

    static let pages = ["Vibration", "Log Download", "Geotag Images", "MAVLink Inspector",
                        "MAVLink Console"]

    static let symbols = ["Vibration": "waveform.path.ecg", "Log Download": "doc.text.fill",
                          "Geotag Images": "mappin.and.ellipse",
                          "MAVLink Inspector": "dot.radiowaves.left.and.right",
                          "MAVLink Console": "terminal"]
    static let colours: [String: Color] = ["Vibration": .pink, "Log Download": .indigo,
                                           "Geotag Images": .teal, "MAVLink Inspector": .orange,
                                           "MAVLink Console": .gray]

    var body: some View {
        HStack(spacing: 0) {
            List(AnalyzeView.pages, id: \.self, selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) { name in
                SidebarRow(title: name,
                           symbol: AnalyzeView.symbols[name] ?? "doc.text.fill",
                           colour: AnalyzeView.colours[name] ?? .indigo)
                    .tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 200)
            Divider()
            switch selection.page {
            case "Log Download": LogDownloadView(store: logs)
            case "Geotag Images": GeoTagView(store: geoTag)
            case "MAVLink Inspector": MavlinkInspectorView(store: inspector)
            case "MAVLink Console": MavlinkConsoleView(store: console)
            default: VibrationView(store: vibration)
            }
        }
        .frame(minWidth: 720, minHeight: 460)
    }
}

final class AnalyzeWindow: NSObject, NSWindowDelegate {
    static let shared = AnalyzeWindow()

    private let vibration = VibrationStore()
    private let logs = LogDownloadStore()
    private let geoTag = GeoTagStore()
    private let inspector = MavlinkInspectorStore()
    private let console = MavlinkConsoleStore()
    private let selection = PageSelection(owner: "analyze", pages: AnalyzeView.pages)
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(vibration)
        NativeProbe.register(logs)
        NativeProbe.register(geoTag)
        NativeProbe.register(inspector)
        NativeProbe.register(console)
        NativeProbe.register(selection, as: selection.identifier)
    }

    @objc func showFromMenu() {
        show()
    }

    func show() {
        if let window {
            window.makeKeyAndOrderFront(nil)
            return
        }

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 700, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Analyze"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: AnalyzeView(vibration: vibration, logs: logs, geoTag: geoTag, inspector: inspector, console: console, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        vibration.stop()
        inspector.stop()
        logs.stopWatching()
        window = nil
    }
}
