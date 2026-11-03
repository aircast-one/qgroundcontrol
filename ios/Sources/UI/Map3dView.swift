import SwiftUI

private let MAP3D_PAN = "map3d.pan"
private let MAP3D_TURN = "map3d.turn"
private let MAP3D_TILT = "map3d.tilt"
private let MAP3D_ZOOM = "map3d.zoom"
private let MAP3D_FOLLOW = "map3d.follow"

func map3dNote(_ view: JSON?) -> String? { syntheticAvailable(view) ? nativeDraw(view).note : LOOKING_TITLE }

private func send(_ path: String, _ args: Double...) {
    offMainInOrder { _ = Qgc.invoke(path, arguments: args) }
}

private final class Map3dTouches: UIView, UIGestureRecognizerDelegate {
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
        guard pan.state == .changed else { return }
        let moved = pan.translation(in: self)
        pan.setTranslation(.zero, in: self)
        if pan.numberOfTouches > 1 {
            send(MAP3D_TILT, -moved.y, bounds.height)
        } else {
            send(MAP3D_PAN, moved.x, moved.y, bounds.width)
        }
    }

    @objc private func pinched(_ pinch: UIPinchGestureRecognizer) {
        guard pinch.state == .changed else { return }
        send(MAP3D_ZOOM, pinch.scale)
        pinch.scale = 1
    }

    @objc private func turned(_ rotation: UIRotationGestureRecognizer) {
        guard rotation.state == .changed else { return }
        send(MAP3D_TURN, rotation.rotation * 180 / .pi)
        rotation.rotation = 0
    }

    @objc private func followed() { send(MAP3D_FOLLOW) }
}

private struct Map3dGestures: UIViewRepresentable {
    func makeUIView(context: Context) -> Map3dTouches { Map3dTouches(frame: .zero) }

    func updateUIView(_ uiView: Map3dTouches, context: Context) {}
}

struct Map3dView: View {
    @QgcPath(SYNTHETIC_VIEW) private var view
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack {
            SyntheticSurface(orbit: true)
            if let note = map3dNote(view) {
                Text(note)
                    .font(.labelLarge)
                    .foregroundStyle(theme.aircast.outdoorForeground)
                    .multilineTextAlignment(.center)
                    .padding(Space.s4)
                    .osdShadow()
            }
            Map3dGestures()
        }
    }
}
