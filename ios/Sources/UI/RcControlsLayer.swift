import SwiftUI
import os

final class RcHolder: Sendable {
    private let send: @Sendable (@escaping @Sendable () -> Void) -> Void
    private let sent = OSAllocatedUnfairLock(initialState: [Int: Int]())

    init(send: @escaping @Sendable (@escaping @Sendable () -> Void) -> Void = { offMainInOrder($0) }) {
        self.send = send
    }

    func hold(_ channel: Int, _ pwm: Int) {
        guard channel > 0 else { return }
        sent.withLock { $0[channel] = pwm }
        send { VehicleCommands.overrideRcChannel(channel, pwm: pwm) }
    }

    func release() {
        let held = sent.withLock { held in
            let all = held.keys.sorted()
            held.removeAll()
            return all
        }
        if !held.isEmpty {
            send { held.forEach { VehicleCommands.releaseRcChannel($0) } }
        }
    }

    func forget() { sent.withLock { $0.removeAll() } }

    func holding() -> Set<Int> { sent.withLock { Set($0.keys) } }

    func held(_ channel: Int) -> Int? { sent.withLock { $0[channel] } }
}

let customRcControls = RcHolder()
let cameraRcControls = RcHolder()

private func sendRcOverride(_ channel: Int, _ pwm: Int) { customRcControls.hold(channel, pwm) }

private func releaseOverrides() {
    customRcControls.forget()
    cameraRcControls.forget()
    offMainInOrder { VehicleCommands.clearRcOverrides() }
}

private let RC_RELEASE_GRACE_MS = 300

@MainActor private var rcLayersShown = 0

@MainActor private func rcLayerShown() { rcLayersShown += 1 }

@MainActor private func rcLayerGone() {
    rcLayersShown -= 1
    Task { @MainActor in
        try? await Task.sleep(for: .milliseconds(RC_RELEASE_GRACE_MS))
        if rcLayersShown == 0 { customRcControls.release() }
    }
}

private struct ControlLabel: View {
    let text: String

    var body: some View {
        Text(text).font(.labelMedium).frame(width: 84, alignment: .leading)
    }
}

private struct RcSlider: View {
    let control: RcControl
    @State private var pwm: Int
    @State private var lastSent: Int64 = 0

    init(control: RcControl) {
        self.control = control
        _pwm = State(initialValue: customRcControls.held(control.channel) ?? PWM_CENTER)
    }

    private func send(_ value: Int, _ finished: Bool) {
        let now = uptimeMillis()
        if rcSendDue(now, lastSent, finished) {
            lastSent = now
            sendRcOverride(control.channel, value)
        }
    }

    var body: some View {
        HStack {
            ControlLabel(text: control.label)
            Slider(
                value: Binding(
                    get: { Double(pwm) },
                    set: { raw in
                        let next = Int(raw)
                        if next != pwm {
                            pwm = next
                            send(next, false)
                        }
                    }
                ),
                in: Double(PWM_MIN)...Double(PWM_MAX),
                onEditingChanged: { editing in if !editing { send(pwm, true) } }
            )
            .frame(maxWidth: .infinity)
        }
    }
}

private struct RcButton: View {
    let control: RcControl
    @State private var on: Bool

    init(control: RcControl) {
        self.control = control
        _on = State(initialValue: customRcControls.held(control.channel) == PWM_MAX)
    }

    var body: some View {
        Toggle(control.label, isOn: Binding(
            get: { on },
            set: { next in
                on = next
                sendRcOverride(control.channel, next ? PWM_MAX : PWM_MIN)
            }
        ))
        .toggleStyle(.button)
    }
}

private let SWITCH3_LABELS = ["Low", "Mid", "High"]

private struct RcSwitch3: View {
    let control: RcControl
    @State private var position: Int

    init(control: RcControl) {
        self.control = control
        _position = State(initialValue: switch3Pwms().firstIndex(of: customRcControls.held(control.channel) ?? PWM_CENTER) ?? 1)
    }

    var body: some View {
        HStack {
            ControlLabel(text: control.label)
            Picker(control.label, selection: Binding(
                get: { position },
                set: { index in
                    position = index
                    sendRcOverride(control.channel, switch3Pwms()[index])
                }
            )) {
                ForEach(Array(SWITCH3_LABELS.enumerated()), id: \.offset) { index, label in Text(label).tag(index) }
            }
            .pickerStyle(.segmented)
            .fixedSize()
        }
    }
}

enum MomentaryPress { case idle, held, cancelled }

func momentaryPress(_ press: MomentaryPress, _ location: CGPoint, _ size: CGSize) -> MomentaryPress {
    press != .cancelled && CGRect(origin: .zero, size: size).contains(location) ? .held : .cancelled
}

private struct RcMomentary: View {
    let control: RcControl
    @GestureState private var press = MomentaryPress.idle
    @State private var everPressed = false
    @State private var size = CGSize.zero

    var body: some View {
        Button(control.label) {}
            .buttonStyle(.filled)
            .onGeometryChange(for: CGSize.self) { $0.size } action: { size = $0 }
            .simultaneousGesture(
                DragGesture(minimumDistance: 0).updating($press) { drag, press, _ in press = momentaryPress(press, drag.location, size) }
            )
            .onChange(of: press == .held) { _, down in
                if down {
                    everPressed = true
                    sendRcOverride(control.channel, PWM_MAX)
                } else if everPressed {
                    sendRcOverride(control.channel, PWM_MIN)
                }
            }
    }
}

struct RcControlsLayer: View {
    @HasVehicle private var hasVehicle
    @QgcString(settingControl(RC_CONTROLS)) private var configured
    @QgcPath(FLY_STATE) private var stateJson
    @Environment(\.theme) private var theme

    var body: some View {
        let controls = parseRcControls(configured)
        let overriding = flyState(stateJson)?.rcOverride == true
        ZStack {
            Color.clear
                .invisibleAnchor()
                .onAppear { rcLayerShown() }
                .onDisappear { rcLayerGone() }
            if !controls.isEmpty && hasVehicle {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(Array(controls.enumerated()), id: \.offset) { _, control in
                        Group {
                            switch control.type {
                            case .Slider: RcSlider(control: control)
                            case .Button: RcButton(control: control)
                            case .Switch3: RcSwitch3(control: control)
                            case .Momentary: RcMomentary(control: control)
                            }
                        }
                        .id(control.channel)
                    }
                    if overriding {
                        HStack(spacing: Space.s2) {
                            Text("These channels are held by this tablet.")
                                .font(.labelMedium)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            Button("Give back") { releaseOverrides() }
                                .buttonStyle(.filled)
                        }
                    }
                }
                .padding(.horizontal, Space.s3)
                .padding(.vertical, Space.s2)
                .background(theme.colors.surface.opacity(0.85))
            }
        }
    }
}
