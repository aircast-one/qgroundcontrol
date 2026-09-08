import AppKit
import SwiftUI

struct FlyPanel: View {
    @ObservedObject var fly: FlyStore
    @ObservedObject var video: VideoStore

    var body: some View {
        GlassPanel {
            ScrollView {
              VStack(alignment: .leading, spacing: Overlay.gutter) {
                header

                GroupCard {
                    ForEach(Array(fly.batteries.enumerated()), id: \.offset) { index, pack in
                        expandable(
                            key: "battery\(index)",
                            title: fly.batteries.count > 1 ? "Battery \(index + 1)" : "Battery",
                            value: index == 0 ? fly.telemetry.batteryText : pack.first?.value ?? "",
                            level: fly.telemetry.batteryLevel,
                            detail: pack,
                            showSeparator: index > 0)
                    }
                    if fly.batteries.isEmpty {
                        GroupRow(title: "Battery", value: fly.telemetry.batteryText,
                                 showSeparator: false,
                                 leading: { dot(fly.telemetry.batteryLevel) })
                    }
                    expandable(key: "gps", title: "GPS", value: fly.telemetry.gpsText,
                               level: fly.telemetry.gpsLevel, detail: fly.gpsDetail,
                               showSeparator: true)
                    if !fly.linkDetail.isEmpty {
                        expandable(key: "link", title: "Link",
                                   value: fly.linkDetail.first?.value ?? "",
                                   level: fly.telemetry.gpsLevel, detail: fly.linkDetail,
                                   showSeparator: true)
                    }
                }

                if video.camera.present {
                    cameraCard
                }

                if video.status.available {
                    videoCard
                }

                if fly.connected {
                    messages
                }
              }
              .padding(Overlay.gutter)
            }
            .frame(width: 300)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .sheet(isPresented: $fly.showingChecklist) { checklistSheet }
        .sheet(isPresented: $fly.showingModes) { modePicker }
    }

    private var modePicker: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.5) {
            Text("Flight mode").font(.headline)
            ScrollView {
                GroupCard {
                    ForEach(Array(FlightModes.everyday(fly.modes).enumerated()),
                            id: \.element.id) { row, mode in
                        modeRow(mode, showSeparator: row > 0)
                    }
                }
                if fly.showingAdvancedModes {
                    GroupCard {
                        ForEach(Array(FlightModes.folded(fly.modes).enumerated()),
                                id: \.element.id) { row, mode in
                            modeRow(mode, showSeparator: row > 0)
                        }
                    }
                    .padding(.top, Overlay.unit * 0.5)
                }
            }
            .frame(maxHeight: 380)

            if !FlightModes.folded(fly.modes).isEmpty {
                Button(fly.showingAdvancedModes
                    ? "Fewer modes"
                    : "More modes (\(FlightModes.folded(fly.modes).count))") {
                    fly.showingAdvancedModes.toggle()
                }
                .buttonStyle(.plain)
                .font(.callout)
                .foregroundColor(.accentColor)
            }

            Divider()

            if fly.confirmingMode.isEmpty {
                HStack {
                    Spacer()
                    Button("Cancel") { fly.showingModes = false }
                        .keyboardShortcut(.cancelAction)
                }
            } else {
                HStack {
                    Text("\(fly.confirmingMode) while flying — confirm?")
                        .font(.callout)
                        .foregroundColor(.orange)
                    Spacer()
                    Button("Back", action: fly.cancelMode)
                        .keyboardShortcut(.cancelAction)
                    Button(fly.confirmingMode, action: fly.confirmMode)
                        .buttonStyle(.borderedProminent)
                }
            }
        }
        .padding(Overlay.unit)
        .frame(width: 380)
    }

    private func modeRow(_ mode: FlightModeChoice, showSeparator: Bool) -> some View {
        Button {
            fly.request(mode)
        } label: {
            GroupRow(title: mode.name, description: mode.summary,
                     showSeparator: showSeparator, current: mode.current,
                     leading: {
                         Image(systemName: mode.symbol)
                             .font(.callout)
                             .foregroundColor(mode.current ? .accentColor : .secondary)
                             .frame(width: 20)
                     },
                     trailing: {
                         if mode.current {
                             Image(systemName: "checkmark").font(.caption.weight(.semibold))
                         }
                     })
        }
        .buttonStyle(.plain)
        .disabled(mode.current)
    }

    @ViewBuilder private func expandable(key: String, title: String, value: String,
                                         level: FlyTelemetry.Level, detail: [DetailRow],
                                         showSeparator: Bool) -> some View {
        let open = fly.expanded.contains(key)
        Button {
            fly.expanded = open ? fly.expanded.subtracting([key]) : fly.expanded.union([key])
        } label: {
            GroupRow(title: title, value: value, showSeparator: showSeparator,
                     leading: { dot(level) },
                     trailing: {
                         Image(systemName: open ? "chevron.up" : "chevron.down")
                             .font(.caption2)
                             .foregroundColor(detail.isEmpty ? .clear : Overlay.chevron)
                     })
        }
        .buttonStyle(.plain)
        .disabled(detail.isEmpty)

        if open {
            ForEach(detail) { row in
                GroupRow(title: row.label, value: row.value, showSeparator: false)
                    .padding(.leading, Overlay.unit * 1.5)
            }
        }
    }

    private var header: some View {
        HStack(spacing: Overlay.step) {
            VStack(alignment: .leading, spacing: 2) {
                if fly.modes.isEmpty {
                    Text(fly.connected ? fly.telemetry.mode : "No vehicle")
                        .font(.title3.weight(.semibold))
                } else {
                    Button { fly.showingModes = true } label: {
                        HStack(spacing: 4) {
                            Text(fly.requestedMode.isEmpty ? fly.telemetry.mode : fly.requestedMode)
                                .font(.title3.weight(.semibold))
                            if fly.requestedMode.isEmpty {
                                Image(systemName: "chevron.down").font(.caption2)
                            } else {
                                ProgressView().controlSize(.small).scaleEffect(0.6)
                            }
                        }
                    }
                    .buttonStyle(.plain)
                }
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
                    Text("\(fly.airframe.rawValue) · \(Preflight.progress(fly.checklist, ticked: fly.ticked))")
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

    private var cameraCard: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.3) {
            SectionLabel(text: "Camera")
            GroupCard {
                GroupRow(title: video.camera.title,
                         description: video.camera.stateText,
                         showSeparator: false,
                         leading: {
                             Image(systemName: video.camera.isRecording
                                 ? "record.circle.fill" : "camera.fill")
                                 .foregroundColor(video.camera.isRecording ? Overlay.vehicle : .secondary)
                         },
                         trailing: { EmptyView() })
                if video.camera.hasModes {
                    GroupRow(title: "Mode", trailing: {
                        Picker("", selection: Binding(
                            get: { video.camera.mode },
                            set: { video.setCameraMode(photo: $0 == CameraControl.photoMode) })
                        ) {
                            Text("Photo").tag(CameraControl.photoMode)
                            Text("Video").tag(CameraControl.videoMode)
                        }
                        .pickerStyle(.segmented)
                        .labelsHidden()
                        .frame(width: 140)
                    })
                }
                GroupRow(title: "Storage", value: video.camera.storageText)
            }
            if !video.camera.modeKnown {
                Text("The camera has not said which mode it is in.")
                    .font(.caption).foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, Overlay.horizontalPadding)
            }
        }
    }

    private var videoCard: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.3) {
            SectionLabel(text: "Video")
            GroupCard {
                GroupRow(title: video.status.summary, showSeparator: false, titleLines: 2,
                         leading: {
                             Circle()
                                 .fill(video.status.settled ? Color.green
                                     : (video.status.anyConnecting ? Color.orange : Color.secondary))
                                 .frame(width: 7, height: 7)
                         },
                         trailing: { EmptyView() })
                ForEach(video.status.configuredCameras) { camera in
                    GroupRow(title: camera.title,
                             description: camera.recording ? "Recording" : "",
                             value: camera.status)
                }
            }
        }
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
                ForEach(Array(instruments.values.enumerated()), id: \.element.id) { slot, value in
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
                    .contentShape(Rectangle())
                    .contextMenu {
                        Button("Change reading\u{2026}") { instruments.edit(slot: slot) }
                        Button("Add reading") { instruments.addSlot() }
                            .disabled(!instruments.canAdd)
                        Button("Remove this reading") { instruments.removeSlot(slot) }
                            .disabled(!instruments.canRemove)
                        Divider()
                        Button("Reset to defaults") { instruments.resetSlots() }
                    }
                }
            }
            .padding(.horizontal, Overlay.unit * 0.9)
            .padding(.vertical, Overlay.unit * 0.6)
        }
        .sheet(isPresented: $instruments.showingEditor) { editor }
    }

    private var editor: some View {
        VStack(alignment: .leading, spacing: Overlay.unit * 0.75) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Change reading").font(.title3.weight(.semibold))
                Text(instruments.editingLabel)
                    .font(.callout).foregroundColor(.secondary)
            }

            HStack(alignment: .top, spacing: Overlay.unit) {
                GroupCard {
                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(Array(instruments.groups.enumerated()), id: \.element.id) { row, group in
                                Button {
                                    instruments.chosenGroup = group.group
                                } label: {
                                    GroupRow(title: group.title,
                                             value: "\(group.facts.count)",
                                             showSeparator: row > 0,
                                             current: group.group == instruments.chosenGroup)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
                .frame(width: 210, height: 320)

                GroupCard {
                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(Array(instruments.chosenFacts.enumerated()), id: \.element.id) { row, fact in
                                Button {
                                    instruments.assign(group: instruments.chosenGroup, factName: fact.name)
                                } label: {
                                    GroupRow(title: fact.label,
                                             description: fact.name,
                                             showSeparator: row > 0)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity)
                .frame(height: 320)
            }

            Divider()

            HStack {
                Spacer()
                Button("Cancel", action: instruments.cancelEdit)
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(Overlay.unit)
        .frame(width: 560)
    }
}

struct SlideToConfirm: View {
    let title: String
    let destructive: Bool
    let confirm: () -> Void

    @State private var offset: CGFloat = 0

    private static let knob: CGFloat = 38
    private static let track: CGFloat = 260

    var body: some View {
        ZStack(alignment: .leading) {
            Capsule()
                .fill(Color.primary.opacity(0.12))
            Text(title)
                .font(.callout.weight(.medium))
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity)
            Circle()
                .fill(destructive ? Overlay.vehicle : Color.accentColor)
                .overlay(Image(systemName: "chevron.right.2").foregroundColor(.white))
                .frame(width: SlideToConfirm.knob, height: SlideToConfirm.knob)
                .offset(x: offset)
                .gesture(
                    DragGesture()
                        .onChanged { drag in
                            offset = min(max(0, drag.translation.width), SlideToConfirm.limit)
                        }
                        .onEnded { _ in
                            if offset >= SlideToConfirm.limit { confirm() }
                            offset = 0
                        })
        }
        .frame(width: SlideToConfirm.track, height: SlideToConfirm.knob)
    }

    static var limit: CGFloat { track - knob }
}

struct GuidedConfirm: View {
    @ObservedObject var guided: GuidedStore

    var body: some View {
        if let action = guided.pending {
            GlassPanel {
                VStack(spacing: Overlay.unit * 0.6) {
                    Text(action.title)
                        .font(.title3.weight(.semibold))
                        .foregroundColor(action.destructive ? Overlay.vehicle : .primary)
                    Text(action.prompt)
                        .font(.callout)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .fixedSize(horizontal: false, vertical: true)
                    if let range = guided.range {
                        valuePicker(range)
                    }
                    SlideToConfirm(title: "Slide to \(action.title.lowercased())",
                                   destructive: action.destructive,
                                   confirm: guided.confirm)
                    Button("Cancel", action: guided.cancel)
                        .keyboardShortcut(.cancelAction)
                }
                .padding(Overlay.unit)
                .frame(width: 320)
            }
        }
    }

    private func valuePicker(_ range: GuidedValue) -> some View {
        VStack(spacing: 2) {
            HStack {
                Text(range.label).font(.caption).foregroundColor(.secondary)
                Spacer()
                Text(range.text(guided.chosen))
                    .font(.title3.monospacedDigit().weight(.medium))
            }
            Slider(value: $guided.chosen, in: range.minimum...range.maximum)
            HStack {
                Text(range.text(range.minimum))
                Spacer()
                Text(range.text(range.maximum))
            }
            .font(.caption2)
            .foregroundColor(.secondary)
        }
    }
}

struct GuidedStrip: View {
    @ObservedObject var guided: GuidedStore

    static func explain(_ offer: GuidedAction.Offer, _ action: GuidedAction) -> String {
        if case .blocked(let reason) = offer { return reason }
        return action.prompt
    }

    var body: some View {
        GlassPanel {
            VStack(spacing: Overlay.step) {
                ForEach(guided.actions) { action in
                    let offer = guided.offer(action)
                    let blocked = offer != .ready
                    Button {
                        guided.ask(action)
                    } label: {
                        VStack(spacing: 2) {
                            Image(systemName: action.symbol)
                                .font(.system(size: 18))
                            Text(action.title)
                                .font(.caption2)
                                .lineLimit(1)
                        }
                        .foregroundColor(action.destructive ? Overlay.vehicle : .primary)
                        .opacity(blocked ? 0.35 : 1)
                        .frame(width: 68)
                        .padding(.vertical, 5)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(blocked)
                    .help(GuidedStrip.explain(offer, action))
                }
            }
            .padding(Overlay.step)
        }
    }
}

struct FlyView: View {
    @ObservedObject var fly: FlyStore
    @ObservedObject var mission: MissionStore
    @ObservedObject var instruments: InstrumentsStore
    @ObservedObject var guided: GuidedStore
    @ObservedObject var video: VideoStore

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

            FlyPanel(fly: fly, video: video)
                .padding(Overlay.unit)
                .frame(maxWidth: .infinity, alignment: .trailing)

            if !instruments.values.isEmpty {
                InstrumentBar(instruments: instruments)
                    .padding(Overlay.unit)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
            }

            if !guided.actions.isEmpty {
                GuidedStrip(guided: guided)
                    .padding(Overlay.unit)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            }

            if video.nativeFrames > 0 {
                NativeVideoView()
                    .frame(width: 320, height: 180)
                    .clipShape(RoundedRectangle(cornerRadius: Overlay.panelRadius))
                    .overlay(RoundedRectangle(cornerRadius: Overlay.panelRadius)
                        .strokeBorder(Overlay.border, lineWidth: 1))
                    .shadow(color: .black.opacity(0.3), radius: 10, y: 3)
                    .padding(.bottom, Overlay.unit * 5)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
            }

            GuidedConfirm(guided: guided)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
        }
        .frame(minWidth: 820, minHeight: 600)
        .onAppear {
            mission.reload()
            fly.start()
            instruments.refresh()
            guided.refresh(prearmClear: !fly.warning.showing)
            video.useNativeRendering()
            video.refresh()
        }
        .onDisappear {
            fly.stop()
            instruments.clear()
            video.clear()
        }
        .onChange(of: fly.telemetry) { _ in
            instruments.refresh()
            guided.refresh(prearmClear: !fly.warning.showing)
            video.refresh()
        }
    }
}

final class FlyWindow: NSObject, NSWindowDelegate {
    static let shared = FlyWindow()

    private let fly = FlyStore()
    private let mission = MissionStore()
    private let instruments = InstrumentsStore()
    private let guided = GuidedStore()
    private let video = VideoStore.shared
    private var window: NSWindow?

    override init() {
        super.init()
        NativeProbe.register(fly)
        NativeProbe.register(instruments)
        NativeProbe.register(guided)
        NativeProbe.register(video)
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
        window.contentView = NSHostingView(rootView: FlyView(fly: fly, mission: mission, instruments: instruments, guided: guided, video: video))
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    func windowWillClose(_ notification: Notification) {
        fly.stop()
        window = nil
    }
}
