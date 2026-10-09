import SwiftUI

private let SWITCH_SCRIM_ALPHA = 0.55
private let SWITCH_BORDER_ALPHA = 0.4
private let SWITCH_DOUBLE_TAP_MS: Int64 = 300
private let SWITCH_SETTLE_MS: Int64 = 2000
private let SWITCH_GLYPH: CGFloat = 15
private let SWITCH_DOT: CGFloat = 8
private let SWITCH_BORDER: CGFloat = 1
private let SWITCH_PAD_HORIZONTAL: CGFloat = 10
private let SWITCH_PAD_VERTICAL: CGFloat = 6
private let SWITCH_GAP: CGFloat = 6
private let SWITCH_LABEL_MAX: CGFloat = 72
private let PIP_BUTTON_SIZE: CGFloat = 40
private let PIP_GLYPH: CGFloat = 17
private let PIP_BORDER: CGFloat = 2
private let PIP_INSET: CGFloat = 4
private let PIP_LABEL_PAD: CGFloat = 6
private let CHIP_HEIGHT: CGFloat = 32
private let CHIP_CHECK_SIZE: CGFloat = 12
struct CameraSwitchState: Equatable {
    let shown: CameraEntry
    let cameras: [CameraEntry]
    let toggleTo: Int?
}

struct SwitchTap: Equatable {
    let from: Int
    let atMs: Int64
}

func switchTapAllowed(_ last: SwitchTap?, _ shown: Int, _ nowMs: Int64) -> Bool {
    guard let last else { return true }
    let since = nowMs - last.atMs
    return since >= SWITCH_SETTLE_MS || (last.from != shown && since >= SWITCH_DOUBLE_TAP_MS)
}

func cameraSwitchState(_ reading: CamerasReading?) -> CameraSwitchState? {
    let cameras = (reading?.cameras ?? []).filter { $0.active || $0.problem == nil }
    guard let shown = cameras.first(where: \.active) ?? cameras.first, cameras.count > 1 else { return nil }
    let others = cameras.filter { $0.slot != shown.slot }
    return CameraSwitchState(shown: shown, cameras: cameras, toggleTo: others.count == 1 ? others[0].slot : nil)
}

func pipCamera(_ reading: CamerasReading?, _ streamOn: Bool) -> CameraEntry? {
    guard streamOn, let pip = reading?.pip, pip.enabled, let slot = pip.slot else { return nil }
    return reading?.cameras.first { $0.slot == slot }
}

func pipToggleTarget(_ reading: CamerasReading?, _ thumbnailRoom: Bool, _ streamOn: Bool) -> Bool? {
    guard let pip = reading?.pip, streamOn, thumbnailRoom, pip.slot != nil else { return nil }
    return !pip.enabled
}

@propertyWrapper
private struct StreamOn: DynamicProperty {
    @QgcPath(VIDEO_VIEW) private var videoJson

    var wrappedValue: Bool { videoReading(videoJson)?.streamEnabled == true }
}

enum MenuSide {
    case Start, Above
}

func cameraStatusTint(_ status: CameraStatus, _ theme: Theme) -> Color {
    switch status {
    case .Live: theme.aircast.success
    case .Connecting: theme.aircast.warning
    case .NoSignal: theme.colors.error
    case .Idle: theme.colors.outline
    }
}

struct CameraStatusDot: View {
    let status: CameraStatus
    @Environment(\.theme) private var theme

    var body: some View {
        Circle()
            .fill(cameraStatusTint(status, theme))
            .frame(width: SWITCH_DOT, height: SWITCH_DOT)
            .accessibilityLabel(status.label)
    }
}

private func showCamera(_ slot: Int, _ onRefused: @escaping @MainActor () -> Void = {}) {
    offMainInOrder { if !VideoCommands.setActiveSource(slot) { onMain(onRefused) } }
}

struct CameraSwitch: View {
    let thumbnailRoom: Bool
    @QgcPath(CAMERAS_VIEW) private var json
    @StreamOn private var streamOn
    @FlyIsPortrait private var portrait

