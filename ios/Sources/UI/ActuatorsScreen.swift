import SwiftUI

let ACTUATOR_OUTPUTS_VIEW = "view.actuatorOutputs"
let ACTUATORS_SCREEN = "actuators"
private let SHOW_BITSET = "bitset"
private let SHOW_TRUE_IF_POSITIVE = "true-if-positive"

struct ActuatorFact: Equatable {
    var label: String
    let showAs: String
    let bit: Int
    let advanced: Bool
    let fact: Fact
}

struct ActuatorColumn: Equatable {
    let label: String
    let advanced: Bool
    let visible: Bool
}

struct ActuatorChannel: Equatable {
    let label: String
    let configs: [ActuatorFact?]
}

struct ActuatorSubgroup: Equatable {
    let label: String
    let primary: ActuatorFact?
    let params: [ActuatorFact]
    let columns: [ActuatorColumn]
    let channels: [ActuatorChannel]
}

struct ActuatorGroup: Equatable {
    let label: String
    let enable: ActuatorFact?
    let groupsVisible: Bool
    let params: [ActuatorFact]
    let subgroups: [ActuatorSubgroup]
    var notes: [String] = []
}

enum GeometryCell: Equatable {
    case Editable(item: ActuatorFact, param: String, channelFunction: Int, hidden: Bool, disabled: Bool)
    case Fixed(label: String, valueString: String, advanced: Bool, hidden: Bool = false)
    case Axis(options: [String], index: Int, params: [String], advanced: Bool, hidden: Bool, disabled: Bool)
    case Unavailable(label: String, advanced: Bool, hidden: Bool)

    var hidden: Bool {
        switch self {
        case .Editable(_, _, _, let hidden, _), .Fixed(_, _, _, let hidden), .Axis(_, _, _, _, let hidden, _), .Unavailable(_, _, let hidden): hidden
        }
    }
}

let PARAM_NOT_AVAILABLE = "(Param not available)"

let ACTUATOR_MIXER_SET = "actuatorMixer.set"
let ACTUATOR_MIXER_AXIS = "actuatorMixer.setAxis"

struct GeometryChannel: Equatable {
    let label: String
    let cells: [GeometryCell?]
}

struct GeometryGroup: Equatable {
    let label: String
    let count: Fact?
    let channels: [GeometryChannel]
    let params: [ActuatorFact]
}

struct Geometry: Equatable {
    let title: String
    let helpUrl: String
    let groups: [GeometryGroup]
    var motors: [GeometryMotor] = []
}

struct MotorAssignmentState: Equatable {
    var multirotor = false
    var enabled = false
    var active = false
    var message = ""
    var highlighted: Set<Int> = []
}

let MOTOR_ASSIGNMENT_INIT = "motorAssignment.init"
let MOTOR_ASSIGNMENT_START = "motorAssignment.start"
let MOTOR_ASSIGNMENT_SELECT = "motorAssignment.selectMotor"
let MOTOR_ASSIGNMENT_SPIN = "motorAssignment.spinCurrentMotor"
let MOTOR_ASSIGNMENT_ABORT = "motorAssignment.abort"
private let ASSIGNMENT_POLL_MS = 300
private let TAG = "<[^>]+>"

func plainMessage(_ html: String) -> String {
    html.replacingOccurrences(of: "<br />", with: "\n")
        .replacingOccurrences(of: TAG, with: "", options: .regularExpression)
        .split(omittingEmptySubsequences: false, whereSeparator: \.isNewline)
        .map { String($0.reversed().drop(while: \.isWhitespace).reversed()) }
        .joined(separator: "\n")
}

func motorAssignment(_ json: JSON?) -> MotorAssignmentState {
    guard let read = json, read.object != nil else { return MotorAssignmentState() }
    return MotorAssignmentState(
        multirotor: read["multirotor"].bool,
        enabled: read["enabled"].bool,
        active: read["active"].bool,
        message: read["message"].string,
        highlighted: Set(read["highlighted"].array.map { $0.int(0) })
    )
}

struct ActuatorOutputs: Equatable {
    let available: Bool
    let reason: String
    let showUi: Bool
    let groups: [ActuatorGroup]
    var testing: ActuatorTesting? = nil
    var geometry: Geometry? = nil
    var hasUnsetRequiredFunctions = false
    var assignment = MotorAssignmentState()
    var actions: [ActuatorActionGroup] = []
}

private func objects<T>(_ list: JSON, _ read: (JSON) -> T?) -> [T] {
    list.array.filter { $0.object != nil }.compactMap(read)
}

