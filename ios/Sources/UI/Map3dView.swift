import SwiftUI

private let MAP3D_VIEW = "view.map3d"
private let MAP3D_PAN = "map3d.pan"
private let MAP3D_TURN = "map3d.turn"
private let MAP3D_TILT = "map3d.tilt"
private let MAP3D_ZOOM = "map3d.zoom"
private let MAP3D_FOLLOW = "map3d.follow"
private let MAP3D_NORTH = "map3d.north"
private let MAP3D_OPEN = "map3d.open"
private let TURN_PER_VIEW_WIDTH_DEG = 180.0
private let CONTROL_SCRIM_ALPHA = 0.55
private let CONTROL_SIZE: CGFloat = 40
private let CONTROL_ICON: CGFloat = 20
private let BACK_TO_AIRCRAFT = "Back to aircraft"
private let COMPASS_LABEL = "Compass. Drag to turn and tilt the view, tap to face north"

struct Map3dState: Equatable {
    let heading: Double
    let following: Bool
    let aircraft: Bool
}

func map3dState(_ view: JSON?) -> Map3dState {
    let view = view ?? .null
    return Map3dState(heading: view["heading"].double ?? 0, following: view["following"].bool(true), aircraft: view["aircraft"].bool)
}

func map3dNote(_ synthetic: JSON?, _ state: Map3dState) -> String? {
    !syntheticAvailable(synthetic) && state.following ? LOOKING_TITLE : nativeDraw(synthetic).note
}

private func send(_ path: String, _ args: Double...) {
    offMainInOrder { _ = Qgc.invoke(path, arguments: args) }
}

