import SwiftUI

private let MAP3D_VIEW = "view.map3d"
private let MAP3D_PAN = "map3d.pan"
private let MAP3D_TURN = "map3d.turn"
private let MAP3D_TILT = "map3d.tilt"
private let MAP3D_ZOOM = "map3d.zoom"
private let MAP3D_FOLLOW = "map3d.follow"
private let MAP3D_NORTH = "map3d.north"
private let MAP3D_OPEN = "map3d.open"
private let MAP3D_PICK = "map3d.pick"
private let MAP3D_SUBJECT = "The 3D map"
private let TURN_PER_VIEW_WIDTH_DEG = 180.0
private let CONTROL_SCRIM_ALPHA = 0.55
private let CONTROL_SIZE: CGFloat = 40
private let CONTROL_ICON: CGFloat = 20
private let BACK_TO_AIRCRAFT = "Back to aircraft"
private let COMPASS_LABEL = "Compass. Drag to turn and tilt the view, tap to face north"
private let MAP3D_HINT = "Drag to move · Two fingers to turn and tilt"
private let HINT_SHOWN_MS = 8_000
private let NEEDLE_HALF_WIDTH: CGFloat = 0.22
private let NEEDLE_REACH: CGFloat = 0.8

func map3dToggleText(_ shown: Bool) -> String { shown ? "2D" : "3D" }

func map3dToggleLabel(_ shown: Bool) -> String { shown ? "Show the flat map" : "Show the 3D map" }

