import SwiftUI

let GIMBAL_INDICATOR_PATH = "view.gimbalIndicator"
let OTHERS_HAVE_CONTROL = "othersHaveControl"

@Observable
final class GimbalControlRequest {
    var value = false
}

let gimbalAsksForControl = GimbalControlRequest()

func serialAsks(_ seen: Int64?, _ serial: Int64) -> Bool { seen.map { serial > $0 } ?? false }

func gimbalRefusal(_ answer: JSON?) -> String? {
    guard answer?["refusal"].string == OTHERS_HAVE_CONTROL else { return refusal(answer) }
    let ask = { gimbalAsksForControl.value = true }
    if Thread.isMainThread { ask() } else { DispatchQueue.main.async(execute: ask) }
    return nil
}

struct GimbalChoice: Equatable, Identifiable {
    var name: String
    var managerCompid: Int
    var deviceId: Int
    var active: Bool

    var id: String { "\(managerCompid)-\(deviceId)" }
}

struct GimbalIndicatorState: Equatable {
    var statusText: String
    var pitchText: String
    var yawText: String
    var yawLockLabel: String
    var yawLocked: Bool
    var yawLockOffered: Bool
    var retractOffered: Bool
    var controlOffered: Bool
    var controlLabel: String
    var haveControl: Bool
    var gimbals: [GimbalChoice]
    var pitchDegrees: Double? = nil
}

func gimbalIndicator(_ view: JSON?) -> GimbalIndicatorState? {
    guard let view, view["shown"].bool else { return nil }
    return GimbalIndicatorState(
        statusText: view["statusText"].string,
        pitchText: view["pitchText"].string,
        yawText: view["yawText"].string,
        yawLockLabel: view["yawLockLabel"].string,
        yawLocked: view["yawLocked"].bool,
        yawLockOffered: view["yawLockOffered"].bool,
        retractOffered: view["retractOffered"].bool,
        controlOffered: view["controlOffered"].bool,
        controlLabel: view["controlLabel"].string,
        haveControl: view["haveControl"].bool,
        gimbals: view["gimbals"].objects.map { g in
            GimbalChoice(name: g["name"].string, managerCompid: g["managerCompid"].int(0), deviceId: g["deviceId"].int(0), active: g["active"].bool)
        },
        pitchDegrees: view["pitchDegrees"].double.flatMap { $0.isFinite ? $0 : nil }
    )
}

func gimbalCellText(_ state: GimbalIndicatorState) -> String {
    let active = state.gimbals.count > 1 ? state.gimbals.first(where: \.active)?.name ?? "" : ""
    return [active, state.statusText, state.pitchText, state.yawText].filter { !$0.isBlank }.joined(separator: " · ")
}

struct GimbalTakeControlDialog: View {
    @QgcPath(GIMBAL_INDICATOR_PATH) private var view
    @State private var seen: Int64?

    var body: some View {
        let serial = view?["askSerial"].int64 ?? -1
        Color.clear
            .invisibleAnchor()
            .onChange(of: serial, initial: true) { _, now in
                if serialAsks(seen, now) { gimbalAsksForControl.value = true }
                if now >= 0 { seen = now }
            }
    }
}

struct GimbalTakeControlAlert: View {
    var body: some View {
        Color.clear
            .invisibleAnchor()
            .alert("Request Gimbal Control?", isPresented: .constant(true)) {
                Button("Yes") {
                    gimbalAsksForControl.value = false
                    offMain { _ = SetupCommands.takeGimbalControlRefusal() }
                }
                Button("No", role: .cancel) { gimbalAsksForControl.value = false }
            } message: {
                Text("Command not sent. Another user has control of the gimbal.")
            }
    }
}

struct GimbalIndicatorCell: View {
    @Environment(\.theme) private var theme
    @QgcPath(GIMBAL_INDICATOR_PATH) private var view
    @State private var open = false
    @State private var refusal: String?
    @State private var scope = ViewScope()

