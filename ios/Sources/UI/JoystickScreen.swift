import SwiftUI

let JOYSTICK_VIEW = "view.joystick"
let JOYSTICK_SCREEN = "joystick"
let JOYSTICK_SELECT = "joystick.select"
let JOYSTICK_ENABLE = "joystick.enable"
let JOYSTICK_SETTING = "joystick.setting"
let JOYSTICK_CALIBRATION = "joystick.calibration"
let JOYSTICK_BUTTON_ACTION = "joystick.buttonAction"
let JOYSTICK_BUTTON_REPEAT = "joystick.buttonRepeat"
let NO_ACTION = "No Action"
private let JOYSTICK_POLL_MS = 100
private let AXIS_RANGE: Float = 32767

struct JoystickSetting: Equatable {
    let name: String
    let type: String
    var label: String
    let units: String
    let value: JSON
    var slider: FactSlider? = nil
}

struct JoystickAxis: Equatable {
    let index: Int
    let raw: Int?
    let function: String
}

struct JoystickButton: Equatable {
    let index: Int
    let action: String
    let `repeat`: Bool
    let pressed: Bool
}

struct AssignableAction: Equatable {
    let action: String
    let canRepeat: Bool
}

struct JoystickCalibration: Equatable {
    let calibrating: Bool
    let statusText: String
    let nextText: String
    let nextEnabled: Bool
    let cancelEnabled: Bool
    let oneSidedVisible: Bool
    var stickPositions: [Int] = [0, 0, 0, 0]
    var singleStick: Bool = false
}

func joystickCalibration(_ json: JSON?) -> JoystickCalibration {
    let positions = json?["stickPositions"]
    return JoystickCalibration(
        calibrating: json?["calibrating"].bool == true,
        statusText: json?["statusText"].string ?? "",
        nextText: (json?["nextText"].string ?? "").ifBlank("Calibrate"),
        nextEnabled: json?["nextEnabled"].bool(true) ?? true,
        cancelEnabled: json?["cancelEnabled"].bool == true,
        oneSidedVisible: json?["oneSidedVisible"].bool == true,
        stickPositions: positions?.arrayOrNil != nil ? (0..<4).map { positions?[$0].int(0) ?? 0 } : [0, 0, 0, 0],
        singleStick: json?["singleStickDisplay"].bool == true
    )
}

struct JoystickPage: Equatable {
    let names: [String]
    let active: String?
    let vehicle: Bool
    let enabled: Bool
    let calibrated: Bool
    let settings: [JoystickSetting]
    let axes: [JoystickAxis]
    var armed: Bool = false
    var calibration: JoystickCalibration = joystickCalibration(nil)
    var transmitterMode: Int = 2
    var buttons: [JoystickButton] = []
    var actions: [AssignableAction] = []
}

func joystickPage(_ view: JSON?) -> JoystickPage? {
    guard let it = view, it["available"].bool else { return nil }
    let state = it["state"]
    return JoystickPage(
        names: it["names"].strings,
        active: it["active"].isNull ? nil : it["active"].string,
        vehicle: it["vehicle"].bool,
        enabled: it["enabled"].bool,
        calibrated: it["calibrated"].bool,
        settings: it["settings"].objects.compactMap { s in
            s["visible"].bool(true)
                ? JoystickSetting(name: s["name"].string, type: s["type"].string, label: s["label"].string, units: s["units"].string, value: s["value"], slider: s["slider"].object != nil ? factSlider(s["slider"], "") : nil)
                : nil
        },
        axes: state["axes"].objects.compactMap { a in JoystickAxis(index: a["index"].int(0), raw: a["raw"].isNull ? nil : a["raw"].int(0), function: a["function"].string) },
        armed: it["armed"].bool,
        calibration: joystickCalibration(it["calibration"].objectOrNil),
        transmitterMode: it["transmitterMode"].int(2),
        buttons: state["buttons"].objects.compactMap { b in
            JoystickButton(index: b["index"].int(0), action: b["action"].isNull ? NO_ACTION : b["action"].string, repeat: b["repeat"].bool, pressed: ["down", "repeat"].contains(b["event"].string))
        },
        actions: it["assignableActions"].objects.compactMap { AssignableAction(action: $0["action"].string, canRepeat: $0["canRepeat"].bool) }
    )
}

private let BASIC_SETTINGS = ["throttleModeCenterZero", "throttleSmoothing", "exponentialPct", "negativeThrust"]
private let ADVANCED_SETTINGS = ["circleCorrection", "axisFrequencyHz", "buttonFrequencyHz", "useDeadband"]
private let EXTENSION_SETTINGS = ["enableManualControlPitchExtension", "enableManualControlRollExtension"]
private let ADDITIONAL_SETTINGS = (1...6).map { "enableAdditionalAxis\($0)" }

