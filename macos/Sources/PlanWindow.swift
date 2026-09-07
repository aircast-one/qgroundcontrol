import AppKit
import SwiftUI
import UniformTypeIdentifiers

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

struct TerrainProfileSheet: View {
    let profile: TerrainProfile

    var body: some View {
        GlassPanel {
            VStack(alignment: .leading, spacing: Overlay.step) {
                HStack {
                    Text("Terrain").font(.callout.weight(.semibold))
                    if profile.hasCollision {
                        Text("Mission is below terrain")
                            .font(.caption.weight(.semibold))
                            .foregroundColor(Overlay.vehicle)
                    } else if profile.unknownTerrain > 0 {
                        Text("\(profile.unknownTerrain) point\(profile.unknownTerrain == 1 ? "" : "s") without terrain data")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                    Text(String(format: "%.0f m", profile.totalDistance))
                        .font(.caption.monospacedDigit())
                        .foregroundColor(.secondary)
                }

                plot.frame(height: 90)

                HStack {
                    Text(String(format: "%.0f m", profile.minAltitude))
                    Spacer()
                    Text(String(format: "%.0f m", profile.maxAltitude))
                }
                .font(.caption2.monospacedDigit())
                .foregroundColor(.secondary)
            }
            .padding(Overlay.gutter)
            .frame(width: 460)
        }
    }

    private var plot: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height

            ZStack {
                if profile.groundKnown {
                    ground(width: width, height: height)
                        .fill(Overlay.launch.opacity(0.25))
                    ground(width: width, height: height)
                        .stroke(Overlay.launch, lineWidth: 1.5)
                }

                route(width: width, height: height)
                    .stroke(Overlay.mission, style: StrokeStyle(lineWidth: 2, lineJoin: .round))

                ForEach(Array(profile.points.enumerated()), id: \.offset) { _, point in
                    if point.collision {
                        Circle()
                            .fill(Overlay.vehicle)
                            .frame(width: 7, height: 7)
                            .position(x: profile.x(point, width: width),
                                      y: profile.y(point.missionAltitude, height: height))
                    }
                }
            }
        }
    }

    private func route(width: Double, height: Double) -> Path {
        Path { path in
            profile.points.enumerated().forEach { index, point in
                let location = CGPoint(x: profile.x(point, width: width),
                                       y: profile.y(point.missionAltitude, height: height))
                index == 0 ? path.move(to: location) : path.addLine(to: location)
            }
        }
    }

    private func ground(width: Double, height: Double) -> Path {
        let known = profile.points.filter { $0.terrainAltitude != nil }
        return Path { path in
            known.enumerated().forEach { index, point in
                let location = CGPoint(x: profile.x(point, width: width),
                                       y: profile.y(point.terrainAltitude ?? 0, height: height))
                index == 0 ? path.move(to: location) : path.addLine(to: location)
            }
            guard let last = known.last, let first = known.first else { return }
            path.addLine(to: CGPoint(x: profile.x(last, width: width), y: height))
            path.addLine(to: CGPoint(x: profile.x(first, width: width), y: height))
            path.closeSubpath()
        }
    }
}

struct PlanInspector: View {
    @Binding var showTerrain: Bool
    @ObservedObject var mission: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore
    @ObservedObject var selection: PageSelection

    static let pages = ["Mission", "Fence", "Rally"]

