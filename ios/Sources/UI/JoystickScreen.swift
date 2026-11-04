import SwiftUI

let JOYSTICK_VIEW = "view.joystick"
let JOYSTICK_SETUP_VIEW = "view.joystickSetup"
let JOYSTICK_SCREEN = "joystick"
let JOYSTICK_SELECT = "joystick.select"
let JOYSTICK_ENABLE = "joystick.enable"
let JOYSTICK_SETTING = "joystick.setting"
let JOYSTICK_CALIBRATION = "joystick.calibration"
let JOYSTICK_BUTTON_ACTION = "joystick.buttonAction"
let JOYSTICK_BUTTON_REPEAT = "joystick.buttonRepeat"
let NO_ACTION = "No Action"
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
    let label: String
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

struct StatusLines: Equatable {
    let bullets: Bool
    let lines: [String]
}

func statusLines(_ json: JSON?) -> StatusLines {
    StatusLines(bullets: json?["bullets"].bool == true, lines: json?["lines"].strings ?? [])
}

struct JoystickCalibration: Equatable {
    let calibrating: Bool
    let status: StatusLines
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
        status: statusLines(json?["status"]),
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
    var standbyText: String?
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
        axes: state["axes"].objects.compactMap { a in JoystickAxis(index: a["index"].int(0), raw: a["raw"].isNull ? nil : a["raw"].int(0), label: a["label"].string) },
        armed: it["armed"].bool,
        calibration: joystickCalibration(it["calibration"].objectOrNil),
        transmitterMode: it["transmitterMode"].int(2),
        buttons: state["buttons"].objects.compactMap { b in
            JoystickButton(index: b["index"].int(0), action: b["action"].isNull ? NO_ACTION : b["action"].string, repeat: b["repeat"].bool, pressed: ["down", "repeat"].contains(b["event"].string))
        },
        actions: it["assignableActions"].objects.compactMap { AssignableAction(action: $0["action"].string, canRepeat: $0["canRepeat"].bool) },
        standbyText: it["standby"]["text"].string.isEmpty ? nil : it["standby"]["text"].string
    )
}

private let BASIC_SETTINGS = ["standby", "throttleModeCenterZero", "throttleSmoothing", "exponentialPct", "negativeThrust"]
private let ADVANCED_SETTINGS = ["circleCorrection", "axisFrequencyHz", "buttonFrequencyHz", "useDeadband"]
private let EXTENSION_SETTINGS = ["enableManualControlPitchExtension", "enableManualControlRollExtension"]
private let ADDITIONAL_SETTINGS = (1...6).map { "enableAdditionalAxis\($0)" }