    var body: some View {
        if let state = gimbalIndicator(view) {
            Text(gimbalCellText(state))
                .font(.labelMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .lineLimit(1)
                .contentShape(Rectangle())
                .onTapGesture { open = true }
                .accessibilityAddTraits(.isButton)
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            NavigationStack { sheet(state).toolbar(.hidden, for: .navigationBar) }
                        }
                    }
                }
                .onDisappear { scope.cancel() }
        }
    }

    private func sheet(_ state: GimbalIndicatorState) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("Gimbal").font(.titleMedium)
                if state.gimbals.count > 1 {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 4) {
                            ForEach(state.gimbals) { gimbal in
                                CameraChip(label: gimbal.name, selected: gimbal.active) { act("gimbal.select", .number(Double(gimbal.managerCompid)), .number(Double(gimbal.deviceId))) }
                            }
                        }
                    }
                }
                Text(gimbalCellText(state)).font(.bodyMedium)
                if state.yawLockOffered {
                    wide(state.yawLockLabel) { act("gimbal.yawLock", .bool(!state.yawLocked)) }
                }
                wide("Center") { act("gimbal.center") }
                wide("Tilt 90") { act("gimbal.tilt90") }
                wide("Point home") { act("gimbal.pointHome") }
                if state.retractOffered {
                    wide("Retract") { act("gimbal.retract") }
                }
                if state.controlOffered {
                    Button { act("gimbal.control", .bool(!state.haveControl)) } label: {
                        Text(state.controlLabel).frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.filled)
                }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
                NavigationLink("Gimbal settings") { GimbalSettings() }
                    .padding(.vertical, Space.s2)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 20)
            .padding(.bottom, 24)
        }
    }

    private func wide(_ label: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) { Text(label).frame(maxWidth: .infinity) }.buttonStyle(.bordered)
    }

    private func act(_ path: String, _ args: JSON...) {
        scope.launch {
            let answer = await offMain { Qgc.call(path, arguments: args.map { $0 as Any? }) }
            guard !Task.isCancelled else { return }
            refusal = gimbalRefusal(answer)
            if refusal == nil { open = false }
        }
    }
}

private let GIMBAL_CONTROLLER_SETTINGS = "gimbalControllerSettings"
private let GIMBAL_SETTINGS_PAGE = "Gimbal Controller"
private let JOYSTICK_BUTTONS_SPEED = "joystickButtonsSpeed"

func joystickButtonsAvailable(_ view: JSON?) -> Bool {
    guard let view else { return false }
    return view.has("active") && view["vehicle"].bool && view["enabled"].bool
}

func gimbalSettingsBlocks(_ sections: [SettingsSectionRows], _ joystickButtons: Bool) -> [SettingsBlock] {
    sections.filter { $0.group == GIMBAL_CONTROLLER_SETTINGS }.flatMap(\.blocks).map { block in
        SettingsBlock(title: block.title, facts: block.facts.map { fact in
            fact.name == JOYSTICK_BUTTONS_SPEED && !joystickButtons
                ? withChanges(fact) { $0.enabled = false; $0.disabledReason = "No joystick is enabled for this vehicle." }
                : fact
        })
    }
}

private struct GimbalSettings: View {
    @Environment(\.theme) private var theme
    @QgcPath(JOYSTICK_VIEW) private var joystick
    @State private var reloads = 0
    @State private var sections: [SettingsSectionRows] = []

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(gimbalSettingsBlocks(sections, joystickButtonsAvailable(joystick)).enumerated()), id: \.offset) { _, block in
                    if !block.title.isBlank {
                        Text(sentenceCase(block.title))
                            .font(.titleMedium)
                            .padding(.horizontal, 24)
                            .padding(.vertical, 8)
                    }
                    ForEach(block.facts) { fact in
                        FactRow(fact: fact, onWrite: { reloads += 1 })
                    }
                }
            }
            .padding(.bottom, 24)
        }
        .navigationTitle("Gimbal settings")
        .task(id: reloads) {
            sections = await offMain { settingsSections(Qgc.get(settingsPagePath(GIMBAL_SETTINGS_PAGE))) }
        }
    }
}

