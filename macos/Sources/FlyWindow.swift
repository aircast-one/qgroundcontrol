import AppKit
import SwiftUI

struct FlyPanel: View {
    @ObservedObject var fly: FlyStore

    var body: some View {
        GlassPanel {
            VStack(alignment: .leading, spacing: Overlay.gutter) {
                header

                GroupCard {
                    GroupRow(title: "Battery", value: fly.telemetry.batteryText, showSeparator: false,
                             leading: { dot(fly.telemetry.batteryLevel) })
                    GroupRow(title: "GPS", value: fly.telemetry.gpsText,
                             leading: { dot(fly.telemetry.gpsLevel) })
                }

                if fly.connected {
                    messages
                }
            }
            .padding(Overlay.gutter)
            .frame(width: 300)
        }
        .sheet(isPresented: $fly.showingChecklist) { checklistSheet }
    }

    private var header: some View {
        HStack(spacing: Overlay.step) {
            VStack(alignment: .leading, spacing: 2) {
                Text(fly.connected ? fly.telemetry.mode : "No vehicle")
                    .font(.title3.weight(.semibold))
                Text(fly.connected ? fly.telemetry.stateText : "Connect a vehicle to fly")
                    .font(.callout).foregroundColor(.secondary)
            }
            Spacer(minLength: 0)
            Button("Checklist") { fly.showingChecklist = true }
                .controlSize(.small)
                .disabled(!fly.connected)
            if fly.telemetry.armed {
                Text("ARMED")
                    .font(.caption.weight(.bold))
                    .foregroundColor(Overlay.vehicle)
                    .padding(.horizontal, Overlay.step)
                    .padding(.vertical, 3)
                    .background(Overlay.vehicle.opacity(0.16))
                    .clipShape(Capsule())
            }
        }
        .padding(.horizontal, 2)
    }

    private var checklistSheet: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.75) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Pre-flight checklist").font(.title3.weight(.semibold))
                    Text(Preflight.progress(fly.checklist, ticked: fly.ticked))
                        .font(.callout).foregroundColor(.secondary)
                }
                Spacer()
                if Preflight.ready(fly.checklist, ticked: fly.ticked) {
                    StatusPill(text: "Ready", good: true)
                }
            }

            ScrollView {
                VStack(alignment: .leading, spacing: Overlay.unit * 0.75) {
                    ForEach(fly.checklist) { group in
                        VStack(alignment: .leading, spacing: 0) {
                            SectionLabel(text: group.name)
                            GroupCard {
                                ForEach(Array(group.checks.enumerated()), id: \.element.id) { row, check in
                                    checkRow(check, showSeparator: row > 0)
                                }
                            }
                        }
                    }
                }
            }
            .frame(height: 400)

            Divider()

            HStack {
                Button("Reset", action: fly.resetChecklist)
                Spacer()
                Button("Done") { fly.showingChecklist = false }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(Overlay.unit)
        .frame(width: 460)
    }

    private func checkRow(_ check: PreflightCheck, showSeparator: Bool) -> some View {
        GroupRow(title: check.name,
                 description: check.reason,
                 showSeparator: showSeparator,
                 titleLines: 1,
                 leading: {
                     Image(systemName: check.blocked
                         ? "exclamationmark.octagon.fill"
                         : (fly.ticked.contains(check.name) ? "checkmark.circle.fill" : "circle"))
                         .foregroundColor(check.blocked
                             ? Overlay.vehicle
                             : (fly.ticked.contains(check.name) ? .green : .secondary))
                 },
                 trailing: { EmptyView() })
            .contentShape(Rectangle())
            .onTapGesture { fly.toggle(check) }
            .help(check.blocked ? "Fix this before it can be checked off" : check.prompt)
    }

    private var messages: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.3) {
            SectionLabel(text: "From the vehicle")
            GroupCard {
                if fly.latestMessages.isEmpty {
                    EmptyStateRow(text: "Nothing said yet.")
                } else {
                    ForEach(Array(fly.latestMessages.enumerated()), id: \.element.id) { row, message in
                        GroupRow(title: message.text,
                                 description: message.time,
                                 showSeparator: row > 0,
                                 titleLines: 2,
                                 leading: { Circle()
                                     .fill(FlyPanel.colour(message.level))
                                     .frame(width: 7, height: 7) },
                                 trailing: { EmptyView() })
                    }
                }
            }
        }
    }

    static func colour(_ level: VehicleMessage.Level) -> Color {
        switch level {
        case .error: return Overlay.vehicle
        case .warning: return .orange
        case .normal: return .secondary
        }
    }

    private func dot(_ level: FlyTelemetry.Level) -> some View {
        Circle()
            .fill(FlyPanel.colour(level))
            .frame(width: 9, height: 9)
    }

    static func colour(_ level: FlyTelemetry.Level) -> Color {
        switch level {
        case .good: return .green
        case .warning: return .orange
        case .critical: return Overlay.vehicle
        case .unknown: return .secondary
        }
    }
}