struct JoystickScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(JOYSTICK_SETUP_VIEW) private var view
    @State private var refusal: String?
    @State private var advanced = false
    @State private var offerEnable: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let read = joystickPage(view) {
                if read.names.isEmpty || read.active == nil {
                    NativeForm {
                        EmptyState(icon: .gamepad, title: "No joystick", text: "No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB.")
                            .listRowBackground(Color.clear)
                    }
                } else {
                    screen(read)
                }
            }
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
        let additionalAxes = setting("additionalAxesFunction")
        let viaRc = additionalAxes == nil || additionalAxes?.value.numberOrNil.map { $0.rounded(.towardZero) } == 1
        return NativeForm {
            Section {
                if read.names.count > 1 {
                    JoystickPicker(page: read) { act(JOYSTICK_SELECT, $0) }
                }
                Toggle("Enable", isOn: Binding(get: { read.enabled }, set: { act(JOYSTICK_ENABLE, $0) }))
                    .disabled(!read.vehicle)
                LabeledContent("Calibration") {
                    Text(read.calibrated ? "Calibrated" : "Requires calibration")
                        .foregroundStyle(read.calibrated ? Color.secondary : theme.colors.error)
                }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            } footer: {
                if !read.vehicle { Text("Not currently available") }
            }

            if !read.armed {
                Section {
                    CalibrationPanel(page: read) { calibrate($0, read.enabled) }
                    TransmitterModeRow(mode: read.transmitterMode) { act(JOYSTICK_SETTING, "transmitterMode", $0) }
                } header: {
                    SectionHeader(text: "Calibration")
                }
            }

            Section {
                ForEach(read.axes, id: \.index) { axis in
                    HStack(spacing: 8) {
                        Text(axis.label).frame(width: 96, alignment: .leading)
                        ProgressView(value: Double(min(max((Float(axis.raw ?? 0) / AXIS_RANGE + 1) / 2, 0), 1)))
                            .frame(maxWidth: .infinity)
                    }
                }
            } header: {
                SectionHeader(text: "Axis monitor")
            }

            Section {
                ForEach(read.buttons, id: \.index) { button in
                    ButtonRow(
                        button: button,
                        actions: read.actions,
                        calibrated: read.calibrated,
                        onAction: { act(JOYSTICK_BUTTON_ACTION, button.index, $0) },
                        onRepeat: { act(JOYSTICK_BUTTON_REPEAT, button.index, $0) }
                    )
                }
            } header: {
                SectionHeader(text: "Buttons")
            } footer: {
                Text("Multiple buttons that have the same action must be pressed simultaneously to invoke the action.")
            }

            Section {
                ForEach(BASIC_SETTINGS.compactMap(setting), id: \.name) { each in
                    SettingRow(setting: each) { act(JOYSTICK_SETTING, each.name, $0) }
                }
                AdvancedToggle(open: advanced, label: "Advanced settings") { advanced.toggle() }
            } header: {
                SectionHeader(text: "Settings")
            } footer: {
                if let standby = read.standbyText { Text(standby) }
            }
            if advanced {
                Section {
                    ForEach(ADVANCED_SETTINGS.compactMap(setting), id: \.name) { each in
                        SettingRow(setting: each) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                } header: {
                    SectionHeader(text: "Advanced")
                } footer: {
                    if setting("useDeadband")?.value == .bool(true) {
                        Text("Deadband can be set during the first step of calibration by gently wiggling each axis. ")
                    }
                }
                Section {
                    ForEach(Array(zip(EXTENSION_SETTINGS.compactMap(setting), ["Pitch", "Roll"])), id: \.0.name) { each, label in
                        SettingRow(setting: relabeled(each, label)) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                } header: {
                    SectionHeader(text: "MANUAL_CONTROL Extensions")
                }
                Section {
                    if additionalAxes != nil {
                        Picker("Send using", selection: Binding(get: { viaRc }, set: { rc in
                            if rc != viaRc { act(JOYSTICK_SETTING, "additionalAxesFunction", rc ? 1 : 0) }
                        })) {
                            Text("MANUAL_CONTROL").tag(false)
                            Text("RC_CHANNELS_OVERRIDE").tag(true)
                        }
                    }
                    ForEach(Array(ADDITIONAL_SETTINGS.compactMap(setting).enumerated()), id: \.element.name) { index, each in
                        SettingRow(setting: relabeled(each, viaRc ? "Channel \(index + 5)" : "Aux\(index + 1)")) { act(JOYSTICK_SETTING, each.name, $0) }
                    }
                } header: {
                    SectionHeader(text: "Additional axes")
                }
            }
        }
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                if !read.armed {
                    Button(read.calibration.nextText) { calibrate("next", read.enabled) }
                        .disabled(!read.calibration.nextEnabled)
                }
            }
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
        HStack(spacing: Space.s3) {
            Text(String(button.index))
                .fontWeight(.semibold)
                .monospacedDigit()
                .foregroundStyle(button.pressed ? theme.colors.primary : Color.primary)
                .frame(width: 32, alignment: .leading)
            Menu {
                ForEach(Array(actions.enumerated()), id: \.offset) { _, option in
                    Button(option.action) { onAction(option.action) }
                }
            } label: {
                Text(button.action).lineLimit(1)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Toggle("Repeat", isOn: Binding(get: { button.repeat }, set: onRepeat))
                .font(.subheadline)
                .fixedSize()
                .disabled(!(canRepeat && calibrated))
        }
    }
}

// The joystick calibration steps: the live sticks while calibrating, the step's instructions, One-Sided and Cancel
// (Calibrate / Next is the page's primary action in the navigation bar).
private struct CalibrationPanel: View {
    let page: JoystickPage
    let onStep: (String) -> Void

    var body: some View {
        let cal = page.calibration
        if cal.calibrating {
            StickDiagram(positions: cal.stickPositions, single: cal.singleStick)
                .frame(maxWidth: .infinity)
                .padding(.vertical, Space.s2)
        }
        if !cal.status.lines.isEmpty {
            VStack(alignment: .leading, spacing: Space.s1) {
                ForEach(Array(cal.status.lines.enumerated()), id: \.offset) { _, line in
                    Text(cal.status.bullets ? "\u{2022} \(line)" : line)
                }
            }
        }
        if cal.oneSidedVisible {
            Button("One-Sided") { onStep("oneSided") }
        }
        if cal.cancelEnabled {
            Button("Cancel calibration") { onStep("cancel") }
        }
    }
}

private struct TransmitterModeRow: View {
    let mode: Int
    let onPick: (Int) -> Void

    var body: some View {
        Picker("Mode", selection: Binding(get: { mode }, set: { picked in if picked != mode { onPick(picked) } })) {
            ForEach(1...4, id: \.self) { m in
                Text("Mode \(m)").tag(m)
            }
        }
    }
}

private struct JoystickPicker: View {
    let page: JoystickPage
    let onPick: (String) -> Void

    var body: some View {
        Picker("Joystick", selection: Binding(get: { page.active ?? "" }, set: { picked in if picked != page.active { onPick(picked) } })) {
            ForEach(page.names, id: \.self) { name in
                Text(name).tag(name)
            }
        }
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
        if setting.type == "bool" {
            Toggle(setting.label, isOn: Binding(get: { setting.value == .bool(true) }, set: { onChange($0) }))
        } else {
            LabeledContent(setting.label) {
                HStack(spacing: 4) {
                    TextField(setting.label, text: $typed)
                        .labelsHidden()
                        .multilineTextAlignment(.trailing)
                        .keyboardType(.numbersAndPunctuation)
                        .submitLabel(.done)
                        .onSubmit { if let number = Double(typed) { onChange(number) } }
                    if !setting.units.isBlank {
                        Text(setting.units).foregroundStyle(Color.secondary)
                    }
                }
            }
            .onChange(of: setting.value, initial: true) { typed = setting.value.numberOrNil.map(JSON.format) ?? "" }
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
