import SwiftUI

private let REFUSAL_MS = 4000
private let ZOOM_CHIP_BORDER_ALPHA = 0.4
private let MANAGER = "vehicle.cameraManager"

let SHOW_PHOTO_VIDEO_CONTROL = "settings.flyViewSettings.showPhotoVideoControl"
private let CAMERA_SCRIM_ALPHA = 0.55
private let SHUTTER_SIZE: CGFloat = 56
private let SHUTTER_STOP_SIZE: CGFloat = 20
private let SHUTTER_DOT_SIZE: CGFloat = 26
private let SHUTTER_RING: CGFloat = 4
private let SHUTTER_STOP_CORNER: CGFloat = 4
private let DISABLED_SHUTTER_ALPHA = 0.38
private let CAMERA_TARGET: CGFloat = 48
private let CAMERA_RAIL_MAX_WIDTH: CGFloat = 120
private let REC_DOT_SIZE: CGFloat = 8
private let REC_BLINK_MS = 600
let RECORD_RED = Color(hex: 0xE53935)

private let CAMERA_CONTROLS_GRACE_MS = 300

@MainActor
private func afterGrace(_ work: @escaping @MainActor () -> Void) {
    Task { @MainActor in
        try? await Task.sleep(for: .milliseconds(CAMERA_CONTROLS_GRACE_MS))
        work()
    }
}

@Observable
final class CameraControlsState {
    var details = false
    var refused: String?
    var options = false
    var gimbalRefused: String?
    @ObservationIgnored private var layers = 0
    @ObservationIgnored private var rcLayers = 0

    @MainActor func layerShown() { layers += 1 }

    @MainActor func layerGone() {
        layers -= 1
        afterGrace { [self] in
            guard layers == 0 else { return }
            details = false
            refused = nil
        }
    }

    @MainActor func rcShown() { rcLayers += 1 }

    @MainActor func rcGone() {
        rcLayers -= 1
        afterGrace { [self] in
            guard rcLayers == 0 else { return }
            cameraRcControls.release()
            options = false
            gimbalRefused = nil
        }
    }
}

@MainActor
private func press(_ shutter: CameraShutter, _ refused: Binding<String?>) {
    let action = shutter.action
    Task { @MainActor in refused.wrappedValue = await offMain { action.flatMap { Qgc.refusalOf($0) } } }
}

private struct RefusalExpiry: ViewModifier {
    @Binding var refused: String?

    func body(content: Content) -> some View {
        content.task(id: refused) {
            guard refused != nil else { return }
            try? await Task.sleep(for: .milliseconds(REFUSAL_MS))
            if !Task.isCancelled { refused = nil }
        }
    }
}

struct CameraControlLayer: View {
    var shutters: Bool = true
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @Environment(FlyScreenState.self) private var flyScreen
    @HasVehicle private var hasVehicle
    @FlyIsPortrait private var portrait
    @QgcBool(settingControl(SHOW_PHOTO_VIDEO_CONTROL)) private var shown
    @QgcPath(CAMERA_VIEW) private var cameraJson

    private var state: CameraControlsState { flyScreen.cameraControls }
    private var refused: String? { state.refused }
    private var refusal: Binding<String?> {
        let state = state
        return Binding(get: { state.refused }, set: { state.refused = $0 })
    }

    var body: some View {
        let camera = cameraReading(cameraJson)
        let panel = camera?.panel.flatMap { hasVehicle && $0.visible ? $0 : nil }
        ZStack {
            if shown {
                if let camera, let panel {
                    rail(camera, panel)
                } else {
                    HStack(spacing: 0) {
                        RcCameraControls()
                        if shutters { StreamShutter() }
                    }
                }
            }
        }
        .background(OpenOnRequest(name: "camera") { state.details = true })
        .modifier(RefusalExpiry(refused: refusal))
        .onAppear { state.layerShown() }
        .onDisappear { state.layerGone() }
    }

    private func rail(_ camera: CameraReading, _ panel: CameraPanel) -> some View {
        HStack(spacing: 0) {
            RcCameraControls()
            ShutterCentredRail(centre: shutters && !panel.shutters.isEmpty ? SHUTTER_SIZE : 0) {
                above(camera)
            } shutter: {
                if shutters {
                    ForEach(Array(panel.shutters.enumerated()), id: \.offset) { _, shutter in
                        ShutterButton(caption: shutterCaption(panel, shutter), shutter: shutter) { press(shutter, refusal) }
                    }
                }
            } below: {
                below(camera, panel)
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 8)
        }
        .foregroundStyle(theme.aircast.outdoorForeground)
        .background(portrait ? osdBackdrop(Color.black.opacity(CAMERA_SCRIM_ALPHA), flyOsd) : Color.clear, in: RoundedRectangle(cornerRadius: Corner.extraLarge))
        .background { detailsSheet(camera, panel) }
    }