private func actuatorFact(_ json: JSON?) -> ActuatorFact? {
    guard let control = json, control.object != nil else { return nil }
    return factFromControl(control).map {
        ActuatorFact(label: control["label"].string, showAs: control["showAs"].string, bit: control["bit"].int(0), advanced: control["advanced"].bool, fact: $0)
    }
}

private func strings(_ json: JSON, _ key: String) -> [String] {
    json[key].array.map(\.string)
}

func geometryCell(_ json: JSON?) -> GeometryCell? {
    guard let json, json.object != nil else { return nil }
    if json["axis"].bool {
        return .Axis(options: strings(json, "options"), index: json["index"].int(0), params: strings(json, "params"), advanced: json["advanced"].bool, hidden: json["hidden"].bool, disabled: json["disabled"].bool)
    }
    if json["unavailable"].bool {
        return .Unavailable(label: json["label"].string, advanced: json["advanced"].bool, hidden: json["hidden"].bool)
    }
    if json["fixed"].bool {
        return .Fixed(label: json["label"].string, valueString: json["valueString"].string, advanced: json["advanced"].bool, hidden: json["hidden"].bool)
    }
    return actuatorFact(json).map {
        .Editable(item: $0, param: json["param"].string, channelFunction: json["channelFunction"].int(0), hidden: json["hidden"].bool, disabled: json["disabled"].bool)
    }
}

func geometry(_ json: JSON?) -> Geometry? {
    guard let read = json, read.object != nil else { return nil }
    return Geometry(
        title: read["title"].string,
        helpUrl: read["helpUrl"].string,
        groups: objects(read["groups"]) { group in
            GeometryGroup(
                label: group["label"].string,
                count: group["count"].object != nil ? factFromControl(group["count"]) : nil,
                channels: objects(group["channels"]) { channel in
                    GeometryChannel(label: channel["label"].string, cells: channel["cells"].array.map { geometryCell($0) })
                },
                params: objects(group["params"], actuatorFact)
            )
        },
        motors: geometryMotors(read)
    )
}

func actuatorOutputs(_ view: JSON?) -> ActuatorOutputs? {
    guard let read = view, read["class"].string == "ActuatorOutputs" else { return nil }
    return ActuatorOutputs(
        available: read["available"].bool,
        reason: read["reason"].string,
        showUi: read["showUi"].bool(true),
        groups: objects(read["groups"]) { group in
            ActuatorGroup(
                label: group["label"].string,
                enable: actuatorFact(group["enable"]),
                groupsVisible: group["groupsVisible"].bool,
                params: objects(group["params"], actuatorFact),
                subgroups: objects(group["subgroups"]) { subgroup in
                    ActuatorSubgroup(
                        label: subgroup["label"].string,
                        primary: actuatorFact(subgroup["primary"]),
                        params: objects(subgroup["params"], actuatorFact),
                        columns: objects(subgroup["columns"]) { ActuatorColumn(label: $0["label"].string, advanced: $0["advanced"].bool, visible: $0["visible"].bool) },
                        channels: objects(subgroup["channels"]) { ActuatorChannel(label: $0["label"].string, configs: $0["configs"].array.map { actuatorFact($0) }) }
                    )
                },
                notes: group["notes"].array.map(\.string)
            )
        },
        testing: actuatorTesting(read),
        geometry: geometry(read["geometry"]),
        hasUnsetRequiredFunctions: read["hasUnsetRequiredFunctions"].bool,
        assignment: motorAssignment(read["motorAssignment"]),
        actions: actuatorActions(read)
    )
}

func rawNumber(_ fact: Fact) -> Double? {
    if case .number(let value) = fact.value { return value }
    return Double(fact.valueString)
}

private func asLong(_ raw: Double) -> Int64 {
    raw.isNaN ? 0 : raw >= 9.2e18 ? .max : raw <= -9.2e18 ? .min : Int64(raw)
}

func bitsetChecked(_ raw: Double, _ bit: Int) -> Bool { (asLong(raw) & (Int64(1) << bit)) != 0 }

func bitsetWritten(_ raw: Double, _ bit: Int, _ on: Bool) -> Int64 {
    on ? asLong(raw) | (Int64(1) << bit) : asLong(raw) & ~(Int64(1) << bit)
}

func signWritten(_ raw: Double, _ on: Bool) -> Double { on ? abs(raw) : -abs(raw) }

private extension Array {
    func at(_ index: Int) -> Element? { indices.contains(index) ? self[index] : nil }
}

