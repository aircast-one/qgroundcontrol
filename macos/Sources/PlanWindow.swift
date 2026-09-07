import AppKit
import SwiftUI

struct MissionView: View {
    @ObservedObject var store: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if !store.status.isEmpty {
                Notice(text: store.status)
            } else {
                VSplitView {
                    MissionMap(owner: "mission", items: store.items, vehicle: store.vehiclePosition,
                               shapes: fenceRally.shapes, rallyPoints: fenceRally.rallyPoints)
                        .frame(minHeight: 220)
                    list
                        .frame(minHeight: 120)
                }
            }
        }
        .onAppear {
            store.reload()
            fenceRally.reload()
        }
    }

    private var header: some View {
        HStack {
            Text("\(store.items.count) item\(store.items.count == 1 ? "" : "s")")
                .foregroundColor(.secondary)
            if store.syncing {
                ProgressView().controlSize(.small)
                Text("Reading from vehicle…").font(.caption).foregroundColor(.secondary)
            }
            if store.dirty {
                Text("Unsent changes")
                    .font(.caption)
                    .padding(.horizontal, 6).padding(.vertical, 2)
                    .background(Color.orange.opacity(0.18))
                    .cornerRadius(3)
            }
            Spacer()
            Button("Read from Vehicle", action: store.downloadFromVehicle)
                .disabled(store.syncing)
            Button("Send to Vehicle", action: store.uploadToVehicle)
                .disabled(store.syncing || !store.connected || store.items.isEmpty)
        }
        .padding(10)
    }

    private var list: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(store.items) { item in
                    Divider()
                    row(item)
                }
            }
            .padding(.horizontal, 14)
        }
    }

    private func row(_ item: MissionItem) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text("\(item.sequence)")
                .font(.body.monospacedDigit())
                .foregroundColor(.secondary)
                .frame(width: 28, alignment: .trailing)

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(item.command)
                    if item.isCurrent {
                        Text("current")
                            .font(.caption2)
                            .padding(.horizontal, 5).padding(.vertical, 1)
                            .background(Color.accentColor.opacity(0.18))
                            .cornerRadius(3)
                    }
                }
                if !item.description.isEmpty {
                    Text(item.description)
                        .font(.caption).foregroundColor(.secondary)
                }
            }

            Spacer()
            Text(item.positionText)
                .font(.caption.monospacedDigit())
                .foregroundColor(item.hasPosition ? .secondary : Color.secondary.opacity(0.5))
            if item.specifiesAltitude {
                AltitudeField(metres: item.altitude, commit: { store.setAltitude(of: item, metres: $0) })
            } else {
                Text(item.altitudeText)
                    .font(.body.monospacedDigit())
                    .frame(width: 70, alignment: .trailing)
            }
        }
        .padding(.vertical, 8)
    }
}

struct AltitudeField: View {
    let metres: Double?
    let commit: (Double) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(spacing: 4) {
            TextField("", text: $draft)
                .multilineTextAlignment(.trailing)
                .font(.body.monospacedDigit())
                .frame(width: 56)
                .focused($editing)
                .onAppear { draft = AltitudeField.text(metres) }
                .onChange(of: metres) { latest in if !editing { draft = AltitudeField.text(latest) } }
                .onSubmit(send)
                .onChange(of: editing) { focused in if !focused { send() } }
            Text("m").font(.caption).foregroundColor(.secondary)
        }
    }

    private func send() {
        guard let value = Double(draft.trimmingCharacters(in: .whitespaces)), value.isFinite else {
            draft = AltitudeField.text(metres)
            return
        }
        guard value != metres else { return }
        commit(value)
    }

    private static func text(_ metres: Double?) -> String {
        guard let metres, metres.isFinite else { return "" }
        return String(format: "%.1f", metres)
    }
}

