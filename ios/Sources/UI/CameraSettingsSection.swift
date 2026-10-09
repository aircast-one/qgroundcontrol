import SwiftUI

let CAMERA_SETTINGS_VIEW = "view.cameraSettings"
let CAMERA_SETTING_SET = "cameraSettings.set"
private let CAMERA_SETTINGS_POLL_MS = 1000
private let CAMERA_SLIDER_MAX_INTERVALS = 1001.0
private let CAMERA_CONTROL_WIDTH: CGFloat = 180
private let CAMERA_SETTINGS_SPACING: CGFloat = 6
private let CAMERA_SETTINGS_INSET: CGFloat = 20

struct CameraOption: Equatable {
    let label: String
    let value: JSON
}

enum CameraSettingControl: Equatable {
    case Toggle(on: Bool)
    case Choice(options: [CameraOption], selected: Int)
    case Range(min: Double, max: Double, step: Double, value: Double)
    case Entry(text: String)
}

struct CameraSetting: Equatable, Identifiable {
    let name: String
    let label: String
    let readOnly: Bool
    let control: CameraSettingControl

    var id: String { name }
}

private func number(_ parameter: JSON, _ key: String) -> Double? {
    parameter.has(key) ? parameter[key].double.flatMap { $0.isNaN ? nil : $0 } : nil
}

func cameraSettingControl(_ parameter: JSON) -> CameraSettingControl {
    let listed = parameter["options"].objects.map { CameraOption(label: $0["label"].string, value: $0["value"]) }
    let value = parameter["value"]
    let step = number(parameter, "step")
    let min = number(parameter, "min")
    let max = number(parameter, "max")
    if parameter["isBool"].bool {
        return .Toggle(on: value == .bool(true) || value.numberOrNil.map { $0.rounded(.towardZero) == 1 } == true)
    }
    if !listed.isEmpty { return .Choice(options: listed, selected: parameter["selected"].int(-1)) }
    if let step, let min, let max { return .Range(min: min, max: max, step: step, value: value.numberOrNil ?? min) }
    return .Entry(text: value.isNull ? "" : value.stringOrNil ?? value.text)
}

func cameraSliderStep(_ min: Double, _ max: Double, _ step: Double) -> Double? {
    let ratio = ((max - min) / step).rounded(.towardZero)
    let intervals = ratio.isNaN ? 0 : Swift.min(ratio, CAMERA_SLIDER_MAX_INTERVALS)
    return max > min && intervals >= 2 ? (max - min) / intervals : nil
}

func cameraSettings(_ view: JSON?) -> [CameraSetting] {
    guard let view, view["state"].string == "ready",
          let parameters = view["parameters"].arrayOrNil,
          let active = view["activeSettings"].arrayOrNil else { return [] }
    let byName = Dictionary(parameters.filter { $0.object != nil }.map { ($0["name"].string, $0) }, uniquingKeysWith: { _, last in last })
    return active.compactMap { byName[$0.string] }.map { p in
        CameraSetting(name: p["name"].string, label: p["description"].string, readOnly: p["readOnly"].bool, control: cameraSettingControl(p))
    }
}

struct CameraDefinitionSettings: View {
    @Environment(\.theme) private var theme
    @State private var settings: [CameraSetting] = []
    @State private var refusal: String?
    @State private var scope = ViewScope()

    var body: some View {
        VStack(alignment: .leading, spacing: CAMERA_SETTINGS_SPACING) {
            ForEach(settings) { setting in
                CameraSettingRow(setting: setting) { write(setting.name, $0) }
            }
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .padding(.horizontal, CAMERA_SETTINGS_INSET)
        .task {
            while !Task.isCancelled {
                settings = await offMain { cameraSettings(Qgc.get(CAMERA_SETTINGS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(CAMERA_SETTINGS_POLL_MS))
            }
        }
        .onDisappear { scope.cancel() }
    }

    private func write(_ name: String, _ value: JSON) {
        scope.launch {
            let refused = await offMain { Qgc.refusalOf(CAMERA_SETTING_SET, name, value) }
            guard !Task.isCancelled else { return }
            refusal = refused
        }
    }
}

private struct CameraSettingRow: View {
    let setting: CameraSetting
    let onWrite: (JSON) -> Void

    var body: some View {
        HStack {
            Text(setting.label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
            control.disabled(setting.readOnly)
        }
    }

    @ViewBuilder private var control: some View {
        switch setting.control {
        case .Toggle(let on):
            Toggle("", isOn: Binding(get: { on }, set: { onWrite(.number($0 ? 1 : 0)) })).labelsHidden()
        case .Choice(let options, let selected):
            Menu {
                ForEach(Array(options.enumerated()), id: \.offset) { _, option in
                    Button(option.label) { onWrite(option.value) }
                }
            } label: {
                Text(options.indices.contains(selected) ? options[selected].label : "")
            }
            .buttonStyle(.bordered)
        case .Range(let min, let max, let step, let value):
            CameraRangeControl(min: min, max: max, step: step, value: value) { onWrite(.number($0)) }
        case .Entry(let text):
            CameraEntryControl(text: text) { typed in onWrite(Double(typed).map(JSON.number) ?? .string(typed)) }
        }
    }
}

private struct CameraRangeControl: View {
    let min: Double
    let max: Double
    let step: Double
    let value: Double
    let onWrite: (Double) -> Void
    @State private var dragged: Double?

    var body: some View {
        let shown = Binding(get: { dragged ?? value }, set: { dragged = $0 })
        let finished: (Bool) -> Void = { editing in if !editing { onWrite(shown.wrappedValue) } }
        Group {
            if let stepped = cameraSliderStep(min, max, step) {
                Slider(value: shown, in: min...max, step: stepped, onEditingChanged: finished)
            } else {
                Slider(value: shown, in: min...Swift.max(max, min + 1), onEditingChanged: finished)
            }
        }
        .frame(width: CAMERA_CONTROL_WIDTH)
        .onChange(of: value) { dragged = nil }
    }
}

private struct CameraEntryControl: View {
    let text: String
    let onWrite: (String) -> Void
    @State private var typed = ""

    var body: some View {
        TextField("", text: $typed)
            .textFieldStyle(.roundedBorder)
            .submitLabel(.done)
            .onSubmit { onWrite(typed) }
            .frame(width: CAMERA_CONTROL_WIDTH)
            .onChange(of: text, initial: true) { typed = text }
    }
}
