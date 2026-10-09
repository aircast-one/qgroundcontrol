import Combine
import SwiftUI
import os

let VIRTUAL_JOYSTICK_PATH = "view.virtualJoystick"
let VIRTUAL_JOYSTICK_VALUE = "vehicle.virtualTabletJoystickValue"
private let MAX_PAD_SIZE: CGFloat = 160
private let PAD_HEIGHT_FRACTION: CGFloat = 0.25
private let DEFAULT_PERIOD_MS: Int64 = 40

struct VirtualJoystickState: Equatable, Sendable {
    var show: Bool
    var sending: Bool
    var autoCenterThrottle: Bool
    var leftHandedMode: Bool
    var leftPositiveOnly: Bool
    var rightPositiveOnly: Bool
    var periodMs: Int64
}

func virtualJoystick(_ view: JSON?) -> VirtualJoystickState? {
    guard let view, view["class"].string == "VirtualJoystick" else { return nil }
    return VirtualJoystickState(
        show: view["show"].bool,
        sending: view["sending"].bool,
        autoCenterThrottle: view["autoCenterThrottle"].bool,
        leftHandedMode: view["leftHandedMode"].bool,
        leftPositiveOnly: view["leftPositiveOnly"].bool,
        rightPositiveOnly: view["rightPositiveOnly"].bool,
        periodMs: view["periodMs"].int64 ?? DEFAULT_PERIOD_MS
    )
}

struct StickAxes: Equatable {
    let x: Double
    let y: Double
}

struct StickStates: Equatable {
    let previous: VirtualJoystickState?
    let current: VirtualJoystickState?
}

func sticksReset(_ previous: VirtualJoystickState?, _ current: VirtualJoystickState?) -> Bool {
    current?.show != true || current?.autoCenterThrottle != previous?.autoCenterThrottle
}

func restingLeft(_ autoCenterThrottle: Bool) -> CGPoint { CGPoint(x: 0.5, y: restingY(autoCenterThrottle)) }

let RESTING_RIGHT = CGPoint(x: 0.5, y: 0.5)

func released(_ stick: CGPoint, _ reCenterY: Bool) -> CGPoint { CGPoint(x: 0.5, y: reCenterY ? 0.5 : stick.y) }

func stickValues(_ state: VirtualJoystickState, _ left: CGPoint?, _ right: CGPoint?) -> [Double] {
    let l = left ?? restingLeft(state.autoCenterThrottle)
    let r = right ?? RESTING_RIGHT
    return joystickValues(stickAxes(l.x, l.y, state.leftPositiveOnly), stickAxes(r.x, r.y, state.rightPositiveOnly), state.leftHandedMode)
}

enum VirtualStickSender {
    private static let held = OSAllocatedUnfairLock<(left: CGPoint?, right: CGPoint?)>(initialState: (nil, nil))
    private static var watching: AnyCancellable?
    private static var sender: Task<Void, Never>?

    static var left: CGPoint? {
        get { held.withLock { $0.left } }
        set { held.withLock { $0.left = newValue } }
    }

    static var right: CGPoint? {
        get { held.withLock { $0.right } }
        set { held.withLock { $0.right = newValue } }
    }

    @MainActor
    static func start() {
        watching = QgcWatch.retain(VIRTUAL_JOYSTICK_PATH).$json
            .map(virtualJoystick)
            .removeDuplicates()
            .scan(StickStates(previous: nil, current: nil)) { states, next in StickStates(previous: states.current, current: next) }
            .sink { states in
                if sticksReset(states.previous, states.current) {
                    left = nil
                    right = nil
                }
                sender?.cancel()
                sender = states.current.flatMap { $0.show && $0.sending ? $0 : nil }.map { sending in
                    Task.detached(priority: .userInitiated) {
                        while !Task.isCancelled {
                            _ = Qgc.call(VIRTUAL_JOYSTICK_VALUE, arguments: stickValues(sending, left, right))
                            try? await Task.sleep(for: .milliseconds(sending.periodMs))
                        }
                    }
                }
            }
    }
}