struct JoystickScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var page: JoystickPage?
    @State private var refusal: String?
    @State private var advanced = false
    @State private var offerEnable: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let read = page {
                if read.names.isEmpty || read.active == nil {
                    EmptyState(icon: .gamepad, title: "No joystick", text: "No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB.")
                } else {
                    screen(read)
                }
            }
        }
        .task(id: revision) {
            let read = await offMain { joystickPage(Qgc.get(JOYSTICK_VIEW)) }
            page = read
            try? await Task.sleep(for: .milliseconds(JOYSTICK_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
        .alert("Enable joystick", isPresented: Binding(get: { offerEnable != nil }, set: { if !$0 { offerEnable = nil } }), presenting: offerEnable) { _ in
            Button("Yes") {
                offerEnable = nil
                act(JOYSTICK_ENABLE, true)
            }
            Button("No", role: .cancel) { offerEnable = nil }
        } message: { name in
            Text("\(name) calibration is complete. Enable it now?")
        }
    }

    private func screen(_ read: JoystickPage) -> some View {
        let setting = { (name: String) in read.settings.first { $0.name == name } }
        let viaRc = setting("additionalAxesFunction")?.value.numberOrNil.map { $0.rounded(.towardZero) } == 1
        return ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                if read.names.count > 1 {
                    JoystickPicker(page: read) { act(JOYSTICK_SELECT, $0) }
                }
                HStack(spacing: 12) {
                    Toggle("Enable", isOn: Binding(get: { read.enabled }, set: { act(JOYSTICK_ENABLE, $0) }))
                        .fixedSize()
                        .disabled(!read.vehicle)
                    if !read.vehicle {
                        Text("Not currently available").foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                Text(read.calibrated ? "Calibrated" : "Requires calibration")
                    .foregroundStyle(read.calibrated ? theme.colors.onSurface : theme.colors.error)
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }

                if !read.armed {
                    SectionHeader(text: "Calibration")
                    CalibrationPanel(page: read) { calibrate($0, read.enabled) }
                    TransmitterModeRow(mode: read.transmitterMode) { act(JOYSTICK_SETTING, "transmitterMode", $0) }
                }

                SectionHeader(text: "Axis monitor")
                ForEach(read.axes, id: \.index) { axis in
                    HStack(spacing: 8) {
                        Text(axis.function.ifBlank("Axis \(axis.index + 1)")).frame(width: 96, alignment: .leading)
                        ProgressView(value: Double(min(max((Float(axis.raw ?? 0) / AXIS_RANGE + 1) / 2, 0), 1)))
                            .frame(maxWidth: .infinity)
                    }
                }

                SectionHeader(text: "Buttons")
                Text("Multiple buttons that have the same action must be pressed simultaneously to invoke the action.").font(.bodySmall)
                ForEach(read.buttons, id: \.index) { button in
                    ButtonRow(
                        button: button,
                        actions: read.actions,
                        calibrated: read.calibrated,
                        onAction: { act(JOYSTICK_BUTTON_ACTION, button.index, $0) },
                        onRepeat: { act(JOYSTICK_BUTTON_REPEAT, button.index, $0) }
                    )
                }

                SectionHeader(text: "Settings")
                ForEach(BASIC_SETTINGS.compactMap(setting), id: \.name) { each in
                    SettingRow(setting: each) { act(JOYSTICK_SETTING, each.name, $0) }
                }
                Button("Advanced settings") { advanced.toggle() }.buttonStyle(.bordered)
                if advanced {
                    ForEach(ADVANCED_SETTINGS.compactMap(setting), id: \.name) { each in
                        SettingRow(setting: each) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                    if setting("useDeadband")?.value == .bool(true) {
                        Text("Deadband can be set during the first step of calibration by gently wiggling each axis. ").font(.bodySmall)
                    }
                    Text("MANUAL_CONTROL Extensions").font(.titleSmall)
                    ForEach(Array(zip(EXTENSION_SETTINGS.compactMap(setting), ["Pitch", "Roll"])), id: \.0.name) { each, label in
                        SettingRow(setting: relabeled(each, label)) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                    Text("Additional axes").font(.titleSmall)
                    RadioChoiceRow(label: "Send using MANUAL_CONTROL", selected: !viaRc) { act(JOYSTICK_SETTING, "additionalAxesFunction", 0) }
                    RadioChoiceRow(label: "Send using RC_CHANNELS_OVERRIDE", selected: viaRc) { act(JOYSTICK_SETTING, "additionalAxesFunction", 1) }
                    ForEach(Array(ADDITIONAL_SETTINGS.compactMap(setting).enumerated()), id: \.element.name) { index, each in
                        SettingRow(setting: relabeled(each, viaRc ? "Channel \(index + 5)" : "Aux\(index + 1)")) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        }
    }

    private func calibrate(_ op: String, _ enabled: Bool) {
        Task {
            let answer = await offMain { Qgc.call(JOYSTICK_CALIBRATION, op) }
            refusal = answer.flatMap { $0["ok"].bool ? nil : $0["reason"].string }
            if answer?["completed"].bool == true && !enabled { offerEnable = answer?["name"].string }
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task { refusal = await offMain { Qgc.refusalOf(path, arguments: args) } }
    }
}

private func relabeled(_ setting: JoystickSetting, _ label: String) -> JoystickSetting {
    var copy = setting
    copy.label = label
    return copy
}

private struct ButtonRow: View {
    let button: JoystickButton
    let actions: [AssignableAction]
    let calibrated: Bool
    let onAction: (String) -> Void
    let onRepeat: (Bool) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let canRepeat = actions.first { $0.action == button.action }?.canRepeat == true
        HStack(spacing: 8) {
            Text(String(button.index))
                .font(.titleSmall)
                .foregroundStyle(button.pressed ? theme.colors.primary : theme.colors.onSurface)
                .frame(width: 32, alignment: .leading)
            Menu {
                ForEach(Array(actions.enumerated()), id: \.offset) { _, option in
                    Button(option.action) { onAction(option.action) }
                }
            } label: {
                Text(button.action).lineLimit(1).frame(maxWidth: .infinity)
            }
            .buttonStyle(.bordered)
            .frame(maxWidth: .infinity)
            Toggle("", isOn: Binding(get: { button.repeat }, set: onRepeat))
                .labelsHidden()
                .disabled(!(canRepeat && calibrated))
            Text("Repeat").font(.bodySmall)
        }
    }
}

private struct CalibrationPanel: View {
    let page: JoystickPage
    let onStep: (String) -> Void

    var body: some View {
        let cal = page.calibration
        VStack(alignment: .leading, spacing: 8) {
            if cal.calibrating {
                StickDiagram(positions: cal.stickPositions, single: cal.singleStick)
            }
            if !cal.statusText.isBlank {
                Text(cal.statusText).font(.bodyMedium)
            }
            HStack(spacing: 8) {
                Button("Cancel") { onStep("cancel") }.buttonStyle(.bordered).disabled(!cal.cancelEnabled)
                if cal.oneSidedVisible {
                    Button("One-Sided") { onStep("oneSided") }.buttonStyle(.bordered)
                }
                Button(cal.nextText) { onStep("next") }.buttonStyle(.filled).disabled(!cal.nextEnabled)
            }
        }
    }
}

private struct TransmitterModeRow: View {
    let mode: Int
    let onPick: (Int) -> Void

    var body: some View {
        HStack(spacing: 12) {
            Text("Mode").frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(1...4, id: \.self) { m in
                    Button("Mode \(m)") { onPick(m) }
                }
            } label: {
                Text("Mode \(mode)")
            }
            .buttonStyle(.bordered)
        }
    }
}

private struct JoystickPicker: View {
    let page: JoystickPage
    let onPick: (String) -> Void

    var body: some View {
        Menu {
            ForEach(Array(page.names.enumerated()), id: \.offset) { _, name in
                Button(name) { onPick(name) }
            }
        } label: {
            Text(page.active ?? "")
        }
        .buttonStyle(.bordered)
    }
}

private struct SettingRow: View {
    let setting: JoystickSetting
    let onChange: (Any) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            SettingField(setting: setting, onChange: onChange)
            if let slider = setting.slider, setting.type != "bool" {
                FieldSlider(value: setting.value.numberOrNil, slider: slider, enabled: true) { onChange($0) }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct SettingField: View {
    let setting: JoystickSetting
    let onChange: (Any) -> Void
    @State private var typed = ""

    var body: some View {
        HStack {
            Text(setting.label).frame(maxWidth: .infinity, alignment: .leading)
            if setting.type == "bool" {
                Toggle("", isOn: Binding(get: { setting.value == .bool(true) }, set: { onChange($0) })).labelsHidden()
            } else {
                HStack(spacing: 4) {
                    TextField("", text: $typed)
                        .keyboardType(.numbersAndPunctuation)
                        .submitLabel(.done)
                        .onSubmit { if let number = Double(typed) { onChange(number) } }
                    Text(setting.units)
                }
                .textFieldStyle(.roundedBorder)
                .frame(width: 140)
                .onChange(of: setting.value, initial: true) { typed = setting.value.numberOrNil.map(JSON.format) ?? "" }
            }
        }
    }
}

private let STICK_BOX: CGFloat = 72

struct StickDiagram: View {
    let positions: [Int]
    let single: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        let pairs = stride(from: 0, to: positions.count - 1, by: 2).map { (positions[$0], positions[$0 + 1]) }
        HStack(spacing: 24) {
            ForEach(Array(pairs.prefix(single ? 1 : 2).enumerated()), id: \.offset) { _, pair in
                Canvas { context, size in
                    let half = min(size.width, size.height) / 2
                    context.stroke(Path(CGRect(origin: .zero, size: size).insetBy(dx: 0.5, dy: 0.5)), with: .color(theme.colors.outline), lineWidth: 1)
                    let radius = half * 0.18
                    let knob = CGPoint(x: size.width / 2 + CGFloat(pair.0) * half * 0.75, y: size.height / 2 - CGFloat(pair.1) * half * 0.75)
                    context.fill(Path(ellipseIn: CGRect(x: knob.x - radius, y: knob.y - radius, width: radius * 2, height: radius * 2)), with: .color(theme.colors.primary))
                }
                .frame(width: STICK_BOX, height: STICK_BOX)
            }
        }
    }
}
