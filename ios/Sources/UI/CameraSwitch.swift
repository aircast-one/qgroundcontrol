import SwiftUI

private let SWITCH_SCRIM_ALPHA = 0.55
private let SWITCH_BORDER_ALPHA = 0.4
private let SWITCH_DOT: CGFloat = 8
private let SWITCH_BORDER: CGFloat = 1
private let SEGMENT_PAD_HORIZONTAL: CGFloat = 10
private let SEGMENT_PAD_VERTICAL: CGFloat = 6
private let SEGMENT_GAP: CGFloat = 6
private let SEGMENT_LABEL_MAX: CGFloat = 72
private let SPLIT_BUTTON_SIZE: CGFloat = 40
private let SPLIT_GLYPH: CGFloat = 17
private let CHIP_HEIGHT: CGFloat = 32
private let CHIP_CHECK_SIZE: CGFloat = 12
struct CameraSwitchState: Equatable {
    let shown: CameraEntry
    let cameras: [CameraEntry]
}

func cameraSwitchState(_ reading: CamerasReading?) -> CameraSwitchState? {
    let cameras = (reading?.cameras ?? []).filter { $0.active || $0.problem == nil }
    guard let shown = cameras.first(where: \.active) ?? cameras.first, cameras.count > 1 else { return nil }
    return CameraSwitchState(shown: shown, cameras: cameras)
}

func pipCamera(_ reading: CamerasReading?, _ streamOn: Bool) -> CameraEntry? {
    guard streamOn, let pip = reading?.pip, pip.enabled, let slot = pip.slot else { return nil }
    return reading?.cameras.first { $0.slot == slot }
}

func splitToggleTarget(_ reading: CamerasReading?, _ room: Bool, _ streamOn: Bool) -> Bool? {
    guard let pip = reading?.pip, streamOn, room, pip.slot != nil else { return nil }
    return !pip.enabled
}

@propertyWrapper
private struct StreamOn: DynamicProperty {
    @QgcPath(VIDEO_VIEW) private var videoJson

    var wrappedValue: Bool { videoReading(videoJson)?.streamEnabled == true }
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

func showCamera(_ slot: Int, _ onRefused: @escaping @MainActor () -> Void = {}) {
    offMainInOrder { if !VideoCommands.setActiveSource(slot) { onMain(onRefused) } }
}

struct CameraSwitch: View {
    let splitRoom: Bool
    @QgcPath(CAMERAS_VIEW) private var json
    @StreamOn private var streamOn
    @FlyIsPortrait private var portrait

    var body: some View {
        let reading = camerasReading(json)
        let switchState = cameraSwitchState(reading)
        let splitTo = splitToggleTarget(reading, splitRoom, streamOn)
        if switchState != nil || splitTo != nil {
            if portrait {
                HStack(spacing: Space.s2) { parts(switchState, splitTo) }
            } else {
                VStack(spacing: Space.s2) { parts(switchState, splitTo) }
            }
        }
    }

    @ViewBuilder private func parts(_ switchState: CameraSwitchState?, _ splitTo: Bool?) -> some View {
        if let switchState {
            CameraSegments(state: switchState, across: portrait)
        }
        if let next = splitTo {
            SplitButton(shown: !next) { offMainInOrder { VideoCommands.setPictureInPicture(next) } }
        }
    }
}

private struct CameraSegments: View {
    let state: CameraSwitchState
    let across: Bool
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        let white = theme.aircast.outdoorForeground
        Group {
            if across {
                HStack(spacing: 0) { segments }
            } else {
                ViewThatFits(in: .vertical) {
                    VStack(spacing: 0) { segments }
                    ScrollView(.vertical, showsIndicators: false) { VStack(spacing: 0) { segments } }
                        .fixedSize(horizontal: true, vertical: false)
                }
            }
        }
        .background(osdBackdrop(Color.black.opacity(SWITCH_SCRIM_ALPHA), flyOsd), in: Capsule())
        .overlay(Capsule().stroke(white.opacity(SWITCH_BORDER_ALPHA), lineWidth: SWITCH_BORDER))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Cameras, showing \(state.shown.title)")
    }

    private var segments: some View {
        ForEach(state.cameras) { camera in
            CameraSegment(camera: camera, selected: camera.slot == state.shown.slot)
        }
    }
}

private struct CameraSegment: View {
    let camera: CameraEntry
    let selected: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        let white = theme.aircast.outdoorForeground
        Button { if !selected { showCamera(camera.slot) } } label: {
            HStack(spacing: SEGMENT_GAP) {
                CameraStatusDot(status: camera.status)
                CappedWidth(limit: SEGMENT_LABEL_MAX) {
                    Text(camera.short).font(.labelLarge).lineLimit(1).truncationMode(.tail)
                }
            }
            .padding(.horizontal, SEGMENT_PAD_HORIZONTAL)
            .padding(.vertical, SEGMENT_PAD_VERTICAL)
            .foregroundStyle(selected ? Color.black : white)
            .background(selected ? white : Color.clear, in: Capsule())
            .frame(minHeight: MINIMUM_TOUCH_TARGET)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(camera.title)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }
}

private struct SplitButton: View {
    let shown: Bool
    let onToggle: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        let white = theme.aircast.outdoorForeground
        Button(action: onToggle) {
            Image(.splitView)
                .font(.system(size: SPLIT_GLYPH))
                .foregroundStyle(shown ? theme.colors.onPrimary : white)
                .frame(width: SPLIT_BUTTON_SIZE, height: SPLIT_BUTTON_SIZE)
                .background(shown ? theme.colors.primary : osdBackdrop(Color.black.opacity(SWITCH_SCRIM_ALPHA), flyOsd), in: Circle())
                .overlay(Circle().stroke(shown ? Color.clear : white.opacity(SWITCH_BORDER_ALPHA), lineWidth: SWITCH_BORDER))
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Show two cameras side by side")
        .accessibilityValue(shown ? "On" : "Off")
        .accessibilityAddTraits(.isToggle)
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

@propertyWrapper
struct ShownSecondCamera: DynamicProperty {
    @QgcPath(CAMERAS_VIEW) private var json
    @StreamOn private var streamOn

    init() {}

    var wrappedValue: CameraEntry? { pipCamera(camerasReading(json), streamOn) }
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