#if os(iOS)
private final class Map3dTouches: UIView, UIGestureRecognizerDelegate {
    private var twoFingered = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        let pan = UIPanGestureRecognizer(target: self, action: #selector(panned(_:)))
        pan.maximumNumberOfTouches = 2
        let follow = UITapGestureRecognizer(target: self, action: #selector(followed))
        follow.numberOfTapsRequired = 2
        let gestures: [UIGestureRecognizer] = [
            pan,
            UIPinchGestureRecognizer(target: self, action: #selector(pinched(_:))),
            UIRotationGestureRecognizer(target: self, action: #selector(turned(_:))),
            follow,
        ]
        gestures.forEach { gesture in
            gesture.delegate = self
            addGestureRecognizer(gesture)
        }
    }

    required init?(coder: NSCoder) { nil }

    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        true
    }

    @objc private func panned(_ pan: UIPanGestureRecognizer) {
        if pan.state == .began { twoFingered = false }
        guard pan.state == .changed else { return }
        let moved = pan.translation(in: self)
        pan.setTranslation(.zero, in: self)
        twoFingered = twoFingered || pan.numberOfTouches > 1
        if pan.numberOfTouches > 1 {
            if moved.y != 0 { send(MAP3D_TILT, -moved.y, bounds.height) }
        } else if !twoFingered && moved != .zero {
            send(MAP3D_PAN, moved.x, moved.y, bounds.width)
        }
    }

    @objc private func pinched(_ pinch: UIPinchGestureRecognizer) {
        guard pinch.state == .changed, pinch.scale != 1 else { return }
        send(MAP3D_ZOOM, pinch.scale)
        pinch.scale = 1
    }

    @objc private func turned(_ rotation: UIRotationGestureRecognizer) {
        guard rotation.state == .changed, rotation.rotation != 0 else { return }
        send(MAP3D_TURN, rotation.rotation * 180 / .pi)
        rotation.rotation = 0
    }

    @objc private func followed() { send(MAP3D_FOLLOW) }
}
#elseif os(macOS)
// The Mac's pointer and trackpad stand in for touch: drag pans, right-drag or a two-finger trackpad scroll tilts,
// pinch or the mouse wheel zooms, a trackpad rotate turns, and a double click follows the aircraft again.
private let WHEEL_ZOOM_STEP = 1.1

private final class Map3dTouches: NSView {
    override var isFlipped: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        let pan = NSPanGestureRecognizer(target: self, action: #selector(panned(_:)))
        let tilt = NSPanGestureRecognizer(target: self, action: #selector(tilted(_:)))
        tilt.buttonMask = 0x2
        let follow = NSClickGestureRecognizer(target: self, action: #selector(followed))
        follow.numberOfClicksRequired = 2
        let gestures: [NSGestureRecognizer] = [
            pan,
            tilt,
            NSMagnificationGestureRecognizer(target: self, action: #selector(pinched(_:))),
            NSRotationGestureRecognizer(target: self, action: #selector(turned(_:))),
            follow,
        ]
        gestures.forEach { addGestureRecognizer($0) }
    }

    required init?(coder: NSCoder) { nil }

    @objc private func panned(_ pan: NSPanGestureRecognizer) {
        guard pan.state == .changed else { return }
        let moved = pan.translation(in: self)
        pan.setTranslation(.zero, in: self)
        if moved != .zero { send(MAP3D_PAN, moved.x, moved.y, bounds.width) }
    }

    @objc private func tilted(_ pan: NSPanGestureRecognizer) {
        guard pan.state == .changed else { return }
        let moved = pan.translation(in: self)
        pan.setTranslation(.zero, in: self)
        if moved.y != 0 { send(MAP3D_TILT, -moved.y, bounds.height) }
    }

    @objc private func pinched(_ pinch: NSMagnificationGestureRecognizer) {
        guard pinch.state == .changed, pinch.magnification != 0 else { return }
        send(MAP3D_ZOOM, 1 + pinch.magnification)
        pinch.magnification = 0
    }

    // AppKit counts rotation anticlockwise and UIKit clockwise, so the sign flips to turn the same way as on iOS.
    @objc private func turned(_ rotation: NSRotationGestureRecognizer) {
        guard rotation.state == .changed, rotation.rotation != 0 else { return }
        send(MAP3D_TURN, -rotation.rotation * 180 / .pi)
        rotation.rotation = 0
    }

    @objc private func followed() { send(MAP3D_FOLLOW) }

    override func scrollWheel(with event: NSEvent) {
        if event.hasPreciseScrollingDeltas {
            if event.scrollingDeltaY != 0 { send(MAP3D_TILT, -event.scrollingDeltaY, bounds.height) }
        } else if event.scrollingDeltaY != 0 {
            send(MAP3D_ZOOM, pow(WHEEL_ZOOM_STEP, Double(event.scrollingDeltaY)))
        }
    }
}
#endif

private struct Map3dGestures: PlatformViewRepresentable {
    typealias Coordinator = Void

    func makePlatformView(context: Context) -> Map3dTouches { Map3dTouches(frame: .zero) }
}

private struct Compass: View {
    let heading: Double
    let view: CGSize
    @State private var dragged = CGSize.zero
    @Environment(\.theme) private var theme

    var body: some View {
        Image(.navigation)
            .font(.system(size: CONTROL_ICON))
            .rotationEffect(.degrees(-heading))
            .foregroundStyle(theme.aircast.outdoorForeground)
            .frame(width: CONTROL_SIZE, height: CONTROL_SIZE)
            .background(Color.black.opacity(CONTROL_SCRIM_ALPHA), in: Circle())
            .contentShape(Circle())
            .onTapGesture { send(MAP3D_NORTH) }
            .gesture(
                DragGesture()
                    .onChanged { drag in
                        let dx = drag.translation.width - dragged.width
                        let dy = drag.translation.height - dragged.height
                        dragged = drag.translation
                        send(MAP3D_TURN, dx / max(view.width, 1) * TURN_PER_VIEW_WIDTH_DEG)
                        send(MAP3D_TILT, -dy, view.height)
                    }
                    .onEnded { _ in dragged = .zero }
            )
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(COMPASS_LABEL)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { send(MAP3D_NORTH) }
    }
}

private struct BackToAircraft: View {
    @Environment(\.theme) private var theme

    var body: some View {
        Button { send(MAP3D_FOLLOW) } label: {
            HStack(spacing: Space.s2) {
                Image(.myLocation).font(.system(size: CONTROL_ICON))
                Text(BACK_TO_AIRCRAFT).font(.labelLarge)
            }
            .foregroundStyle(theme.aircast.outdoorForeground)
            .padding(.horizontal, Space.s3)
            .padding(.vertical, Space.s2)
            .background(Color.black.opacity(CONTROL_SCRIM_ALPHA), in: Capsule())
            .minimumTouchTarget()
        }
        .buttonStyle(.plain)
    }
}

struct Map3dView: View {
    let bottomInsetPx: CGFloat
    let logoEndInsetPx: CGFloat?
    @QgcPath(SYNTHETIC_VIEW) private var synthetic
    @QgcPath(MAP3D_VIEW) private var map3dJson
    @Environment(\.theme) private var theme

    var body: some View {
        let state = map3dState(map3dJson)
        GeometryReader { geometry in
            ZStack {
                SyntheticSurface(orbit: true)
                if let note = map3dNote(synthetic, state) {
                    Text(note)
                        .font(.labelLarge)
                        .foregroundStyle(theme.aircast.outdoorForeground)
                        .multilineTextAlignment(.center)
                        .padding(Space.s4)
                        .osdShadow()
                }
                Map3dGestures()
                HStack(spacing: Space.s2) {
                    if state.aircraft && !state.following { BackToAircraft() }
                    Compass(heading: state.heading, view: geometry.size)
                }
                .padding(logoEndInsetPx.map { EdgeInsets(top: 0, leading: 0, bottom: bottomInsetPx + Space.s3, trailing: $0) }
                    ?? EdgeInsets(top: Space.s3, leading: Space.s3, bottom: Space.s3, trailing: Space.s3))
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
            }
        }
        .onAppear {
            guard let centre = FlightMapPosition.latest ?? readCamera()?.centre else { return send(MAP3D_OPEN) }
            send(MAP3D_OPEN, centre.latitude, centre.longitude)
        }
    }
}
