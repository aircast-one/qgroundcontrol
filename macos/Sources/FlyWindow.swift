import AppKit
import SwiftUI

struct FlyPanel: View {
    @ObservedObject var fly: FlyStore

    var body: some View {
        GlassPanel {
            VStack(alignment: .leading, spacing: Overlay.gutter) {
                header

                GroupCard {
                    GroupRow(title: "Altitude", value: FlyTelemetry.metres(fly.telemetry.altitude),
                             showSeparator: false)
                    GroupRow(title: "Ground speed", value: FlyTelemetry.speed(fly.telemetry.groundSpeed))
                    GroupRow(title: "Climb", value: FlyTelemetry.speed(fly.telemetry.climbRate))
                    GroupRow(title: "Heading", value: FlyTelemetry.degrees(fly.telemetry.heading))
                }

                GroupCard {
                    GroupRow(title: "Battery", value: fly.telemetry.batteryText, showSeparator: false,
                             leading: { dot(fly.telemetry.batteryLevel) })
                    GroupRow(title: "GPS", value: fly.telemetry.gpsText,
                             leading: { dot(fly.telemetry.gpsLevel) })
                }
            }
            .padding(Overlay.gutter)
            .frame(width: 300)
        }
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

struct FlyView: View {
    @ObservedObject var fly: FlyStore
    @ObservedObject var mission: MissionStore

    var body: some View {
        ZStack(alignment: .topTrailing) {
            MissionMap(owner: "fly", items: mission.items, vehicle: fly.position,
                       shapes: [], rallyPoints: [],
                       padding: NSEdgeInsets(top: 56, left: 24, bottom: 40, right: 352),
                       select: { _ in }, adding: false, add: { _, _ in }, move: { _, _, _ in })
                .ignoresSafeArea()

            FlyPanel(fly: fly)
                .padding(Overlay.unit)
        }
        .frame(minWidth: 820, minHeight: 600)
        .onAppear {
            mission.reload()
            fly.start()
        }
        .onDisappear(perform: fly.stop)
    }
}

final class FlyWindow: NSObject, NSWindowDelegate {
    static let shared = FlyWindow()

    private let fly = FlyStore()
    private let mission = MissionStore()
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(fly)
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
        window.contentView = NSHostingView(rootView: FlyView(fly: fly, mission: mission))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        fly.stop()
        window = nil
    }
}