func stickAxes(_ fractionX: CGFloat, _ fractionY: CGFloat, _ positiveOnly: Bool) -> StickAxes {
    let pctUp = 1.0 - Double(min(max(fractionY, 0), 1))
    return StickAxes(
        x: Double(min(max(fractionX, 0), 1)) * 2.0 - 1.0,
        y: positiveOnly ? pctUp : pctUp * 2.0 - 1.0
    )
}

func restingY(_ reCenter: Bool) -> CGFloat { reCenter ? 0.5 : 1 }

func joystickValues(_ left: StickAxes, _ right: StickAxes, _ leftHanded: Bool) -> [Double] {
    leftHanded ? [left.x, left.y, right.x, right.y] : [right.x, right.y, left.x, left.y]
}

struct VirtualJoystick: View {
    @QgcPath(VIRTUAL_JOYSTICK_PATH) private var view

    var body: some View {
        if let state = virtualJoystick(view), state.show {
            VirtualJoystickPads(state: state)
        }
    }
}

private struct VirtualJoystickPads: View {
    let state: VirtualJoystickState
    @State private var left: CGPoint
    @State private var right: CGPoint

    init(state: VirtualJoystickState) {
        self.state = state
        _left = State(initialValue: VirtualStickSender.left ?? restingLeft(state.autoCenterThrottle))
        _right = State(initialValue: VirtualStickSender.right ?? RESTING_RIGHT)
    }

    var body: some View {
        HStack(spacing: 0) {
            ThumbPad(stick: left, reCenterY: state.autoCenterThrottle, description: "Left virtual stick") { left = $0 }
            Spacer(minLength: 0)
            ThumbPad(stick: right, reCenterY: true, description: "Right virtual stick") { right = $0 }
        }
        .frame(maxWidth: .infinity)
        .containerRelativeFrame(.vertical) { length, _ in min(length * PAD_HEIGHT_FRACTION, MAX_PAD_SIZE) }
        .onChange(of: state.autoCenterThrottle) { _, autoCenter in
            left = VirtualStickSender.left ?? restingLeft(autoCenter)
        }
        .onChange(of: left, initial: true) { _, stick in VirtualStickSender.left = stick }
        .onChange(of: right, initial: true) { _, stick in VirtualStickSender.right = stick }
        .onDisappear {
            VirtualStickSender.left = VirtualStickSender.left.map { released($0, state.autoCenterThrottle) }
            VirtualStickSender.right = VirtualStickSender.right.map { released($0, true) }
        }
    }
}

private struct ThumbPad: View {
    let stick: CGPoint
    let reCenterY: Bool
    let description: String
    let onMove: (CGPoint) -> Void
    @State private var start: CGPoint?
    @GestureState private var touching = false
    @Environment(\.theme) private var theme

    var body: some View {
        let ring = theme.colors.onSurface
        let fill = theme.colors.surface.opacity(0.5)
        let hat = theme.colors.primary
        GeometryReader { geometry in
            let size = geometry.size
            Canvas { context, canvas in
                let radius = min(canvas.width, canvas.height) / 2
                let center = CGPoint(x: canvas.width / 2, y: canvas.height / 2)
                let circle = { (r: CGFloat, at: CGPoint) in Path(ellipseIn: CGRect(x: at.x - r, y: at.y - r, width: r * 2, height: r * 2)) }
                context.fill(circle(radius, center), with: .color(fill))
                context.stroke(circle(radius, center), with: .color(ring), lineWidth: 2)
                context.stroke(circle(radius / 2, center), with: .color(ring), lineWidth: 2)
                context.fill(circle(12, CGPoint(x: stick.x * canvas.width, y: stick.y * canvas.height)), with: .color(hat))
            }
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($touching) { _, down, _ in down = true }
                    .onChanged { drag in
                        start = start ?? stick
                        let origin = start ?? stick
                        onMove(CGPoint(
                            x: min(max(origin.x + drag.translation.width / size.width, 0), 1),
                            y: min(max(origin.y + drag.translation.height / size.height, 0), 1)
                        ))
                    }
            )
            .onChange(of: touching) { _, down in
                if !down {
                    start = nil
                    onMove(released(stick, reCenterY))
                }
            }
        }
        .aspectRatio(1, contentMode: .fit)
        .accessibilityElement()
        .accessibilityLabel(description)
    }
}