struct InstrumentBar: View {
    @ObservedObject var instruments: InstrumentsStore

    var body: some View {
        GlassPanel {
            HStack(alignment: .top, spacing: Overlay.unit) {
                ForEach(instruments.values) { value in
                    VStack(alignment: .leading, spacing: 1) {
                        Text(value.label)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                        HStack(alignment: .firstTextBaseline, spacing: 3) {
                            Text(value.value)
                                .font(.title3.monospacedDigit())
                                .foregroundColor(value.missing ? .secondary : .primary)
                            if !value.units.isEmpty {
                                Text(value.units)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .fixedSize()
                }
            }
            .padding(.horizontal, Overlay.unit * 0.9)
            .padding(.vertical, Overlay.unit * 0.6)
        }
    }
}

struct FlyView: View {
    @ObservedObject var fly: FlyStore
    @ObservedObject var mission: MissionStore
    @ObservedObject var instruments: InstrumentsStore

    private var warningBanner: some View {
        GlassPanel {
            HStack(alignment: .top, spacing: Overlay.step) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(fly.warning.lines, id: \.self) { line in
                        Text(line)
                            .font(.callout.weight(.medium))
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
            .padding(.horizontal, Overlay.unit * 0.9)
            .padding(.vertical, Overlay.unit * 0.6)
        }
        .frame(maxWidth: 380)
    }

    var body: some View {
        ZStack(alignment: .top) {
            MissionMap(owner: "fly", items: mission.items, vehicle: fly.position,
                       shapes: [], rallyPoints: [],
                       padding: NSEdgeInsets(top: 56, left: 24, bottom: 40, right: 352),
                       select: { _ in }, adding: false, add: { _, _ in }, move: { _, _, _ in })
                .ignoresSafeArea()

            if fly.warning.showing {
                warningBanner
                    .padding(.top, Overlay.unit)
                    .transition(.opacity)
            }

            FlyPanel(fly: fly)
                .padding(Overlay.unit)
                .frame(maxWidth: .infinity, alignment: .trailing)

            if !instruments.values.isEmpty {
                InstrumentBar(instruments: instruments)
                    .padding(Overlay.unit)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
            }
        }
        .frame(minWidth: 820, minHeight: 600)
        .onAppear {
            mission.reload()
            fly.start()
            instruments.refresh()
        }
        .onDisappear {
            fly.stop()
            instruments.clear()
        }
        .onChange(of: fly.telemetry) { _ in instruments.refresh() }
    }
}

final class FlyWindow: NSObject, NSWindowDelegate {
    static let shared = FlyWindow()

    private let fly = FlyStore()
    private let mission = MissionStore()
    private let instruments = InstrumentsStore()
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(fly)
        NativeProbe.register(instruments)
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
            contentRect: NSRect(x: 0, y: 0, width: 960, height: 640),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false)
        window.title = "Fly"
        window.titlebarAppearsTransparent = true
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: FlyView(fly: fly, mission: mission, instruments: instruments))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        fly.stop()
        window = nil
    }
}