func pickedPoint(_ answer: JSON?) -> MapPoint? {
    guard let answer, answer["ok"].bool, let latitude = answer["latitude"].double, let longitude = answer["longitude"].double,
          latitude.isFinite, longitude.isFinite else { return nil }
    return MapPoint(latitude: latitude, longitude: longitude)
}

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
    private let onTouch: () -> Void
    private let onPress: (CGPoint, CGSize) -> Void

    init(onTouch: @escaping () -> Void, onPress: @escaping (CGPoint, CGSize) -> Void) {
        self.onTouch = onTouch
        self.onPress = onPress
        super.init(frame: .zero)
        let pan = UIPanGestureRecognizer(target: self, action: #selector(panned(_:)))
        pan.maximumNumberOfTouches = 2
        let follow = UITapGestureRecognizer(target: self, action: #selector(followed))
        follow.numberOfTapsRequired = 2
        let gestures: [UIGestureRecognizer] = [
            pan,
            UIPinchGestureRecognizer(target: self, action: #selector(pinched(_:))),
            UIRotationGestureRecognizer(target: self, action: #selector(turned(_:))),
            follow,
            UILongPressGestureRecognizer(target: self, action: #selector(pressed(_:))),
        ]
        gestures.forEach { gesture in
            gesture.delegate = self
            addGestureRecognizer(gesture)
        }
    }

    required init?(coder: NSCoder) { nil }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        onTouch()
        super.touchesBegan(touches, with: event)
    }

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

    @objc private func pressed(_ press: UILongPressGestureRecognizer) {
        if press.state == .began { onPress(press.location(in: self), bounds.size) }
    }
}
#elseif os(macOS)
private let WHEEL_ZOOM_STEP = 1.1

private final class Map3dTouches: NSView {
    override var isFlipped: Bool { true }
    private let onTouch: () -> Void
    private let onPress: (CGPoint, CGSize) -> Void

    init(onTouch: @escaping () -> Void, onPress: @escaping (CGPoint, CGSize) -> Void) {
        self.onTouch = onTouch
        self.onPress = onPress
        super.init(frame: .zero)
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
            mapPressRecognizer(self, #selector(pressed(_:))),
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
        if pinch.state == .began { onTouch() }
        guard pinch.state == .changed, pinch.magnification != 0 else { return }
        send(MAP3D_ZOOM, 1 + pinch.magnification)
        pinch.magnification = 0
    }

    @objc private func turned(_ rotation: NSRotationGestureRecognizer) {
        if rotation.state == .began { onTouch() }
        guard rotation.state == .changed, rotation.rotation != 0 else { return }
        send(MAP3D_TURN, -rotation.rotation * 180 / .pi)
        rotation.rotation = 0
    }

    @objc private func followed() { send(MAP3D_FOLLOW) }

    @objc private func pressed(_ press: NSPressGestureRecognizer) {
        if press.state == .began { onPress(press.location(in: self), bounds.size) }
    }

    override func mouseDown(with event: NSEvent) {
        onTouch()
        super.mouseDown(with: event)
    }

    override func rightMouseDown(with event: NSEvent) {
        onTouch()
        super.rightMouseDown(with: event)
    }

    override func scrollWheel(with event: NSEvent) {
        onTouch()
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
    let onTouch: () -> Void
    let onPress: (CGPoint, CGSize) -> Void

    func makePlatformView(context: Context) -> Map3dTouches { Map3dTouches(onTouch: onTouch, onPress: onPress) }
}

private struct Compass: View {
    let heading: Double
    let view: CGSize
    @State private var dragged = CGSize.zero
    @Environment(\.theme) private var theme

    var body: some View {
        let north = theme.colors.error
        let south = theme.aircast.outdoorForeground
        Canvas { context, size in
            let (cx, cy) = (size.width / 2, size.height / 2)
            let (reach, half) = (cy * NEEDLE_REACH, cx * NEEDLE_HALF_WIDTH)
            let needle = { (tip: CGFloat) in
                Path { $0.addLines([CGPoint(x: cx, y: tip), CGPoint(x: cx + half, y: cy), CGPoint(x: cx - half, y: cy)]); $0.closeSubpath() }
            }
            context.fill(needle(cy - reach), with: .color(north))
            context.fill(needle(cy + reach), with: .color(south))
        }
        .frame(width: CONTROL_ICON, height: CONTROL_ICON)
        .rotationEffect(.degrees(-heading))
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

private struct Hint: View {
    @Environment(\.theme) private var theme

    var body: some View {
        Text(MAP3D_HINT)
            .font(.labelLarge)
            .foregroundStyle(theme.aircast.outdoorForeground)
            .padding(.horizontal, Space.s3)
            .padding(.vertical, Space.s2)
            .background(Color.black.opacity(CONTROL_SCRIM_ALPHA), in: Capsule())
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
    let onMapClick: (Double, Double) -> Void
    let onRefused: (String) -> Void
    @QgcPath(SYNTHETIC_VIEW) private var synthetic
    @QgcPath(MAP3D_VIEW) private var map3dJson
    @State private var hinting = !Map3d.shared.hinted
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
                Map3dGestures(onTouch: { hinting = false }, onPress: pick)
                VStack(alignment: .trailing, spacing: Space.s2) {
                    if hinting { Hint() }
                    HStack(spacing: Space.s2) {
                        if state.aircraft && !state.following { BackToAircraft() }
                        Compass(heading: state.heading, view: geometry.size)
                    }
                }
                .padding(logoEndInsetPx.map { EdgeInsets(top: 0, leading: 0, bottom: bottomInsetPx + Space.s3, trailing: $0) }
                    ?? EdgeInsets(top: Space.s3, leading: Space.s3, bottom: Space.s3, trailing: Space.s3))
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
            }
        }
        .task(id: hinting) {
            guard hinting else { return }
            Map3d.shared.markHinted()
            try? await Task.sleep(for: .milliseconds(HINT_SHOWN_MS))
            if !Task.isCancelled { hinting = false }
        }
        .onAppear {
            guard let centre = FlightMapPosition.latest ?? readCamera()?.centre else { return send(MAP3D_OPEN) }
            send(MAP3D_OPEN, centre.latitude, centre.longitude)
        }
    }

    private func pick(_ at: CGPoint, _ size: CGSize) {
        Task {
            let answer = await offMain { Qgc.call(MAP3D_PICK, Double(at.x), Double(at.y), Double(size.width), Double(size.height)) }
            if let point = pickedPoint(answer) {
                onMapClick(point.latitude, point.longitude)
            } else if let reason = refusal(answer, subject: MAP3D_SUBJECT) {
                onRefused(reason)
            }
        }
    }
}
