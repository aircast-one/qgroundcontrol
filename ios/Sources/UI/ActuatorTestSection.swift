import SwiftUI

let ACTUATOR_TEST_ACTIVE = "actuatorTest.setActive"
let ACTUATOR_TEST_SET = "actuatorTest.setChannelTo"
let ACTUATOR_TEST_STOP = "actuatorTest.stopControl"
private let SEND_MS = 50
private let SNAP_FRACTION = 0.15

struct TestChannel: Equatable, Hashable {
    let label: String
    let function: Int
    let min: Double
    let max: Double
    let `default`: Double?
    let isMotor: Bool

    var snap: Bool { `default` == nil }
    var snapRange: Double { (max - min) * SNAP_FRACTION }
    var from: Double { snap ? min - snapRange : min }
    var rest: Double { `default` ?? from }
}

struct ActuatorTesting: Equatable {
    let actuators: [TestChannel]
    let allMotors: TestChannel?
    let hadFailure: Bool
}

struct ActuatorActionChoice: Equatable {
    let label: String
    let function: Int
}

struct ActuatorActionGroup: Equatable {
    let label: String
    let type: Int
    let actions: [ActuatorActionChoice]
}

let ACTUATOR_ACTION_TRIGGER = "actuatorAction.trigger"

func actuatorActions(_ view: JSON?) -> [ActuatorActionGroup] {
    guard let groups = view?["actions"].arrayOrNil else { return [] }
    return groups.filter { $0.object != nil }.map { group in
        ActuatorActionGroup(
            label: group["label"].string,
            type: group["type"].int(0),
            actions: group["actions"].array.filter { $0.object != nil }.map { ActuatorActionChoice(label: $0["label"].string, function: $0["function"].int(0)) }
        )
    }
}

private func testChannel(_ json: JSON) -> TestChannel? {
    guard json.object != nil else { return nil }
    return TestChannel(
        label: json["label"].string,
        function: json["function"].int(0),
        min: json["min"].double(.nan),
        max: json["max"].double(.nan),
        default: json["default"].isNull ? nil : json["default"].double(.nan),
        isMotor: json["isMotor"].bool
    )
}

func actuatorTesting(_ view: JSON?) -> ActuatorTesting? {
    guard let testing = view?["testing"], testing.object != nil else { return nil }
    return ActuatorTesting(
        actuators: testing["actuators"].array.compactMap(testChannel),
        allMotors: testChannel(testing["allMotors"]),
        hadFailure: testing["hadFailure"].bool
    )
}

func snapped(_ channel: TestChannel, _ value: Double) -> Double {
    if !channel.snap || value >= channel.min { return value }
    return value < channel.min - channel.snapRange / 2 ? channel.min - channel.snapRange : channel.min
}

func sentValue(_ channel: TestChannel, _ value: Double) -> Double? {
    value < channel.min - channel.snapRange / 2 ? channel.default : value
}

private func testRange(_ channel: TestChannel) -> ClosedRange<Double> {
    channel.from.isFinite && channel.max.isFinite && channel.max > channel.from ? channel.from...channel.max : 0...1
}

private func restValues(_ actuators: [TestChannel]) -> [Int: Double] {
    Dictionary(actuators.map { ($0.function, $0.rest) }, uniquingKeysWith: { _, last in last })
}

struct ActuatorTestSection: View {
    let testing: ActuatorTesting
    var actions: [ActuatorActionGroup] = []
    let enabled: Bool
    var assigning: Bool = false
    let onEnabled: (Bool) -> Void
    @Environment(\.theme) private var theme
    @State private var values: [Int: Double] = [:]
    @State private var moved: Set<Int> = []
    @State private var allMotors = 0.0

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Actuator testing").font(.titleMedium)
            if testing.actuators.isEmpty {
                Text("Configure some outputs in order to test them.")
            } else {
                HStack(spacing: 8) {
                    Toggle("", isOn: Binding(get: { enabled && !testing.hadFailure }, set: setEnabled))
                        .labelsHidden()
                        .disabled(testing.hadFailure || assigning)
                    Text(enabled ? "Careful: actuator sliders are enabled" : "Propellers are removed - enable sliders")
                        .foregroundStyle(theme.aircast.warning)
                }
                if let motors = testing.allMotors {
                    TestSlider(channel: motors, value: allMotors, enabled: enabled) { value in
                        allMotors = value
                        let motorFunctions = testing.actuators.filter(\.isMotor).map(\.function)
                        values = values.merging(motorFunctions.map { ($0, value) }, uniquingKeysWith: { _, new in new })
                        moved = moved.union(motorFunctions)
                    }
                }
                ForEach(testing.actuators, id: \.function) { channel in
                    TestSlider(channel: channel, value: values[channel.function] ?? channel.rest, enabled: enabled) { value in
                        values[channel.function] = value
                        moved.insert(channel.function)
                    }
                    .background {
                        if enabled && moved.contains(channel.function) {
                            ChannelSender(channel: channel, value: { values[channel.function] ?? channel.rest }) {
                                values[channel.function] = channel.rest
                                moved.remove(channel.function)
                            }
                        }
                    }
                }
            }
            if !actions.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(Array(actions.enumerated()), id: \.offset) { _, group in
                            ActionGroupButton(group: group, enabled: !enabled && !assigning)
                        }
                    }
                }
            }
        }
        .onChange(of: testing.actuators, initial: true) {
            values = restValues(testing.actuators)
            moved = []
        }
        .onChange(of: testing.allMotors, initial: true) { allMotors = testing.allMotors?.rest ?? 0 }
        .onDisappear { offMainInOrder { Qgc.invoke(ACTUATOR_TEST_ACTIVE, false) } }
    }

    private func setEnabled(_ on: Bool) {
        onEnabled(on)
        if !on {
            values = restValues(testing.actuators)
            moved = []
            allMotors = testing.allMotors?.rest ?? 0
        }
        offMainInOrder { Qgc.invoke(ACTUATOR_TEST_ACTIVE, on) }
    }
}

private struct ActionGroupButton: View {
    let group: ActuatorActionGroup
    let enabled: Bool

    var body: some View {
        Menu {
            ForEach(Array(group.actions.enumerated()), id: \.offset) { _, action in
                Button(action.label) {
                    let type = group.type
                    offMain { Qgc.invoke(ACTUATOR_ACTION_TRIGGER, type, action.function) }
                }
            }
        } label: {
            Text(group.label)
        }
        .buttonStyle(.bordered)
        .disabled(!enabled)
    }
}

private struct ChannelSender: View {
    let channel: TestChannel
    let value: () -> Double
    let onStopped: () -> Void

    var body: some View {
        Color.clear.task(id: channel.function) {
            while !Task.isCancelled {
                let send = sentValue(channel, value())
                let function = channel.function
                _ = await offMain {
                    send.map { Qgc.invoke(ACTUATOR_TEST_SET, function, $0) } ?? Qgc.invoke(ACTUATOR_TEST_STOP, function)
                }
                if send == nil {
                    onStopped()
                    return
                }
                try? await Task.sleep(for: .milliseconds(SEND_MS))
            }
        }
    }
}

private struct TestSlider: View {
    let channel: TestChannel
    let value: Double
    let enabled: Bool
    let onChange: (Double) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(channel.label).font(.labelMedium)
            Slider(
                value: Binding(get: { value }, set: { onChange(snapped(channel, $0)) }),
                in: testRange(channel)
            )
            .disabled(!enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
