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
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                if store.reading.available {
                    SectionCard(title: "Vibration") {
                        VStack(alignment: .leading, spacing: 12) {
                            HStack(alignment: .bottom, spacing: 28) {
                                VibrationScale()
                                VibrationBar(label: "X", value: store.reading.x)
                                VibrationBar(label: "Y", value: store.reading.y)
                                VibrationBar(label: "Z", value: store.reading.z)
                            }
                            .frame(height: 190)
                            Text(advice)
                                .font(.callout)
                                .foregroundColor(adviceColour)
                        }
                        .padding(.top, 4)
                    }

                    SectionCard(title: "Accelerometer clipping") {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Counted since the vehicle booted. Any clipping means the accelerometer saturated.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.bottom, 6)
                            ForEach(Array(store.reading.clipCounts.enumerated()), id: \.offset) { index, count in
                                if index > 0 { Divider() }
                                MetricRow(label: "Accelerometer \(index + 1)",
                                          value: String(count),
                                          units: count == 1 ? "clip" : "clips")
                            }
                        }
                    }
                } else {
                    Notice(text: "This vehicle is not reporting vibration.")
                        .frame(height: 240)
                }
            }
            .padding(20)
        }
        .onAppear(perform: store.start)
        .onDisappear(perform: store.stop)
    }

    // The numbers alone do not tell an operator whether to fly; the thresholds do.
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

struct AnalyzeView: View {
    @ObservedObject var vibration: VibrationStore
    // Optional selection: the non-optional List initialiser is macOS 13+ and the
    // deployment floor here is 12.0.
    @State private var page: String? = "Vibration"

    private let pages = ["Vibration"]

    var body: some View {
        HStack(spacing: 0) {
            List(pages, id: \.self, selection: $page) { name in
                Text(name).tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 190)
            Divider()
            VibrationView(store: vibration)
        }
        .frame(minWidth: 640, minHeight: 440)
    }
}

final class AnalyzeWindow: NSObject, NSWindowDelegate {
    static let shared = AnalyzeWindow()

    private let vibration = VibrationStore()
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(vibration)
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
        window.contentView = NSHostingView(rootView: AnalyzeView(vibration: vibration))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        vibration.stop()
        window = nil
    }
}