    @ViewBuilder private func detailsSheet(_ camera: CameraReading, _ panel: CameraPanel) -> some View {
        if state.details {
            AircastSheet(onDismissRequest: { state.details = false }) {
                CameraDetailsSheet(
                    camera: camera,
                    thermal: thermalReading(cameraJson),
                    tracking: trackingReading(cameraJson),
                    destructive: destructiveActions(cameraJson),
                    storage: [panel.freeText, panel.batteryText].compactMap { $0 },
                    current: camera.selected ?? 0,
                    onSelect: { index in
                        offMain { Qgc.set("\(MANAGER).currentCamera", index) }
                        state.details = false
                    },
                    onDismiss: { state.details = false }
                )
            }
        }
    }

    @ViewBuilder private func above(_ camera: CameraReading) -> some View {
        Button { state.details = true } label: {
            Image(.settings).font(.system(size: 20)).frame(width: CAMERA_TARGET, height: CAMERA_TARGET).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Camera settings")
        if let label = zoomText(camera) {
            Button { state.details = true } label: {
                Text(label)
                    .font(.labelLarge)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(osdBackdrop(Color.black.opacity(CAMERA_SCRIM_ALPHA), flyOsd), in: Capsule())
                    .overlay(Capsule().stroke(theme.aircast.outdoorForeground.opacity(ZOOM_CHIP_BORDER_ALPHA), lineWidth: 1))
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Zoom \(label)")
        }
    }

    @ViewBuilder private func below(_ camera: CameraReading, _ panel: CameraPanel) -> some View {
        if shutters {
            Text(panel.inPhotoMode ? "PHOTO" : "VIDEO").font(.labelSmall).osdShadow()
        }
        if camera.hasModes {
            let toVideo = panel.inPhotoMode
            Button {
                guard modeTapSwitches(camera, toVideo) else { return }
                let mode = toVideo ? "video" : "photo"
                let state = state
                Task { state.refused = await offMain { Qgc.refusalOf(CAMERA_SET_MODE, mode) } }
            } label: {
                Image(toVideo ? .videocam : .photoCamera).font(.system(size: 22)).osdShadow().frame(width: CAMERA_TARGET, height: CAMERA_TARGET).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(!(toVideo ? panel.selectVideoEnabled : panel.selectPhotoEnabled))
            .accessibilityLabel(toVideo ? "Switch to video" : "Switch to photo")
        }
        if let plan = lapsePlan(camera) {
            Text(plan).font(.labelSmall)
        }
        if let refused {
            Text(refused).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.horizontal, 4)
        }
    }
}

struct CameraShutters: View {
    @Environment(\.theme) private var theme
    @HasVehicle private var hasVehicle
    @QgcBool(settingControl(SHOW_PHOTO_VIDEO_CONTROL)) private var shown
    @QgcPath(CAMERA_VIEW) private var cameraJson
    @State private var refused: String?

    var body: some View {
        let panel = cameraReading(cameraJson)?.panel.flatMap { hasVehicle && $0.visible ? $0 : nil }
        ZStack {
            if shown {
                if let panel {
                    VStack(spacing: 4) {
                        ForEach(Array(panel.shutters.enumerated()), id: \.offset) { _, shutter in
                            ShutterButton(caption: shutterCaption(panel, shutter), shutter: shutter) { press(shutter, $refused) }
                        }
                        if let refused {
                            Text(refused).font(.bodySmall).foregroundStyle(theme.colors.error)
                        }
                    }
                } else {
                    StreamShutter()
                }
            }
        }
        .modifier(RefusalExpiry(refused: $refused))
    }
}

private let RAIL_GAP: CGFloat = 8

private struct ShutterCentredRail<Above: View, Shutter: View, Below: View>: View {
    let centre: CGFloat
    let above: Above
    let shutter: Shutter
    let below: Below

    init(centre: CGFloat, @ViewBuilder above: () -> Above, @ViewBuilder shutter: () -> Shutter, @ViewBuilder below: () -> Below) {
        self.centre = centre
        self.above = above()
        self.shutter = shutter()
        self.below = below()
    }

    var body: some View {
        CentredRailLayout(centre: centre, widest: CAMERA_RAIL_MAX_WIDTH) {
            VStack(spacing: RAIL_GAP) { above }
            VStack(spacing: RAIL_GAP) { shutter }
            VStack(spacing: RAIL_GAP) { below }
        }
    }
}

private struct CentredRailLayout: Layout {
    let centre: CGFloat
    let widest: CGFloat

    private func measured(_ subviews: Subviews, _ width: CGFloat?) -> [CGSize] {
        let room = ProposedViewSize(width: min(width ?? widest, widest), height: nil)
        return subviews.prefix(3).map { $0.sizeThatFits(room) }
    }

    private func reach(_ sizes: [CGSize]) -> CGFloat {
        let half = centre / 2
        let up = sizes[0].height > 0 ? RAIL_GAP + sizes[0].height : 0
        let down = sizes[2].height > 0 ? RAIL_GAP + sizes[2].height : 0
        return max(half + up, sizes[1].height - half + down)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let sizes = measured(subviews, proposal.width)
        guard sizes.count == 3 else { return .zero }
        return CGSize(width: sizes.map(\.width).max() ?? 0, height: reach(sizes) * 2)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let sizes = measured(subviews, bounds.width)
        guard sizes.count == 3 else { return }
        let midTop = bounds.minY + reach(sizes) - centre / 2
        let tops = [midTop - RAIL_GAP - sizes[0].height, midTop, midTop + sizes[1].height + RAIL_GAP]
        zip(subviews.prefix(3), zip(sizes, tops)).forEach { subview, placed in
            subview.place(at: CGPoint(x: bounds.minX + (bounds.width - placed.0.width) / 2, y: placed.1), proposal: ProposedViewSize(placed.0))
        }
    }
}

let VIDEO_RECORDING_STATE = "video.recording"

let STREAM_NOT_RECORDABLE = "Can't record this camera"
private let RECORD_CLOCK_TICK_MS = 1000

func recordClock(_ seconds: Int) -> String {
    String(format: "%02d:%02d:%02d", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

func streamShutter(_ recording: Bool, _ recordable: Bool, _ elapsedSeconds: Int?) -> CameraShutter {
    CameraShutter(
        label: recording ? "Stop recording" : "Start recording",
        recording: recording,
        enabled: recordable || recording,
        action: nil,
        video: true,
        readout: elapsedSeconds.map(recordClock) ?? "",
        readoutActive: recording && elapsedSeconds != nil
    )
}

private struct StreamShutter: View {
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcBool(VIDEO_RECORDING_STATE) private var recording
    @State private var elapsed: Int?

    var body: some View {
        let recordable = !(videoJson?.has("deviceCamera") ?? false)
        ZStack {
            if videoReading(videoJson)?.decoding == true {
                VStack(spacing: 0) {
                    ShutterButton(caption: nil, shutter: streamShutter(recording, recordable, elapsed)) {
                        let next = !recording
                        offMainInOrder { VideoCommands.setRecording(next) }
                    }
                    if !recordable {
                        Text(STREAM_NOT_RECORDABLE).font(.labelSmall).osdShadow()
                    }
                }
            }
        }
        .task(id: recording) {
            let started = Date()
            elapsed = recording ? 0 : nil
            while recording && !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(RECORD_CLOCK_TICK_MS))
                if !Task.isCancelled { elapsed = Int(Date().timeIntervalSince(started)) }
            }
        }
    }
}

private struct ShutterButton: View {
    let caption: String?
    let shutter: CameraShutter
    let onPress: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let white = theme.aircast.outdoorForeground
        VStack(spacing: 2) {
            Button(action: onPress) {
                ZStack {
                    if shutter.video {
                        Circle().stroke(white, lineWidth: SHUTTER_RING).padding(SHUTTER_RING / 2)
                        if shutter.recording {
                            RoundedRectangle(cornerRadius: SHUTTER_STOP_CORNER).fill(RECORD_RED).frame(width: SHUTTER_STOP_SIZE, height: SHUTTER_STOP_SIZE)
                        } else {
                            Circle().fill(RECORD_RED).frame(width: SHUTTER_DOT_SIZE, height: SHUTTER_DOT_SIZE)
                        }
                    } else {
                        Circle().fill(white)
                    }
                }
                .frame(width: SHUTTER_SIZE, height: SHUTTER_SIZE)
                .contentShape(Circle())
            }
            .buttonStyle(.plain)
            .disabled(!shutter.enabled)
            .opacity(shutter.enabled ? 1 : DISABLED_SHUTTER_ALPHA)
            .accessibilityLabel(shutter.label)
            if let caption {
                Text(caption).font(.labelSmall)
            }
            if let readout = shutterReadout(shutter) {
                HStack(spacing: 4) {
                    if shutter.video { RecordingDot() }
                    Text(readout).font(.labelMedium).monospacedDigit()
                }
            }
        }
    }
}

private struct RecordingDot: View {
    @State private var dim = false