struct ActuatorsScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: ActuatorOutputs?
    @State private var tab = 0
    @State private var advanced = false
    @State private var refusal: String?
    @State private var testing = false
    @State private var confirming: String?
    @State private var failure: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            content
        }
        .task(id: revision) {
            let outputs = await offMain { actuatorOutputs(Qgc.get(ACTUATOR_OUTPUTS_VIEW)) }
            read = outputs
        }
        .task(id: read?.assignment.active) {
            while read?.assignment.active == true && !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(ASSIGNMENT_POLL_MS))
                if !Task.isCancelled { revision += 1 }
            }
        }
        .alert("Motor order identification and assignment", isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }), presenting: confirming) { _ in
            Button("Yes") {
                confirming = nil
                Task {
                    _ = await offMain { Qgc.invoke(MOTOR_ASSIGNMENT_START) }
                    revision += 1
                }
            }
            Button("No", role: .cancel) { confirming = nil }
        } message: { Text($0) }
    }

    @ViewBuilder private var content: some View {
        if let outputs = read {
            if !outputs.available || !outputs.showUi {
                Text(outputs.reason.ifBlank("This vehicle does not configure its actuators here.")).padding(16)
            } else if let group = outputs.groups.at(tab) ?? outputs.groups.first {
                screen(outputs, group)
            }
        } else {
            Text("Reading actuator metadata.").padding(16)
        }
    }

    private func screen(_ outputs: ActuatorOutputs, _ group: ActuatorGroup) -> some View {
        let groupIndex = outputs.groups.firstIndex(of: group) ?? 0
        return ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Spacer()
                    Toggle("Advanced", isOn: $advanced).fixedSize()
                }
                .padding(.horizontal, 16)
                if let geometry = outputs.geometry {
                    GeometrySection(
                        geometry: geometry,
                        advanced: advanced,
                        editable: mixerEditable(testing, outputs.assignment.active),
                        write: write,
                        highlighted: outputs.assignment.highlighted,
                        onMotor: { motor in
                            Task {
                                _ = await offMain { Qgc.invoke(MOTOR_ASSIGNMENT_SELECT, motor) }
                                revision += 1
                            }
                        },
                        onWrite: { revision += 1 }
                    )
                    .padding(16)
                }
                if let testingState = outputs.testing {
                    ActuatorTestSection(testing: testingState, actions: outputs.actions, enabled: testing, assigning: outputs.assignment.active) { testing = $0 }
                        .padding(16)
                }
                Text("Actuator outputs").font(.titleMedium).padding(.horizontal, 16)
                if outputs.hasUnsetRequiredFunctions {
                    Text("One or more actuator still needs to be assigned to an output.")
                        .foregroundStyle(theme.colors.error)
                        .padding(.horizontal, 16)
                }
                if outputs.assignment.multirotor {
                    assignmentButtons(outputs, group, groupIndex).padding(.horizontal, 16)
                }
                ScrollView(.horizontal, showsIndicators: false) {
                    Picker("", selection: Binding(get: { groupIndex }, set: { tab = $0 })) {
                        ForEach(Array(outputs.groups.enumerated()), id: \.offset) { index, each in
                            Text(each.label).tag(index)
                        }
                    }
                    .pickerStyle(.segmented)
                    .fixedSize()
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                }
                groupBody(group).padding(16)
            }
        }
        .alert("Error", isPresented: Binding(get: { failure != nil }, set: { if !$0 { failure = nil } }), presenting: failure) { _ in
            Button("Ok") { failure = nil }
        } message: { Text($0) }
    }

    private func assignmentButtons(_ outputs: ActuatorOutputs, _ group: ActuatorGroup, _ groupIndex: Int) -> some View {
        HStack(spacing: 8) {
            if !outputs.assignment.active && group.groupsVisible {
                Button("Identify & Assign Motors") {
                    Task {
                        let refused = await offMain { Qgc.refusalOf(MOTOR_ASSIGNMENT_INIT, groupIndex) }
                        let message = await offMain { actuatorOutputs(Qgc.get(ACTUATOR_OUTPUTS_VIEW))?.assignment.message ?? "" }
                        if let refused { failure = plainMessage(refused) } else { confirming = plainMessage(message) }
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(!outputs.assignment.enabled || testing)
            }
            if outputs.assignment.active {
                Button("Spin motor again") { offMain { Qgc.invoke(MOTOR_ASSIGNMENT_SPIN) } }
                    .buttonStyle(.bordered)
                Button("Abort") {
                    Task {
                        _ = await offMain { Qgc.invoke(MOTOR_ASSIGNMENT_ABORT) }
                        revision += 1
                    }
                }
                .buttonStyle(.bordered)
            }
        }
    }

    private func groupBody(_ group: ActuatorGroup) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            if let enable = group.enable {
                ActuatorFactRow(item: enable, write: write) { revision += 1 }
            }
            if group.groupsVisible {
                ForEach(Array(group.subgroups.enumerated()), id: \.offset) { _, subgroup in
                    if !subgroup.label.isEmpty {
                        Text(subgroup.label).font(.titleSmall)
                    }
                    if let primary = subgroup.primary {
                        ActuatorFactRow(item: primary, write: write) { revision += 1 }
                    }
                    ForEach(Array(subgroup.channels.enumerated()), id: \.offset) { _, channel in
                        OutlinedCard {
                            Text(channel.label).font(.labelLarge)
                            ForEach(Array(channel.configs.enumerated()), id: \.offset) { index, config in
                                if let column = subgroup.columns.at(index), column.visible, advanced || !column.advanced {
                                    if let config {
                                        ActuatorFactRow(
                                            item: ActuatorFact(label: column.label, showAs: config.showAs, bit: config.bit, advanced: config.advanced, fact: config.fact),
                                            write: write
                                        ) { revision += 1 }
                                    } else {
                                        NotAvailableRow(label: column.label)
                                    }
                                }
                            }
                        }
                    }
                    ForEach(Array(subgroup.params.enumerated()), id: \.offset) { _, param in
                        ActuatorFactRow(item: param, write: write) { revision += 1 }
                    }
                }
            }
            ForEach(Array(group.params.enumerated()), id: \.offset) { _, param in
                ActuatorFactRow(item: param, write: write) { revision += 1 }
            }
            ForEach(Array(group.notes.enumerated()), id: \.offset) { _, note in
                Text(note).font(.bodySmall)
            }
            if let refusal {
                Text(refusal).foregroundStyle(theme.colors.error)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func write(_ path: String, _ value: Any) {
        Task {
            refusal = await offMain { Qgc.writeRefusal(path, value) }
            revision += 1
        }
    }
}

func mixerEditable(_ testing: Bool, _ assigning: Bool) -> Bool { !testing && !assigning }

private let DISABLED_ALPHA = 0.38

private struct OutlinedCard<Content: View>: View {
    @ViewBuilder let content: Content
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) { content }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(RoundedRectangle(cornerRadius: Corner.medium).stroke(theme.colors.outlineVariant))
    }
}

