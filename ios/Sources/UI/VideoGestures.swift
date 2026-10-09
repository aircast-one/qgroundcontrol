import SwiftUI

enum VideoSwipe: Equatable {
    case Up, Down, Left, Right
}

func videoSwipe(_ moved: CGSize, _ threshold: CGFloat) -> VideoSwipe? {
    if max(abs(moved.width), abs(moved.height)) < threshold { return nil }
    if abs(moved.height) > abs(moved.width) { return moved.height < 0 ? .Up : .Down }
    return moved.width < 0 ? .Left : .Right
}

func sideways(_ moved: CGSize) -> Bool { abs(moved.width) > abs(moved.height) }

func pipOnStart(_ centreX: CGFloat, _ width: CGFloat) -> Bool { centreX < width / 2 }

final class VideoGestureHandlers {
    let owned: () -> Bool
    let claimsSwipe: (CGSize) -> Bool
    let onTap: () -> Void
    let onDoubleTap: () -> Void
    let onSwipe: (CGSize) -> Void
    let onHold: (CGPoint) -> Bool
    let onHoldDrag: (CGSize) -> Void
    let onHoldEnd: () -> Void

    init(
        owned: @escaping () -> Bool,
        claimsSwipe: @escaping (CGSize) -> Bool,
        onTap: @escaping () -> Void,
        onDoubleTap: @escaping () -> Void,
        onSwipe: @escaping (CGSize) -> Void,
        onHold: @escaping (CGPoint) -> Bool,
        onHoldDrag: @escaping (CGSize) -> Void,
        onHoldEnd: @escaping () -> Void
    ) {
        self.owned = owned
        self.claimsSwipe = claimsSwipe
        self.onTap = onTap
        self.onDoubleTap = onDoubleTap
        self.onSwipe = onSwipe
        self.onHold = onHold
        self.onHoldDrag = onHoldDrag
        self.onHoldEnd = onHoldEnd
    }
}

private let TOUCH_SLOP: CGFloat = 8
private let LONG_PRESS_MS = 400
private let DOUBLE_TAP_MS = 300

private enum PressPhase: Equatable {
    case pressing, swiping, holding, ignored, secondTap
}

private struct Press: Equatable {
    let start: CGPoint
    let owned: Bool
    var phase: PressPhase
    var last: CGPoint
}

private struct VideoGesturesModifier: ViewModifier {
    let handlers: VideoGestureHandlers
    @State private var press: Press?
    @State private var origin = CGPoint.zero
    @State private var holdTimer: Task<Void, Never>?
    @State private var tapTimer: Task<Void, Never>?
    @GestureState private var touching = false

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGPoint.self) { $0.frame(in: .global).origin } action: { origin = $0 }
            .simultaneousGesture(
                DragGesture(minimumDistance: 0, coordinateSpace: .global)
                    .updating($touching) { _, touching, _ in touching = true }
                    .onChanged(changed)
                    .onEnded(ended)
            )
            .simultaneousGesture(MagnifyGesture(minimumScaleDelta: 0).onChanged { _ in secondFinger() })
            .onChange(of: touching) { _, now in if !now { lost() } }
            .onDisappear {
                lost()
                tapTimer?.cancel()
                tapTimer = nil
            }
    }

    private func lost() {
        holdTimer?.cancel()
        holdTimer = nil
        guard let gone = press else { return }
        press = nil
        if gone.phase == .holding { handlers.onHoldEnd() }
    }

    private func secondFinger() {
        guard press?.phase == .pressing else { return }
        holdTimer?.cancel()
        holdTimer = nil
        press?.phase = .ignored
    }

    private func changed(_ drag: DragGesture.Value) {
        guard let current = press else { return began(drag) }
        let moved = drag.translation
        switch current.phase {
        case .pressing where hypot(moved.width, moved.height) > TOUCH_SLOP:
            holdTimer?.cancel()
            press?.phase = current.owned || handlers.claimsSwipe(moved) ? .swiping : .ignored
        case .holding:
            handlers.onHoldDrag(CGSize(width: drag.location.x - current.last.x, height: drag.location.y - current.last.y))
        default:
            break
        }
        press?.last = drag.location
    }

    private func began(_ drag: DragGesture.Value) {
        let second = tapTimer != nil
        tapTimer?.cancel()
        tapTimer = nil
        press = Press(start: CGPoint(x: drag.startLocation.x - origin.x, y: drag.startLocation.y - origin.y), owned: second || handlers.owned(), phase: second ? .secondTap : .pressing, last: drag.location)
        guard !second else { return }
        holdTimer = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(LONG_PRESS_MS))
            guard !Task.isCancelled, let held = press, held.phase == .pressing else { return }
            press?.phase = handlers.onHold(held.start) ? .holding : .ignored
        }
    }

    private func ended(_ drag: DragGesture.Value) {
        holdTimer?.cancel()
        holdTimer = nil
        let finished = press
        press = nil
        switch finished?.phase {
        case .holding:
            handlers.onHoldEnd()
        case .swiping:
            handlers.onSwipe(drag.translation)
        case .secondTap:
            handlers.onDoubleTap()
        case .pressing where finished?.owned == true:
            tapTimer = Task { @MainActor in
                try? await Task.sleep(for: .milliseconds(DOUBLE_TAP_MS))
                guard !Task.isCancelled else { return }
                tapTimer = nil
                handlers.onTap()
            }
        default:
            break
        }
    }
}

extension View {
    func videoGestures(_ handlers: VideoGestureHandlers) -> some View {
        modifier(VideoGesturesModifier(handlers: handlers))
    }

    func verticalSwipe(_ threshold: CGFloat, onSwipe: @escaping (VideoSwipe) -> Void) -> some View {
        simultaneousGesture(
            DragGesture(minimumDistance: TOUCH_SLOP).onEnded { drag in
                videoSwipe(drag.translation, threshold).map(onSwipe)
            }
        )
    }

    func videoFrame(_ target: CGRect, holding: Bool, nudge: CGFloat) -> some View {
        frame(width: max(target.width, 0), height: max(target.height, 0))
            .position(x: target.midX + nudge, y: target.midY)
            .animation(holding ? nil : VIDEO_FRAME_SPRING, value: target)
    }
}

let CAMERA_NUDGE: CGFloat = 48
let VIDEO_SWIPE_DISTANCE: CGFloat = 32
private let VIDEO_FRAME_SPRING = Animation.spring(response: 0.314, dampingFraction: 1)

@propertyWrapper
struct CameraStepper: DynamicProperty {
    @CameraSwiper private var swipeCamera
    @State private var offset: CGFloat = 0

    var wrappedValue: CameraStepper { self }

    var nudge: CGFloat { offset }

    func step(_ step: Int) {
        guard swipeCamera(step) else { return }
        let shift = $offset
        shift.wrappedValue = CAMERA_NUDGE * CGFloat(step)
        DispatchQueue.main.async { withAnimation(VIDEO_FRAME_SPRING) { shift.wrappedValue = 0 } }
    }
}

func cameraStep(_ swipe: VideoSwipe?) -> Int? {
    switch swipe {
    case .Left: 1
    case .Right: -1
    default: nil
    }
}
