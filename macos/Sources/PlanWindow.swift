import AppKit
import SwiftUI

struct AltitudeField: View {
    let metres: Double?
    let commit: (Double) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(spacing: 2) {
            TextField("", text: $draft)
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .font(.body.monospacedDigit())
                .frame(width: 46)
                .focused($editing)
                .onAppear { draft = AltitudeField.text(metres) }
                .onChange(of: metres) { latest in if !editing { draft = AltitudeField.text(latest) } }
                .onSubmit(send)
                .onChange(of: editing) { focused in if !focused { send() } }
            Text("m").font(.caption).foregroundColor(.secondary)
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(Color.primary.opacity(editing ? 0.10 : 0.06))
        .cornerRadius(6)
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
        return String(format: "%.0f", metres)
    }
}

struct PlanInspector: View {
    @ObservedObject var mission: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore
    @ObservedObject var selection: PageSelection

    static let pages = ["Mission", "Fence", "Rally"]

    var body: some View {
        GlassPanel {
            VStack(alignment: .leading, spacing: Overlay.gutter) {
                summary

                Picker("", selection: Binding(
                    get: { selection.page },
                    set: { selection.page = $0 })
                ) {
                    ForEach(PlanInspector.pages, id: \.self) { Text($0).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()

                ScrollView {
                    VStack(alignment: .leading, spacing: Overlay.gutter) { page }
                }
                .frame(height: contentHeight)

                actions
            }
            .padding(Overlay.gutter)
            .frame(width: 320)
        }
    }

    private var rowCount: Int {
        switch selection.page {
        case "Fence": return max(fenceRally.shapes.count, 1)
        case "Rally": return max(fenceRally.rallyPoints.count, 1)
        default: return max(mission.items.count, 1)
        }
    }

    private var contentHeight: CGFloat {
        min(CGFloat(rowCount) * Overlay.rowMinHeight + Overlay.unit * 0.5, 420)
    }

    private var summary: some View {
        HStack(spacing: Overlay.step) {
            Text("\(mission.items.count) item\(mission.items.count == 1 ? "" : "s")")
            if !fenceRally.shapes.isEmpty {
                dot(Overlay.fence)
                Text("\(fenceRally.shapes.count) fence")
            }
            if !fenceRally.rallyPoints.isEmpty {
                dot(Overlay.rally)
                Text("\(fenceRally.rallyPoints.count) rally")
            }
            Spacer(minLength: 0)
            if mission.dirty {
                Text("Unsent")
                    .font(.caption.weight(.semibold))
                    .foregroundColor(Overlay.fence)
            }
        }
        .font(.callout)
        .foregroundColor(.secondary)
        .padding(.horizontal, 2)
    }

    private func dot(_ colour: Color) -> some View {
        Circle().fill(colour).frame(width: 7, height: 7)
    }

    @ViewBuilder private var page: some View {
        switch selection.page {
        case "Fence": fence
        case "Rally": rally
        default: missionItems
        }
    }

    private var missionItems: some View {
        GroupCard {
            if mission.items.isEmpty {
                EmptyStateRow(text: mission.status.isEmpty ? "This plan has no items." : mission.status)
            } else {
                ForEach(mission.items) { item in
                    GroupRow(
                        title: item.command,
                        description: "",
                        showSeparator: item.index > 0,
                        current: item.isCurrent,
                        leading: {
                            Seal(label: "\(item.sequence)",
                                 colour: item.isLaunch ? Overlay.launch : Overlay.mission)
                        },
                        trailing: {
                            if item.specifiesAltitude {
                                AltitudeField(metres: item.altitude,
                                              commit: { mission.setAltitude(of: item, metres: $0) })
                            } else {
                                Text(item.altitudeText)
                                    .font(.body.monospacedDigit())
                                    .foregroundColor(Overlay.value)
                            }
                        })
                }
            }
        }
    }

    private var fence: some View {
        GroupCard {
            if fenceRally.shapes.isEmpty {
                EmptyStateRow(text: fenceRally.connected && !fenceRally.fenceSupported
                    ? "This vehicle's firmware does not support geofences."
                    : fenceRally.connected
                        ? "No geofence. Nothing will stop the vehicle leaving the area."
                        : "No geofence in this plan.")
            } else {
                ForEach(fenceRally.shapes) { shape in
                    GroupRow(
                        title: shape.kindText,
                        description: shape.detailText,
                        showSeparator: shape.id > 0,
                        leading: {
                            Seal(label: shape.inclusion ? "IN" : "OUT",
                                 colour: Overlay.fence, rounded: true)
                        })
                }
            }
        }
    }

    private var rally: some View {
        GroupCard {
            if fenceRally.rallyPoints.isEmpty {
                EmptyStateRow(text: fenceRally.connected && !fenceRally.rallySupported
                    ? "This vehicle's firmware does not support rally points."
                    : fenceRally.connected
                        ? "No rally points. On a failsafe the vehicle returns to launch."
                        : "No rally points in this plan.")
            } else {
                ForEach(fenceRally.rallyPoints) { point in
                    GroupRow(
                        title: "Rally \(point.id + 1)",
                        value: point.altitudeText,
                        showSeparator: point.id > 0,
                        leading: { Seal(label: "\(point.id + 1)", colour: Overlay.rally) })
                }
            }
        }
    }

    private var actions: some View {
        HStack(spacing: Overlay.step) {
            Button("Download", action: reload)
                .disabled(mission.syncing || !mission.connected)
            Spacer()
            if mission.syncing {
                ProgressView().controlSize(.small)
            }
            Button("Upload", action: mission.uploadToVehicle)
                .buttonStyle(.borderedProminent)
                .disabled(mission.syncing || !mission.connected || mission.items.isEmpty)
        }
    }

    private func reload() {
        mission.downloadFromVehicle()
        fenceRally.reload()
    }
}

struct PlanView: View {
    static let mapPadding = NSEdgeInsets(top: 56, left: 24, bottom: 40, right: 372)

    @ObservedObject var mission: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore
    @ObservedObject var selection: PageSelection

    var body: some View {
        ZStack(alignment: .topLeading) {
            MissionMap(owner: "plan", items: mission.items, vehicle: mission.vehiclePosition,
                       shapes: fenceRally.shapes, rallyPoints: fenceRally.rallyPoints,
                       padding: PlanView.mapPadding)
                .ignoresSafeArea()

            HStack {
                Spacer()
                VStack {
                    PlanInspector(mission: mission, fenceRally: fenceRally, selection: selection)
                    Spacer(minLength: 0)
                }
            }
            .padding(Overlay.unit)
        }
        .frame(minWidth: 860, minHeight: 620)
        .onAppear {
            mission.reload()
            fenceRally.reload()
        }
    }
}

final class PlanWindow: NSObject, NSWindowDelegate {
    static let shared = PlanWindow()

    private let mission = MissionStore()
    private let fenceRally = FenceRallyStore()
    private let selection = PageSelection(owner: "plan", pages: PlanInspector.pages)
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
            contentRect: NSRect(x: 0, y: 0, width: 980, height: 660),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false)
        window.title = "Plan"
        window.titlebarAppearsTransparent = true
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.contentView = NSHostingView(
            rootView: PlanView(mission: mission, fenceRally: fenceRally, selection: selection))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        window = nil
    }
}