    var body: some View {
        let reading = camerasReading(json)
        let switchState = cameraSwitchState(reading)
        let pipTo = pipToggleTarget(reading, thumbnailRoom, streamOn)
        if switchState != nil || pipTo != nil {
            if portrait {
                HStack(spacing: Space.s2) { buttons(switchState, pipTo) }
            } else {
                VStack(spacing: Space.s2) { buttons(switchState, pipTo) }
            }
        }
    }

    @ViewBuilder private func buttons(_ switchState: CameraSwitchState?, _ pipTo: Bool?) -> some View {
        if let switchState {
            CameraSwitchButton(state: switchState, side: portrait ? .Above : .Start)
        }
        if let next = pipTo {
            PipButton(shown: !next) { offMainInOrder { VideoCommands.setPictureInPicture(next) } }
        }
    }
}

private struct CameraSwitchButton: View {
    let state: CameraSwitchState
    let side: MenuSide
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @State private var open = false
    @State private var lastTap: SwitchTap?

    var body: some View {
        let white = theme.aircast.outdoorForeground
        Button(action: tapped) {
            HStack(spacing: SWITCH_GAP) {
                Image(.videocam).font(.system(size: SWITCH_GLYPH))
                CappedWidth(limit: SWITCH_LABEL_MAX) {
                    Text(state.shown.short).font(.labelLarge).lineLimit(1).truncationMode(.tail)
                }
                CameraStatusDot(status: state.shown.status)
            }
            .padding(.horizontal, SWITCH_PAD_HORIZONTAL)
            .padding(.vertical, SWITCH_PAD_VERTICAL)
            .foregroundStyle(white)
            .background(osdBackdrop(Color.black.opacity(SWITCH_SCRIM_ALPHA), flyOsd), in: Capsule())
            .overlay(Capsule().stroke(white.opacity(SWITCH_BORDER_ALPHA), lineWidth: SWITCH_BORDER))
            .frame(minHeight: MINIMUM_TOUCH_TARGET)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Switch camera, showing \(state.shown.title)")
        .popover(isPresented: $open, attachmentAnchor: .rect(.bounds), arrowEdge: side == .Start ? .trailing : .bottom) {
            CameraMenu(cameras: state.cameras) { open = false }
                .presentationCompactAdaptation(.popover)
        }
    }

    private func tapped() {
        guard let slot = state.toggleTo else { return open = true }
        let now = uptimeMillis()
        guard switchTapAllowed(lastTap, state.shown.slot, now) else { return }
        let tap = SwitchTap(from: state.shown.slot, atMs: now)
        lastTap = tap
        showCamera(slot) { if lastTap == tap { lastTap = nil } }
    }
}

struct CappedWidth: Layout {
    let limit: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        subviews.first?.sizeThatFits(capped(proposal)) ?? .zero
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, proposal: ProposedViewSize(width: bounds.width, height: bounds.height))
    }

    private func capped(_ proposal: ProposedViewSize) -> ProposedViewSize {
        ProposedViewSize(width: min(proposal.width ?? limit, limit), height: proposal.height)
    }
}

private struct CameraMenu: View {
    let cameras: [CameraEntry]
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(cameras) { camera in
                    Button {
                        onDismiss()
                        if !camera.active { showCamera(camera.slot) }
                    } label: {
                        HStack(spacing: Space.s3) {
                            CameraStatusDot(status: camera.status)
                            Text(camera.title).font(.bodyLarge).lineLimit(1).foregroundStyle(theme.colors.onSurface)
                            Spacer(minLength: Space.s4)
                            if camera.active {
                                Image(.check).foregroundStyle(theme.colors.onSurfaceVariant).accessibilityLabel("On screen")
                            }
                        }
                        .padding(.horizontal, Space.s3)
                        .frame(minHeight: 48)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.vertical, Space.s2)
        }
        .frame(minWidth: 200)
        .fixedSize(horizontal: false, vertical: true)
    }
}

