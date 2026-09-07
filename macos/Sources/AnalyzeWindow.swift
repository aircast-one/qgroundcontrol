import AppKit
import SwiftUI

struct VibrationBar: View {
    // Height of the value + label rows beneath each bar.
    static let captionHeight = 42.0

    let label: String
    let value: Double

    var body: some View {
        VStack(spacing: 6) {
            GeometryReader { geometry in
                let height = geometry.size.height
                let filled = min(value / VibrationReading.scaleMaximum, 1) * height

                ZStack(alignment: .bottom) {
                    Rectangle()
                        .stroke(Color.secondary.opacity(0.5), lineWidth: 1)
                    Rectangle()
                        .fill(colour)
                        .frame(height: filled)
                    threshold(VibrationReading.dangerLevel, in: height)
                    threshold(VibrationReading.warningLevel, in: height)
                }
            }
            .frame(width: 54)

            Text(String(format: "%.1f", value))
                .font(.body.monospacedDigit())
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private var colour: Color {
        switch VibrationReading.severity(value) {
        case .danger: return .red
        case .warning: return .orange
        case .normal: return .accentColor
        }
    }

    private func threshold(_ level: Double, in height: Double) -> some View {
        Rectangle()
            .fill(Color.red)
            .frame(height: 1)
            .offset(y: -(level / VibrationReading.scaleMaximum) * height)
            .frame(maxHeight: .infinity, alignment: .bottom)
    }
}

// Two unlabelled red lines are just decoration; the numbers are the whole point of the
// page, so the scale states them.
struct VibrationScale: View {
    private let marks = [VibrationReading.scaleMaximum, VibrationReading.dangerLevel,
                         VibrationReading.warningLevel, 0.0]

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .bottom) {
                ForEach(marks, id: \.self) { mark in
                    Text(String(Int(mark)))
                        .font(.caption2.monospacedDigit())
                        .foregroundColor(mark == VibrationReading.dangerLevel || mark == VibrationReading.warningLevel
                                         ? .red : .secondary)
                        .offset(y: -(mark / VibrationReading.scaleMaximum) * geometry.size.height + 6)
                        .frame(maxHeight: .infinity, alignment: .bottom)
                }
            }
        }
        .frame(width: 22)
        // The bars sit above a value row and a label row; the scale has to skip both
        // to line its marks up with the bar itself.
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
                            VibrationScale()
                            VibrationBar(label: "X", value: store.reading.x)
                            VibrationBar(label: "Y", value: store.reading.y)
                            VibrationBar(label: "Z", value: store.reading.z)
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
        }
    }

    private var advice: String {
        switch store.reading.worst {
        case .danger: return "Above \(Int(VibrationReading.dangerLevel)) — do not fly until this is fixed."
        case .warning: return "Above \(Int(VibrationReading.warningLevel)) — attitude estimation may degrade."
        case .normal: return "Within normal range."
        }
    }

    private var adviceColour: Color {
        switch store.reading.worst {
        case .danger: return .red
        case .warning: return .orange
        case .normal: return .secondary
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
                    EmptyStateRow(text: store.requestingList
                        ? "Asking the vehicle for its logs\u{2026}"
                        : "No logs listed yet. Refresh to ask the vehicle.")
                } else {
                    ForEach(Array(store.logs.enumerated()), id: \.element.id) { index, entry in
                        GroupRow(title: "Log \(entry.id)",
                                 description: entry.time,
                                 value: entry.status == "Available" ? entry.sizeText
                                     : "\(entry.sizeText) \u{00B7} \(entry.status)",
                                 showSeparator: index > 0,
                                 leading: {
                                     Tile(symbol: "doc.text.fill",
                                          colour: entry.received ? .gray : .accentColor)
                                 },
                                 trailing: {
                                     Button("Download") { store.download(entry) }
                                         .disabled(store.downloading)
                                 })
                    }
                }
            }

            HStack(spacing: Overlay.step) {
                Button("Refresh", action: store.refresh)
                    .disabled(store.requestingList || store.downloading)
                if store.downloading || store.requestingList {
                    ProgressView().controlSize(.small)
                    Button("Cancel", action: store.cancel)
                }
                Spacer()
            }
        }
        .onAppear(perform: store.reload)
    }
}

struct AnalyzeView: View {
    @ObservedObject var vibration: VibrationStore
    @ObservedObject var logs: LogDownloadStore
    @ObservedObject var selection: PageSelection

    static let pages = ["Vibration", "Log Download"]

    var body: some View {
        HStack(spacing: 0) {
            List(AnalyzeView.pages, id: \.self, selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) { name in
                SidebarRow(title: name,
                           symbol: name == "Vibration" ? "waveform.path.ecg" : "doc.text.fill",
                           colour: name == "Vibration" ? .pink : .indigo)
                    .tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 200)
            Divider()
            if selection.page == "Log Download" {
                LogDownloadView(store: logs)
            } else {
                VibrationView(store: vibration)
            }
        }
        .frame(minWidth: 720, minHeight: 460)
    }
}

final class AnalyzeWindow: NSObject, NSWindowDelegate {
    static let shared = AnalyzeWindow()

    private let vibration = VibrationStore()
    private let logs = LogDownloadStore()
    private let selection = PageSelection(owner: "analyze", pages: AnalyzeView.pages)
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(vibration)
        NativeProbe.register(logs)
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
        window.contentView = NSHostingView(rootView: AnalyzeView(vibration: vibration, logs: logs, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        vibration.stop()
        window = nil
    }
}