    var body: some View {
        Circle()
            .fill(RECORD_RED)
            .frame(width: REC_DOT_SIZE, height: REC_DOT_SIZE)
            .opacity(dim ? 0 : 1)
            .onAppear {
                withAnimation(.easeInOut(duration: Double(REC_BLINK_MS) / 1000).repeatForever(autoreverses: true)) { dim = true }
            }
    }
}

private struct TrackingToggle: View {
    let reading: TrackingReading
    @Environment(\.theme) private var theme

    var body: some View {
        let requested = reading.requested
        Button(trackingToggleLabel(reading)) {
            offMain {
                if requested {
                    Qgc.set(CAMERA_TRACKING_ARMED, false)
                    Qgc.invoke(CAMERA_STOP_TRACKING)
                } else {
                    Qgc.set(CAMERA_TRACKING_ARMED, true)
                }
            }
        }
        .font(.labelLarge)
        .lineLimit(1)
        .buttonStyle(.borderless)
        .foregroundStyle(theme.aircast.outdoorForeground)
    }
}

private struct SheetRadioRow: View {
    let label: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: 16) {
                Image(systemName: selected ? "largecircle.fill.circle" : "circle")
                    .font(.system(size: 20))
                    .foregroundStyle(selected ? theme.colors.primary : theme.colors.onSurfaceVariant)
                Text(label).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                Spacer(minLength: 0)
            }
            .frame(minHeight: 56)
            .padding(.horizontal, 16)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private final class ConflatedWrites: @unchecked Sendable {
    private let path: String
    private let lock = NSLock()
    private var pending: Double?
    private var draining = false

    init(path: String) {
        self.path = path
    }

    func send(_ value: Double) {
        let start = lock.withLock {
            pending = value
            defer { draining = true }
            return !draining
        }
        if start { offMain { self.drain() } }
    }

    private func drain() {
        while let next = take() { Qgc.set(path, next) }
    }

    private func take() -> Double? {
        lock.withLock {
            let next = pending
            pending = nil
            if next == nil { draining = false }
            return next
        }
    }
}

private struct CameraDetailsSheet: View {
    let camera: CameraReading
    let thermal: ThermalReading?
    let tracking: TrackingReading?
    let destructive: [DestructiveAction]
    let storage: [String]
    let current: Int
    let onSelect: (Int) -> Void
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme
    @State private var dragging: Double?
    @State private var zoomDragged = false
    @State private var zoomWrites = ConflatedWrites(path: CAMERA_ZOOM)
    @State private var interval = PHOTO_LAPSE_MIN_S
    @State private var typed = 0.0
    @State private var facts: [(Fact, String)] = []
    @State private var confirming: DestructiveAction?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                Text(camera.title.ifBlank("Camera")).font(.titleLarge).padding(.horizontal, 16)
                ForEach(Array(cameraDetails(camera).enumerated()), id: \.offset) { _, detail in
                    HStack {
                        Text(detail.0).font(.bodyMedium)
                        Spacer()
                        Text(detail.1).font(.bodyMedium)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 4)
                }
                ForEach(Array(storage.enumerated()), id: \.offset) { _, line in
                    Text(line).font(.bodyMedium).padding(.horizontal, 16).padding(.vertical, 4)
                }
                if camera.labels.count > 1 {
                    SectionHeader(text: "Cameras")
                    ForEach(Array(camera.labels.enumerated()), id: \.offset) { index, label in
                        SheetRadioRow(label: label, selected: index == current) { onSelect(index) }
                    }
                }
                if camera.streamLabels.count > 1 {
                    SectionHeader(text: "Video stream")
                    ForEach(Array(camera.streamLabels.enumerated()), id: \.offset) { index, label in
                        SheetRadioRow(label: label, selected: index == camera.currentStream) { offMain { Qgc.set(CAMERA_CURRENT_STREAM, index) } }
                    }
                }
                if camera.hasZoom { zoom }
                if camera.capturesPhotos { photoMode }
                if let tracking { TrackingToggle(reading: tracking).padding(.horizontal, 12) }
                if let thermal { thermalSection(thermal) }
                if camera.hasVideoStream {
                    ForEach(Array(facts.enumerated()), id: \.offset) { _, entry in
                        FactRow(fact: entry.0, title: sentenceCase(entry.1))
                    }
                }
                CameraDefinitionSettings()
                ForEach(destructive) { action in destructiveRow(action) }
                Spacer().frame(height: 24)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .task(id: camera.hasVideoStream) {
            guard camera.hasVideoStream else { return }
            facts = await offMain {
                CAMERA_SHEET_VIDEO_SETTINGS.compactMap { entry in
                    factFromControl(Qgc.get("view.control(\(entry.key))")).map { ($0, entry.value) }
                }
            }
        }
        .alert(confirming?.title ?? "", isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }), presenting: confirming) { action in
            Button("Yes", role: .destructive) { run(action) }
            Button("No", role: .cancel) {}
        } message: { action in
            Text(action.prompt)
        }
    }

    private var zoom: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: "Zoom")
            Slider(
                value: Binding(get: { dragging ?? camera.zoomLevel }, set: { level in
                    dragging = level
                    zoomWrites.send(level)
                }),
                in: ZOOM_LOWEST...ZOOM_HIGHEST,
                onEditingChanged: { zoomDragged = $0 }
            )
            .padding(.horizontal, 20)
        }
        .onChange(of: camera.zoomLevel) { if !zoomDragged { dragging = nil } }
    }

    private var photoMode: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: "Photo mode")
            HStack(spacing: 8) {
                ForEach(Array([("Single", false), ("Time lapse", true)].enumerated()), id: \.offset) { index, choice in
                    CameraChip(label: choice.0, selected: camera.timelapse == choice.1) { offMain { Qgc.set(CAMERA_PHOTO_MODE, index) } }
                }
            }
            .padding(.horizontal, 16)
            if camera.timelapse {
                Text("Photo Interval (seconds)  \(Int(interval))")
                    .font(.titleSmall)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 8)
                Slider(value: $interval, in: PHOTO_LAPSE_MIN_S...PHOTO_LAPSE_MAX_S, step: 1, onEditingChanged: { editing in
                    guard !editing else { return }
                    let seconds = Double(Int(interval))
                    offMain { Qgc.set(CAMERA_PHOTO_LAPSE, seconds) }
                })
                .padding(.horizontal, 20)
            }
        }
        .onChange(of: camera.lapseSeconds, initial: true) {
            interval = min(max(camera.lapseSeconds ?? 1, PHOTO_LAPSE_MIN_S), PHOTO_LAPSE_MAX_S)
        }
    }

    private func thermalSection(_ thermal: ThermalReading) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: "Thermal view mode")
            ForEach(THERMAL_MODES, id: \.self) { token in
                SheetRadioRow(label: thermalModeLabel(token), selected: token == thermal.mode) {
                    let index = THERMAL_MODES.firstIndex(of: token) ?? 0
                    offMain { Qgc.set(CAMERA_THERMAL_MODE, index) }
                }
            }
            if thermalOpacityIsOffered(thermal) {
                SectionHeader(text: "Blend opacity")
                Slider(value: $typed, in: 0...100, onEditingChanged: { editing in
                    guard !editing else { return }
                    let opacity = typed
                    offMain { Qgc.set(CAMERA_THERMAL_OPACITY, opacity) }
                })
                .padding(.horizontal, 20)
            }
        }
        .onChange(of: thermal.opacity, initial: true) { typed = thermal.opacity ?? 0 }
    }

    private func destructiveRow(_ action: DestructiveAction) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(sentenceCase(action.label)).font(.bodyLarge)
                Spacer()
                Button(action.button) { confirming = action }
                    .buttonStyle(.borderless)
                    .foregroundStyle(theme.colors.error)
                    .disabled(!action.ready)
            }
            .padding(.leading, 16)
            .padding(.trailing, 12)
            .frame(minHeight: 48)
            if let reason = destructiveReasonFor(action) {
                Text(reason).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, 20)
            }
        }
    }

    private func run(_ action: DestructiveAction) {
        confirming = nil
        guard let path = destructiveInvokePath(action.id) else { return }
        let formats = action.id == "formatStorage"
        offMain {
            if formats { Qgc.invoke(path, 1) } else { Qgc.invoke(path) }
        }
    }
}
