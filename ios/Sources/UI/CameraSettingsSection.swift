import SwiftUI

let CAMERA_SETTINGS_VIEW = "view.cameraSettings"
let CAMERA_SETTING_SET = "cameraSettings.set"
private let CAMERA_SETTINGS_POLL_MS = 1000

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

private func numeric(_ value: JSON) -> Double? {
    if case .number(let number) = value { return number }
    return nil
}

func cameraSettingControl(_ parameter: JSON) -> CameraSettingControl {
    let listed = parameter["options"].array.filter { $0.object != nil }.map { CameraOption(label: $0["label"].string, value: $0["value"]) }
    let value = parameter["value"]
    let step = number(parameter, "step")
    let min = number(parameter, "min")
    let max = number(parameter, "max")
    if parameter["isBool"].bool {
        return .Toggle(on: value == .bool(true) || numeric(value).map { $0.isFinite && Int($0) == 1 } == true)
    }
    if !listed.isEmpty { return .Choice(options: listed, selected: parameter["selected"].int(-1)) }
    if let step, let min, let max { return .Range(min: min, max: max, step: step, value: numeric(value) ?? min) }
    return .Entry(text: value.isNull ? "" : value.stringOrNil ?? value.text)
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

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(settings) { setting in
                CameraSettingRow(setting: setting) { write(setting.name, $0) }
            }
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .padding(.horizontal, 20)
        .task {
            while !Task.isCancelled {
                settings = await offMain { cameraSettings(Qgc.get(CAMERA_SETTINGS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(CAMERA_SETTINGS_POLL_MS))
            }
        }
    }

    private func write(_ name: String, _ value: JSON) {
        Task { refusal = await offMain { Qgc.refusalOf(CAMERA_SETTING_SET, name, value) } }
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
        let steps = Int((max - min) / step) - 1
        Group {
            if max > min, step > 0, (0...1000).contains(steps) {
                Slider(value: shown, in: min...max, step: step, onEditingChanged: finished)
            } else {
                Slider(value: shown, in: min...Swift.max(max, min + 1), onEditingChanged: finished)
            }
        }
        .frame(width: 180)
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
            .frame(width: 180)
            .onChange(of: text, initial: true) { typed = text }
    }
}
