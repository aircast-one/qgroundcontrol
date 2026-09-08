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
            Text("m").font(.caption).foregroundColor(.secondary).fixedSize()
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
    static let maximumContentHeight: CGFloat = 460

    @State private var contentHeight: CGFloat = 0
    @State private var shapeError: String?
    @State private var replacing: String?

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
                        .measuringHeight(into: $contentHeight)
                }
                .frame(height: min(max(contentHeight, Overlay.rowMinHeight),
                                   PlanInspector.maximumContentHeight))
                .overlay(alignment: .bottom) {
                    if contentHeight > PlanInspector.maximumContentHeight {
                        LinearGradient(colors: [.clear, Color(nsColor: .windowBackgroundColor)],
                                       startPoint: .top, endPoint: .bottom)
                            .frame(height: Overlay.unit * 1.5)
                            .allowsHitTesting(false)
                    }
                }

                actions
            }
            .padding(Overlay.gutter)
            .frame(width: 320)
        }
        .sheet(isPresented: Binding(get: { mission.pickingCommandFor != nil },
                                    set: { if !$0 { mission.pickingCommandFor = nil } })) {
            commandPicker
        }
        .confirmationDialog("Replace this plan?",
                            isPresented: Binding(get: { replacing != nil },
                                                 set: { if !$0 { replacing = nil } }),
                            titleVisibility: .visible) {
            Button("Replace", role: .destructive) {
                let kind = MissionItemKind(rawValue: replacing ?? "")
                replacing = nil
                shapeError = mission.createPlan(kind)
            }
            Button("Cancel", role: .cancel) { replacing = nil }
        } message: {
            Text("The \(mission.items.count) items already in this plan will be discarded.")
        }
        .alert("That file could not be used",
               isPresented: Binding(get: { shapeError != nil }, set: { if !$0 { shapeError = nil } })) {
            Button("OK") { shapeError = nil }
        } message: {
            Text(shapeError ?? "")
        }
    }

    private var commandPicker: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.75) {
            Text("Choose what this item does").font(.title3.weight(.semibold))

            Picker("Category", selection: Binding(
                get: { mission.pickerCategory },
                set: { mission.showCategory($0) })
            ) {
                ForEach(mission.commandCategories, id: \.self) { Text($0).tag($0) }
            }
            .frame(maxWidth: 320)

            if mission.commands.isEmpty {
                GroupCard {
                    EmptyStateRow(text: mission.connected
                        ? "This category has no commands this vehicle accepts."
                        : "Connect a vehicle to see the commands it accepts.")
                }
            } else {
                ScrollView {
                    GroupCard {
                        ForEach(Array(mission.commands.enumerated()), id: \.element.id) { row, command in
                            commandRow(command, showSeparator: row > 0)
                        }
                    }
                }
                .frame(height: 380)
            }

            Divider()

            HStack {
                Spacer()
                Button("Cancel") { mission.pickingCommandFor = nil }
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(Overlay.unit)
        .frame(width: 520)
    }

    private func commandRow(_ command: MissionCommand, showSeparator: Bool) -> some View {
        Button {
            chooseCommand(command)
        } label: {
            GroupRow(title: command.name,
                     description: command.summary,
                     showSeparator: showSeparator,
                     current: command.command == chosenCommand,
                     titleLines: 2, descriptionLines: 4)
        }
        .buttonStyle(.plain)
    }

    private func chooseCommand(_ command: MissionCommand) {
        guard let item = mission.items.first(where: { $0.sequence == mission.pickingCommandFor })
        else { return }
        mission.setCommand(of: item, to: command.command)
    }

    private var chosenCommand: Int {
        mission.items.first { $0.sequence == mission.pickingCommandFor }?.commandId ?? -1
    }

    private var summary: some View {
        VStack(alignment: .leading, spacing: 2) {
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

            if mission.summary.hasFlight {
                HStack(spacing: Overlay.step) {
                    Label(mission.summary.distanceText, systemImage: "arrow.triangle.turn.up.right.diamond")
                        .help("Distance flown")
                    Label(mission.summary.durationText, systemImage: "clock")
                        .help("How long the mission takes")
                    Spacer(minLength: 0)
                    Text("\(mission.summary.telemetryText) from launch")
                        .help("The furthest the vehicle gets from where it took off")
                }
                .font(.caption.monospacedDigit())
                .foregroundColor(.secondary)
                .labelStyle(.titleAndIcon)
            }
        }
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
                                if item.isCurrent && item.canChangeCommand {
                                    Button {
                                        mission.pickCommand(for: item)
                                    } label: {
                                        Image(systemName: "chevron.up.chevron.down")
                                    }
                                    .buttonStyle(.borderless)
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
        if showsMissionSettings {
            missionCard
        }

        if mission.surveyStats.describes {
            surveyCard
        }

        ForEach([ItemFact.cameraGroup, ItemFact.itemGroup], id: \.self) { group in
            let facts = mission.selectedFacts.filter { $0.group == group }
            if !facts.isEmpty || showsAltitudeMode(in: group) {
                factCard(group, facts)
            }
        }

        if showsMissionSettings, mission.vehicle.showsAnything {
            vehicleCard
        }
    }

    private func showsAltitudeMode(in group: String) -> Bool {
        group == ItemFact.itemGroup && AltitudeMode.isChoice(mission.itemAltitudeMode)
    }

    private var showsMissionSettings: Bool {
        mission.items.first(where: \.isCurrent)?.sequence == 0
            && AltitudeMode.isMissionChoice(mission.globalAltitudeMode)
    }

    private var missionCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionLabel(text: "Mission")
            GroupCard {
                GroupRow(title: "Altitude mode", showSeparator: false, trailing: {
                    Picker("", selection: Binding(
                        get: { mission.globalAltitudeMode },
                        set: { mission.setGlobalAltitudeMode($0) })
                    ) {
                        ForEach(AltitudeMode.missionChoices) { Text($0.title).tag($0.raw) }
                    }
                    .labelsHidden()
                    .frame(maxWidth: 170)
                })

                GroupRow(title: "Altitude for new items", trailing: {
                    ValueField(value: mission.defaultAltitude, units: "m") {
                        mission.setDefaultAltitude($0)
                    }
                })
            }
        }
    }

    private var surveyCard: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
            SectionLabel(text: "Survey")
            GroupCard {
                GroupRow(title: "Photos", value: mission.surveyStats.shotsText, showSeparator: false)
                GroupRow(title: "Between shots", value: mission.surveyStats.intervalText)
                GroupRow(title: "Area covered", value: mission.surveyStats.areaText)
                GroupRow(title: "Each photo covers", value: mission.surveyStats.footprintText)
            }
            if !mission.surveyStats.warning.isEmpty {
                Label(mission.surveyStats.warning, systemImage: "exclamationmark.triangle.fill")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
            }
        }
    }

    private var vehicleCard: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.35) {
            SectionLabel(text: "Vehicle")
            GroupCard {
                if !mission.vehicle.firmware.isEmpty {
                    GroupRow(title: "Firmware", value: mission.vehicle.firmware, showSeparator: false)
                }
                if !mission.vehicle.type.isEmpty {
                    GroupRow(title: "Type", value: mission.vehicle.type,
                             showSeparator: !mission.vehicle.firmware.isEmpty)
                }
                if mission.vehicle.showsCruiseSpeed {
                    GroupRow(title: "Cruise speed", showSeparator: mission.vehicle.isDescribed, trailing: {
                        ValueField(value: mission.cruiseSpeed, units: "m/s") {
                            mission.setCruiseSpeed($0)
                        }
                    })
                }
                if mission.vehicle.showsHoverSpeed {
                    GroupRow(title: "Hover speed",
                             showSeparator: mission.vehicle.isDescribed || mission.vehicle.showsCruiseSpeed,
                             trailing: {
                        ValueField(value: mission.hoverSpeed, units: "m/s") {
                            mission.setHoverSpeed($0)
                        }
                    })
                }
            }
            if mission.vehicle.showsCruiseSpeed || mission.vehicle.showsHoverSpeed {
                Text("Speeds only estimate how long the mission takes. They do not change how fast the vehicle flies.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
            }
        }
    }

    @ViewBuilder private func factCard(_ group: String, _ facts: [ItemFact]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionLabel(text: group)
            GroupCard {
                    if group == ItemFact.cameraGroup, mission.camera.canChooseBrand {
                        GroupRow(title: "Camera", showSeparator: false, trailing: {
                            Picker("", selection: Binding(
                                get: { mission.camera.brand },
                                set: { mission.setCamera(brand: $0) })
                            ) {
                                ForEach(mission.camera.brands, id: \.self) { Text($0).tag($0) }
                            }
                            .labelsHidden()
                            .frame(maxWidth: 170)
                        })

                        if mission.camera.canChooseModel {
                            GroupRow(title: "Model", trailing: {
                                Picker("", selection: Binding(
                                    get: { mission.camera.model },
                                    set: { mission.setCamera(model: $0) })
                                ) {
                                    ForEach(mission.camera.models, id: \.self) { Text($0).tag($0) }
                                }
                                .labelsHidden()
                                .frame(maxWidth: 170)
                            })
                        }

                        if AltitudeMode.isChoice(mission.distanceMode) {
                            GroupRow(title: "Altitude mode", trailing: {
                                Picker("", selection: Binding(
                                    get: { mission.distanceMode },
                                    set: { mission.setDistanceMode($0) })
                                ) {
                                    ForEach(AltitudeMode.choices) { Text($0.title).tag($0.raw) }
                                }
                                .labelsHidden()
                                .frame(maxWidth: 170)
                            })
                        }
                    }

                    if showsAltitudeMode(in: group) {
                        GroupRow(title: "Altitude mode", showSeparator: false, trailing: {
                            Picker("", selection: Binding(
                                get: { mission.itemAltitudeMode },
                                set: { mission.setItemAltitudeMode($0) })
                            ) {
                                ForEach(AltitudeMode.choices) { Text($0.title).tag($0.raw) }
                            }
                            .labelsHidden()
                            .frame(maxWidth: 170)
                        })
                    }

                    ForEach(Array(facts.enumerated()), id: \.element.id) { index, fact in
                        GroupRow(title: fact.title,
                                 showSeparator: index > 0 || group == ItemFact.cameraGroup
                                     || showsAltitudeMode(in: group),
                                 titleLines: 2,
                                 trailing: {
                                     if fact.isBool {
                                         Toggle("", isOn: Binding(
                                             get: { fact.value == "true" },
                                             set: { mission.setFact(fact, to: $0 ? "true" : "false") })
                                         )
                                         .labelsHidden()
                                         .disabled(fact.readOnly)
                                     } else if fact.options.isEmpty {
                                         ValueField(value: fact.value, units: fact.units) {
                                             mission.setFact(fact, to: $0)
                                         }
                                         .disabled(fact.readOnly)
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
                Button("Export KML\u{2026}", action: exportKml)
                    .disabled(mission.items.count < 2)
                Divider()
                Menu("New Plan") {
                    Button("Empty") { startPlan(nil) }
                    ForEach(MissionItemKind.shapeImportable) { kind in
                        Button(kind.title) { startPlan(kind) }
                    }
                }
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
                    Divider()
                    ForEach(MissionItemKind.shapeImportable) { kind in
                        Button {
                            importShape(kind)
                        } label: {
                            Label("\(kind.title) from KML or SHP\u{2026}", systemImage: kind.symbol)
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

    private func startPlan(_ kind: MissionItemKind?) {
        guard mission.items.count > 1 else {
            shapeError = mission.createPlan(kind)
            return
        }
        replacing = kind?.rawValue ?? ""
    }

    private func exportKml() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = PlanView.kmlTypes
        panel.nameFieldStringValue = "\(mission.planName).kml"
        guard panel.runModal() == .OK, let file = panel.url else { return }
        mission.exportKml(to: file)
    }

    private func importShape(_ kind: MissionItemKind) {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = PlanView.shapeTypes
        panel.allowsMultipleSelection = false
        panel.message = "Choose a KML or shape file holding the \(kind.shapeNoun) to \(kind.title.lowercased())."
        guard panel.runModal() == .OK, let file = panel.url else { return }
        shapeError = mission.importShape(kind, from: file)
    }
}

struct PlanView: View {
    @AppStorage("plan.showTerrain") private var showTerrain = true

    static let mapPadding = NSEdgeInsets(top: 56, left: 24, bottom: 40, right: 372)
    static let planTypes = [UTType(filenameExtension: "plan")].compactMap { $0 }
    static let kmlTypes = [UTType(filenameExtension: "kml")].compactMap { $0 }
    static let shapeTypes = ["kml", "shp"].compactMap { UTType(filenameExtension: $0) }

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
                       move: mission.move(sequence:latitude:longitude:),
                       surveys: mission.surveyAreas,
                       corridors: mission.corridorPaths)
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