private struct PipButton: View {
    let shown: Bool
    let onToggle: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        let white = theme.aircast.outdoorForeground
        Button(action: onToggle) {
            Image(.pictureInPicture)
                .font(.system(size: PIP_GLYPH))
                .foregroundStyle(shown ? theme.colors.onPrimary : white)
                .frame(width: PIP_BUTTON_SIZE, height: PIP_BUTTON_SIZE)
                .background(shown ? theme.colors.primary : osdBackdrop(Color.black.opacity(SWITCH_SCRIM_ALPHA), flyOsd), in: Circle())
                .overlay(Circle().stroke(shown ? Color.clear : white.opacity(SWITCH_BORDER_ALPHA), lineWidth: SWITCH_BORDER))
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Second camera picture-in-picture")
        .accessibilityValue(shown ? "On" : "Off")
        .accessibilityAddTraits(.isToggle)
    }
}

@propertyWrapper
struct ShownPipCamera: DynamicProperty {
    @QgcPath(CAMERAS_VIEW) private var json
    @StreamOn private var streamOn

    init() {}

    var wrappedValue: CameraEntry? { pipCamera(camerasReading(json), streamOn) }
}

struct CameraPipThumbnail<Editor: View>: View {
    let camera: CameraEntry
    let editor: Editor
    @Environment(\.theme) private var theme

    init(camera: CameraEntry, @ViewBuilder editor: () -> Editor) {
        self.camera = camera
        self.editor = editor()
    }

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: Corner.medium)
        ZStack(alignment: .topLeading) {
            VideoChannelSurface(channel: PIP_VIDEO_CHANNEL).padding(PIP_INSET)
            (camera.status == .Live ? Color.clear : theme.aircast.outdoorBackground)
                .contentShape(Rectangle())
                .onTapGesture { showCamera(camera.slot) }
                .accessibilityElement()
                .accessibilityLabel("Second camera, \(camera.title)")
                .accessibilityAddTraits(.isButton)
                .accessibilityAction(named: "Make \(camera.title) the main view") { showCamera(camera.slot) }
            HStack(spacing: Space.s1) {
                CameraStatusDot(status: camera.status)
                Text(camera.short).font(.labelSmall).foregroundStyle(theme.aircast.outdoorForeground).lineLimit(1)
            }
            .padding(PIP_LABEL_PAD)
            .osdShadow()
            .allowsHitTesting(false)
            editor
        }
        .background(theme.aircast.outdoorBackground)
        .clipShape(shape)
        .overlay(shape.stroke(theme.colors.onSurface, lineWidth: PIP_BORDER))
    }
}

extension CameraPipThumbnail where Editor == EmptyView {
    init(camera: CameraEntry) {
        self.init(camera: camera) { EmptyView() }
    }
}

func neighbourCamera(_ state: CameraSwitchState?, _ step: Int) -> CameraEntry? {
    guard let state, state.cameras.count > 1 else { return nil }
    let count = state.cameras.count
    let at = state.cameras.firstIndex(of: state.shown) ?? -1
    return state.cameras[((at + step) % count + count) % count]
}

@propertyWrapper
struct CameraSwiper: DynamicProperty {
    @QgcPath(CAMERAS_VIEW) private var json

    init() {}

    var wrappedValue: (Int) -> Bool {
        let state = cameraSwitchState(camerasReading(json))
        return { step in
            guard let next = neighbourCamera(state, step) else { return false }
            showCamera(next.slot)
            return true
        }
    }
}

@propertyWrapper
struct GimbalDrags: DynamicProperty {
    @QgcPath(GIMBAL_INDICATOR_PATH) private var view

    init() {}

    var wrappedValue: Bool { onScreenGimbal(view) != nil }
}

struct CameraChip: View {
    let label: String
    let selected: Bool
    var enabled: Bool = true
    var checkmark: Bool = true
    let action: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: Corner.small)
        Button(action: action) {
            HStack(spacing: Space.s1) {
                if selected && checkmark { Image(.check).font(.system(size: CHIP_CHECK_SIZE, weight: .semibold)) }
                Text(label).font(.labelLarge).lineLimit(1)
            }
            .padding(.horizontal, Space.s3)
            .frame(minHeight: CHIP_HEIGHT)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : Color.clear, in: shape)
            .overlay(shape.stroke(selected ? Color.clear : theme.colors.outline, lineWidth: 1))
            .contentShape(shape)
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : DISABLED_ALPHA)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