let GIMBAL_DRAG_REPEAT_MS = 100

func screenFraction(_ x: CGFloat, _ y: CGFloat, _ width: CGFloat, _ height: CGFloat) -> (CGFloat, CGFloat) {
    ((x / width) * 2 - 1, -((y / height) * 2 - 1))
}

struct OnScreenGimbal: Equatable {
    let enabled: Bool
    let clickAndDrag: Bool
}

func onScreenGimbal(_ view: JSON?) -> OnScreenGimbal? {
    guard let view, view["shown"].bool, let onScreen = view["onScreen"].objectOrNil else { return nil }
    return OnScreenGimbal(enabled: onScreen["enabled"].bool, clickAndDrag: onScreen["clickAndDrag"].bool)
}

func aimFraction(_ delta: CGFloat, _ width: CGFloat) -> CGFloat { delta / (max(width, 1) / 2) }

func aimDelta(_ last: CGSize?, _ translation: CGSize) -> CGSize? {
    last.map { CGSize(width: translation.width - $0.width, height: translation.height - $0.height) }
}

private func sendOnScreen(_ pan: CGFloat, _ tilt: CGFloat, _ point: Bool) {
    let panning = Double(pan)
    let tilting = Double(tilt)
    offMain { _ = gimbalRefusal(Qgc.call("gimbal.onScreen", panning, tilting, point)) }
}

struct GimbalScreenControl: View {
    @QgcPath(GIMBAL_INDICATOR_PATH) private var view

    var body: some View {
        if let control = onScreenGimbal(view) {
            GeometryReader { geometry in
                Color.clear
                    .contentShape(Rectangle())
                    .modifier(GimbalAim(control: control, size: geometry.size))
            }
        }
    }
}

private struct GimbalAim: ViewModifier {
    let control: OnScreenGimbal
    let size: CGSize
    @State private var last: CGSize?
    @State private var start: (CGFloat, CGFloat)?
    @State private var latest: CGPoint = .zero
    @State private var repeating: Task<Void, Never>?
    @GestureState private var pressed = false

    func body(content: Content) -> some View {
        aimed(content)
            .onChange(of: pressed) { _, now in if !now { stop() } }
            .onDisappear(perform: stop)
    }

    private func stop() {
        repeating?.cancel()
        repeating = nil
        start = nil
        last = nil
    }

    @ViewBuilder private func aimed(_ content: Content) -> some View {
        if !control.enabled {
            content.gesture(
                DragGesture()
                    .updating($pressed) { _, pressed, _ in pressed = true }
                    .onChanged { drag in
                        let moved = aimDelta(last, drag.translation)
                        last = drag.translation
                        guard let moved else { return }
                        sendOnScreen(aimFraction(moved.width, size.width), -aimFraction(moved.height, size.width), false)
                    }
                    .onEnded { _ in stop() }
            )
        } else if !control.clickAndDrag {
            content.gesture(
                SpatialTapGesture().onEnded { tap in
                    let (pan, tilt) = screenFraction(tap.location.x, tap.location.y, size.width, size.height)
                    sendOnScreen(pan, tilt, true)
                }
            )
        } else {
            content.gesture(
                DragGesture(minimumDistance: 0)
                    .updating($pressed) { _, pressed, _ in pressed = true }
                    .onChanged { drag in
                        latest = drag.location
                        guard start == nil else { return }
                        start = screenFraction(drag.startLocation.x, drag.startLocation.y, size.width, size.height)
                        repeating = Task { @MainActor in
                            while !Task.isCancelled {
                                try? await Task.sleep(for: .milliseconds(GIMBAL_DRAG_REPEAT_MS))
                                guard !Task.isCancelled, let from = start else { return }
                                let now = screenFraction(latest.x, latest.y, size.width, size.height)
                                sendOnScreen(now.0 - from.0, now.1 - from.1, false)
                            }
                        }
                    }
                    .onEnded { _ in stop() }
            )
        }
    }
}
