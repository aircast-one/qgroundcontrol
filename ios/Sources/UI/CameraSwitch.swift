import Combine
import SwiftUI

private let SWITCH_SCRIM_ALPHA = 0.55
private let SWITCH_BORDER_ALPHA = 0.4
private let SWITCH_DOT: CGFloat = 8
private let SWITCH_BORDER: CGFloat = 1
private let CHIP_PAD_HORIZONTAL: CGFloat = 12
private let CHIP_PAD_VERTICAL: CGFloat = 8
private let CHIP_GAP: CGFloat = 6
private let CHIP_LABEL_MAX: CGFloat = 96
private let CHIP_GLYPH: CGFloat = 14
private let CHIP_ARROW: CGFloat = 12
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

func chipCamera(_ reading: CamerasReading?) -> CameraEntry? { reading.flatMap { $0.cameras.first(where: \.active) ?? $0.cameras.first } }

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

struct CameraPick: Equatable, Sendable {
    let slot: Int
    let on: JSON?
}

func pickedReading(_ json: JSON?, _ pick: CameraPick?) -> CamerasReading? {
    guard let reading = camerasReading(json) else { return nil }
    guard let pick, pick.on == json else { return reading }
    return CamerasReading(
        readable: reading.readable,
        reason: reading.reason,
        pip: reading.pip,
        cameras: reading.cameras.map { camera in
            CameraEntry(
                slot: camera.slot, stored: camera.stored, title: camera.title, short: camera.short, name: camera.name, source: camera.source, url: camera.url,
                tilt: camera.tilt, problem: camera.problem, fromDrone: camera.fromDrone, active: camera.slot == pick.slot, status: camera.status
            )
        },
        kinds: reading.kinds
    )
}

@MainActor
@Observable
final class CameraPicks {
    static let shared = CameraPicks()
    private(set) var pick: CameraPick?
    @ObservationIgnored private var answered: AnyCancellable?

    func make(_ slot: Int) -> CameraPick {
        let list = QgcWatch.subject(CAMERAS_VIEW)
        let made = CameraPick(slot: slot, on: list.json)
        pick = made
        answered = list.$json.dropFirst().first().sink { [weak self] _ in self?.drop(made) }
        return made
    }

    func drop(_ made: CameraPick) {
        guard pick == made else { return }
        pick = nil
        answered = nil
    }
}

@propertyWrapper
struct ShownCameras: DynamicProperty {
    @QgcPath(CAMERAS_VIEW) private var json

    init() {}

    var wrappedValue: CamerasReading? { pickedReading(json, MainActor.assumeIsolated { CameraPicks.shared.pick }) }
}

func showCamera(_ slot: Int, _ onRefused: @escaping @MainActor () -> Void = {}) {
    let made = MainActor.assumeIsolated { CameraPicks.shared.make(slot) }
    offMainInOrder {
        if !VideoCommands.setActiveSource(slot) {
            onMain {
                CameraPicks.shared.drop(made)
                onRefused()
            }
        }
    }
}

struct CameraSwitch: View {
    let splitRoom: Bool
    @ShownCameras private var reading
    @StreamOn private var streamOn
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        let switchState = cameraSwitchState(reading)
        let splitTo = splitToggleTarget(reading, splitRoom, streamOn)
        if let shown = switchState?.shown ?? chipCamera(reading), switchState != nil || splitTo != nil {
            Menu {
                if let cameras = switchState?.cameras {
                    Picker("Camera", selection: Binding(get: { shown.slot }, set: { slot in if slot != shown.slot { showCamera(slot) } })) {
                        ForEach(cameras) { camera in Text(camera.title).tag(camera.slot) }
                    }
                    .pickerStyle(.inline)
                }
                if let next = splitTo {
                    Toggle(isOn: Binding(get: { !next }, set: { _ in offMainInOrder { VideoCommands.setPictureInPicture(next) } })) {
                        Label("Split view", systemImage: Icon.splitView.rawValue)
                    }
                }
            } label: {
                chip(shown, splitOn: splitTo == false)
            }
            .menuStyle(.button)
            .buttonStyle(.plain)
            .accessibilityLabel("Cameras, showing \(shown.title)")
        }
    }

    private func chip(_ shown: CameraEntry, splitOn: Bool) -> some View {
        let white = theme.aircast.outdoorForeground
        return HStack(spacing: CHIP_GAP) {
            CameraStatusDot(status: shown.status)
            CappedWidth(limit: CHIP_LABEL_MAX) {
                Text(shown.short).font(.labelLarge).lineLimit(1).truncationMode(.tail)
            }
            if splitOn { Image(.splitView).font(.system(size: CHIP_GLYPH)) }
            Image(.arrowDropDown).font(.system(size: CHIP_ARROW, weight: .semibold))
        }
        .padding(.horizontal, CHIP_PAD_HORIZONTAL)
        .padding(.vertical, CHIP_PAD_VERTICAL)
        .foregroundStyle(white)
        .background(osdBackdrop(Color.black.opacity(SWITCH_SCRIM_ALPHA), flyOsd), in: Capsule())
        .overlay(Capsule().stroke(white.opacity(SWITCH_BORDER_ALPHA), lineWidth: SWITCH_BORDER))
        .frame(minHeight: MINIMUM_TOUCH_TARGET)
        .contentShape(Rectangle())
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
    @ShownCameras private var reading

    init() {}

    var wrappedValue: (Int) -> Bool {
        let state = cameraSwitchState(reading)
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

@propertyWrapper
struct SyntheticAims: DynamicProperty {
    @QgcPath(SYNTHETIC_VIEW) private var synthetic
    @ShownCameras private var cameras

    init() {}

    var wrappedValue: Bool { syntheticAims(synthetic, cameras) }
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