    var body: some View {
        GlassPanel {
            VStack(alignment: .leading, spacing: Overlay.gutter) {
                summary

                if let arming = mission.arming, selection.page == "Mission" {
                    Text(arming.placementHint)
                        .font(.callout)
                        .foregroundColor(Overlay.mission)
                }

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
        let rows = rowCount + (selection.page == "Mission" ? mission.selectedFacts.count : 0)
        return min(CGFloat(rows) * Overlay.rowMinHeight + Overlay.unit * 1.5, 460)
    }

    private var summary: some View {
        HStack(spacing: Overlay.step) {
            Text(mission.planName)
                .foregroundColor(.primary)
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
        default:
            missionItems
            details
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
                        showSeparator: item.index > 0,
                        current: item.isCurrent,
                        leading: {
                            Seal(label: "\(item.sequence)",
                                 colour: item.isLaunch ? Overlay.launch : Overlay.mission)
                        },
                        trailing: {
                            HStack(spacing: Overlay.step) {
                                if item.specifiesAltitude {
                                    AltitudeField(metres: item.altitude,
                                                  commit: { mission.setAltitude(of: item, metres: $0) })
                                } else {
                                    Text(item.altitudeText)
                                        .font(.body.monospacedDigit())
                                        .foregroundColor(Overlay.value)
                                }
                                if item.isCurrent && item.canChangeCommand && !mission.commands.isEmpty {
                                    Menu {
                                        ForEach(mission.commands) { command in
                                            Button(command.name) {
                                                mission.setCommand(of: item, to: command.command)
                                            }
                                        }
                                    } label: {
                                        Image(systemName: "chevron.up.chevron.down")
                                    }
                                    .menuStyle(.borderlessButton)
                                    .menuIndicator(.hidden)
                                    .frame(width: 22)
                                    .help("Change what this item does")
                                }
                                if item.isCurrent && item.canRemove {
                                    Button {
                                        mission.remove(item)
                                    } label: {
                                        Image(systemName: "trash")
                                    }
                                    .buttonStyle(.borderless)
                                    .foregroundColor(.red)
                                    .help("Remove this item from the plan")
                                }
                            }
                        })
                        .contentShape(Rectangle())
                        .onTapGesture { mission.select(item) }
                }
            }
        }
    }

    @ViewBuilder private var details: some View {
        if !mission.selectedFacts.isEmpty {
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: "Details")
                GroupCard {
                    ForEach(Array(mission.selectedFacts.enumerated()), id: \.element.id) { index, fact in
                        GroupRow(title: fact.name,
                                 showSeparator: index > 0,
                                 trailing: {
                                     if fact.options.isEmpty {
                                         ValueField(value: fact.value, units: fact.units) {
                                             mission.setFact(fact, to: $0)
                                         }
                                     } else {
                                         Picker("", selection: Binding(
                                             get: { fact.value },
                                             set: { mission.setFact(fact, to: $0) })
                                         ) {
                                             ForEach(fact.options, id: \.self) { Text($0).tag($0) }
                                         }
                                         .labelsHidden()
                                         .frame(maxWidth: 150)
                                     }
                                 })
                    }
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
            Menu {
                Button("Open\u{2026}", action: openPlan)
                Button("Save As\u{2026}", action: savePlanAs)
                Divider()
                Button("Clear", action: mission.removeAll)
            } label: {
                Image(systemName: "folder")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .frame(width: 26)
            .help("Open, save or clear this plan")

            if mission.arming == nil {
                Menu {
                    ForEach(MissionItemKind.allCases) { kind in
                        Button {
                            mission.arming = kind
                        } label: {
                            Label(kind.title, systemImage: kind.symbol)
                        }
                    }
                } label: {
                    Image(systemName: "plus")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .frame(width: 26)
                .help("Add an item by clicking the map")
                .disabled(mission.syncing || !mission.connected)
            } else {
                Button {
                    mission.arming = nil
                } label: {
                    Image(systemName: "xmark")
                }
                .help("Stop adding")
            }

            Button {
                mission.undo()
            } label: {
                Image(systemName: "arrow.uturn.backward")
            }
            .help("Undo the last change to this plan")
            .disabled(!mission.canUndo || mission.syncing)

            Button {
                mission.redo()
            } label: {
                Image(systemName: "arrow.uturn.forward")
            }
            .help("Redo the change that was undone")
            .disabled(!mission.canRedo || mission.syncing)

            Button {
                showTerrain.toggle()
            } label: {
                Image(systemName: "chart.xyaxis.line")
            }
            .help(showTerrain ? "Hide the terrain profile" : "Show the terrain profile")

            Button(action: reload) {
                Image(systemName: "arrow.down.to.line")
            }
            .help("Read the plan from the vehicle")
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

    private func openPlan() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = PlanView.planTypes
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let file = panel.url else { return }
        mission.load(from: file)
        fenceRally.reload()
    }

    private func savePlanAs() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = PlanView.planTypes
        panel.nameFieldStringValue = "\(mission.planName).plan"
        guard panel.runModal() == .OK, let file = panel.url else { return }
        mission.save(to: file)
    }
}

struct PlanView: View {
    @AppStorage("plan.showTerrain") private var showTerrain = true

    static let mapPadding = NSEdgeInsets(top: 56, left: 24, bottom: 40, right: 372)
    static let planTypes = [UTType(filenameExtension: "plan")].compactMap { $0 }

    @ObservedObject var mission: MissionStore
    @ObservedObject var fenceRally: FenceRallyStore
    @ObservedObject var selection: PageSelection

    var body: some View {
        ZStack(alignment: .topLeading) {
            MissionMap(owner: "plan", items: mission.items, vehicle: mission.vehiclePosition,
                       shapes: fenceRally.shapes, rallyPoints: fenceRally.rallyPoints,
                       padding: PlanView.mapPadding,
                       select: mission.select(sequence:),
                       adding: mission.arming != nil,
                       add: mission.addWaypoint(latitude:longitude:),
                       move: mission.move(sequence:latitude:longitude:))
                .ignoresSafeArea()

            HStack {
                Spacer()
                VStack {
                    PlanInspector(showTerrain: $showTerrain, mission: mission,
                                  fenceRally: fenceRally, selection: selection)
                    Spacer(minLength: 0)
                }
            }
            .padding(Overlay.unit)

            if showTerrain, mission.terrain.usable {
                VStack {
                    Spacer()
                    HStack {
                        TerrainProfileSheet(profile: mission.terrain)
                        Spacer()
                    }
                }
                .padding(Overlay.unit)
            }
        }
        .frame(minWidth: 860, minHeight: 620)
        .onAppear {
            mission.reload()
            fenceRally.reload()
            mission.startEditing()
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
        mission.stopEditing()
        window = nil
    }
}