private struct GeometrySection: View {
    let geometry: Geometry
    let advanced: Bool
    let editable: Bool
    let write: (String, Any) -> Void
    var highlighted: Set<Int> = []
    var onMotor: (Int) -> Void = { _ in }
    let onWrite: () -> Void
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(geometry.title).font(.titleMedium).frame(maxWidth: .infinity, alignment: .leading)
                if !geometry.helpUrl.isEmpty, let url = URL(string: geometry.helpUrl) {
                    Button("?") { openURL(url) }.buttonStyle(.borderless)
                }
            }
            if geometry.motors.count > 1 {
                GeometryImage(motors: geometry.motors, highlighted: highlighted, onMotor: onMotor)
            }
            VStack(alignment: .leading, spacing: 8) {
                ForEach(Array(geometry.groups.enumerated()), id: \.offset) { _, group in
                    Text(group.label).font(.titleSmall)
                    if let count = group.count {
                        FactRow(fact: count, onWrite: onWrite)
                    }
                    ForEach(Array(group.channels.enumerated()), id: \.offset) { _, channel in
                        OutlinedCard {
                            Text(channel.label).font(.labelLarge)
                            ForEach(Array(channel.cells.compactMap { $0 }.filter { !$0.hidden }.enumerated()), id: \.offset) { _, cell in
                                cellRow(cell)
                            }
                        }
                    }
                    ForEach(Array(group.params.filter { advanced || !$0.advanced }.enumerated()), id: \.offset) { _, param in
                        ActuatorFactRow(item: param, write: write, onWrite: onWrite)
                    }
                }
            }
            .opacity(editable ? 1 : DISABLED_ALPHA)
            .allowsHitTesting(editable)
        }
    }

    @ViewBuilder private func cellRow(_ cell: GeometryCell) -> some View {
        switch cell {
        case .Editable(let item, let param, let channelFunction, _, let disabled):
            if advanced || !item.advanced {
                MixerCellRow(item: item, param: param, channelFunction: channelFunction, disabled: disabled, onWrite: onWrite)
            }
        case .Fixed(let label, let valueString, let cellAdvanced, _):
            if advanced || !cellAdvanced {
                HStack {
                    Text(label).frame(maxWidth: .infinity, alignment: .leading)
                    Text(valueString)
                }
            }
        case .Axis(let options, let index, let params, let cellAdvanced, _, let disabled):
            if advanced || !cellAdvanced {
                AxisRow(options: options, index: index, params: params, disabled: disabled, onWrite: onWrite)
            }
        case .Unavailable(let label, let cellAdvanced, _):
            if advanced || !cellAdvanced {
                NotAvailableRow(label: label)
            }
        }
    }
}