struct FenceRallyView: View {
    @ObservedObject var store: FenceRallyStore

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if !store.status.isEmpty {
                Notice(text: store.status)
            } else {
                VSplitView {
                    MissionMap(owner: "fence", items: [], vehicle: nil,
                               shapes: store.shapes, rallyPoints: store.rallyPoints)
                        .frame(minHeight: 200)
                    ScrollView {
                        VStack(alignment: .leading, spacing: 22) {
                            SectionCard(title: "Geofence") { fenceBody }
                            SectionCard(title: "Rally Points") { rallyBody }
                        }
                        .padding(16)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(minHeight: 160)
                }
            }
        }
        .onAppear(perform: store.reload)
    }

    private var header: some View {
        HStack {
            Text(summary).foregroundColor(.secondary)
            if store.syncing {
                ProgressView().controlSize(.small)
                Text("Reading from vehicle\u{2026}").font(.caption).foregroundColor(.secondary)
            }
            Spacer()
            Button("Read from Vehicle", action: store.downloadFromVehicle)
                .disabled(store.syncing)
        }
        .padding(10)
    }

    private var summary: String {
        let fence = store.shapes.count
        let rally = store.rallyPoints.count
        return "\(fence) fence shape\(fence == 1 ? "" : "s") \u{00B7} \(rally) rally point\(rally == 1 ? "" : "s")"
    }

    @ViewBuilder private var fenceBody: some View {
        if store.connected && !store.fenceSupported {
            Text("This vehicle's firmware does not support geofences.")
                .foregroundColor(.secondary)
        } else if store.shapes.isEmpty {
            Text(store.connected
                ? "No geofence is set. The vehicle will not be stopped at any boundary."
                : "No geofence in this plan.")
                .foregroundColor(.secondary)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                if let breach = store.breachReturn {
                    Divider()
                    MetricRow(label: "Breach return", value: breach.positionText,
                              units: breach.altitudeText)
                }
                ForEach(store.shapes) { shape in
                    Divider()
                    HStack(alignment: .firstTextBaseline, spacing: 12) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(shape.kindText)
                            Text(shape.detailText).font(.caption).foregroundColor(.secondary)
                        }
                        Spacer()
                        Text(shape.centreText)
                            .font(.caption.monospacedDigit()).foregroundColor(.secondary)
                    }
                    .padding(.vertical, 8)
                }
            }
        }
    }

    @ViewBuilder private var rallyBody: some View {
        if store.connected && !store.rallySupported {
            Text("This vehicle's firmware does not support rally points.")
                .foregroundColor(.secondary)
        } else if store.rallyPoints.isEmpty {
            Text(store.connected
                ? "No rally points. On a failsafe the vehicle returns to its launch point."
                : "No rally points in this plan.")
                .foregroundColor(.secondary)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(store.rallyPoints) { point in
                    Divider()
                    HStack(alignment: .firstTextBaseline, spacing: 12) {
                        Text("\(point.id + 1)")
                            .font(.body.monospacedDigit()).foregroundColor(.secondary)
                            .frame(width: 28, alignment: .trailing)
                        Text(point.positionText).font(.caption.monospacedDigit())
                        Spacer()
                        Text(point.altitudeText).font(.body.monospacedDigit())
                    }
                    .padding(.vertical, 8)
                }
            }
        }
    }
}

struct PlanView: View {
    @ObservedObject var mission: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore
    @ObservedObject var selection: PageSelection

    private let pages = ["Mission", "Fence & Rally"]

    var body: some View {
        HStack(spacing: 0) {
            List(pages, id: \.self, selection: Binding(
                get: { Optional(selection.page) },
                set: { selection.page = $0 ?? selection.page })
            ) { name in
                Text(name).tag(name)
            }
            .listStyle(.sidebar)
            .frame(width: 170)
            Divider()
            if selection.page == "Mission" {
                MissionView(store: mission, fenceRally: fenceRally)
            } else {
                FenceRallyView(store: fenceRally)
            }
        }
        .frame(minWidth: 820, minHeight: 620)
    }
}

final class PlanWindow: NSObject, NSWindowDelegate {
    static let shared = PlanWindow()

    private let mission = MissionStore()
    private let fenceRally = FenceRallyStore()
    private let selection = PageSelection(owner: "plan", pages: ["Mission", "Fence & Rally"])
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(mission)
        NativeProbe.register(fenceRally)
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
            contentRect: NSRect(x: 0, y: 0, width: 840, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = "Plan"
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(rootView: PlanView(mission: mission, fenceRally: fenceRally, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}