private struct NotAvailableRow: View {
    let label: String

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Text(PARAM_NOT_AVAILABLE)
        }
    }
}

private struct CheckRow: View {
    let label: String
    let checked: Bool
    var enabled = true
    let onChange: (Bool) -> Void

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Toggle("", isOn: Binding(get: { checked }, set: onChange)).labelsHidden().disabled(!enabled)
        }
    }
}

private struct MixerCellRow: View {
    let item: ActuatorFact
    let param: String
    let channelFunction: Int
    let disabled: Bool
    let onWrite: () -> Void
    @State private var typed = ""

    var body: some View {
        let raw = rawNumber(item.fact)
        if item.showAs == SHOW_TRUE_IF_POSITIVE, let raw {
            CheckRow(label: item.label, checked: raw > 0, enabled: !disabled) { set(signWritten(raw, $0)) }
        } else if item.showAs == SHOW_BITSET, let raw {
            CheckRow(label: item.label, checked: bitsetChecked(raw, item.bit), enabled: !disabled) { set(bitsetWritten(raw, item.bit, $0)) }
        } else if !item.fact.enumStrings.isEmpty {
            HStack {
                Text(item.label).frame(maxWidth: .infinity, alignment: .leading)
                Menu {
                    ForEach(Array(item.fact.enumStrings.enumerated()), id: \.offset) { index, label in
                        Button(label) {
                            if let value = item.fact.enumValues.at(index).flatMap(Double.init) { set(value) }
                        }
                    }
                } label: {
                    Text(item.fact.valueString)
                }
                .buttonStyle(.bordered)
                .disabled(disabled)
            }
        } else {
            HStack {
                Text(item.label).frame(maxWidth: .infinity, alignment: .leading)
                HStack(spacing: 4) {
                    TextField("", text: $typed)
                        .keyboardType(.numbersAndPunctuation)
                        .submitLabel(.done)
                        .onSubmit { if let value = Double(typed) { set(value) } }
                    Text(item.fact.units)
                }
                .textFieldStyle(.roundedBorder)
                .disabled(disabled)
                .frame(maxWidth: .infinity)
            }
            .onChange(of: item.fact.valueString, initial: true) { typed = item.fact.valueString }
        }
    }

    private func set(_ value: Any) {
        let param = param
        let channelFunction = channelFunction
        Task {
            _ = await offMain { Qgc.invoke(ACTUATOR_MIXER_SET, param, value, channelFunction) }
            onWrite()
        }
    }
}

private struct AxisRow: View {
    let options: [String]
    let index: Int
    let params: [String]
    let disabled: Bool
    let onWrite: () -> Void

    var body: some View {
        HStack {
            Text("Axis").frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                ForEach(Array(options.enumerated()), id: \.offset) { at, label in
                    Button(label) {
                        let params = params
                        Task {
                            _ = await offMain { Qgc.invoke(ACTUATOR_MIXER_AXIS, params, at) }
                            onWrite()
                        }
                    }
                }
            } label: {
                Text(options.at(index) ?? "")
            }
            .buttonStyle(.bordered)
            .disabled(disabled)
        }
    }
}

private struct ActuatorFactRow: View {
    let item: ActuatorFact
    let write: (String, Any) -> Void
    let onWrite: () -> Void

    var body: some View {
        let raw = rawNumber(item.fact)
        if item.showAs == SHOW_BITSET, let raw {
            CheckRow(label: item.label, checked: bitsetChecked(raw, item.bit)) { write(item.fact.path, bitsetWritten(raw, item.bit, $0)) }
        } else if item.showAs == SHOW_TRUE_IF_POSITIVE, let raw {
            CheckRow(label: item.label, checked: raw > 0) { write(item.fact.path, signWritten(raw, $0)) }
        } else {
            VStack(spacing: 0) {
                FactRow(fact: item.fact, title: item.label.ifEmpty(item.fact.title), onWrite: onWrite)
                Divider()
            }
        }
    }
}
