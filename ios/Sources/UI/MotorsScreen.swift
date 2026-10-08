import SwiftUI

private let FRAME_VIEW = "view.frame"

private let TIMEOUT_SECONDS = 3
private let UNKNOWN_MOTOR_COUNT = 8

func reportedMotors(_ view: JSON?) -> Int? {
    guard let view, !view["motorCount"].isNull else { return nil }
    let count = view["motorCount"].int(0)
    return count >= 1 ? count : nil
}

func motorCount(_ reported: Int?) -> Int { reported ?? UNKNOWN_MOTOR_COUNT }

func motorCountNotice(_ reported: Int?) -> String? {
    reported == nil ? "Warning: Unable to determine motor count" : nil
}

struct MotorGate: Equatable {
    let connected: Bool
    let armed: Bool
    let contactKnownLost: Bool
}

func motorGate(_ view: JSON?) -> MotorGate {
    MotorGate(
        connected: view?["connected"].bool == true,
        armed: view?["armed"].bool == true,
        contactKnownLost: view.map { !$0["contactLost"].isNull && $0["contactLost"].bool } == true
    )
}

func canTest(_ gate: MotorGate, _ propsOff: Bool) -> Bool {
    gate.connected && propsOff && !gate.armed && !gate.contactKnownLost
}

func canStop(_ gate: MotorGate) -> Bool { gate.connected }

func motorRefusal(_ gate: MotorGate) -> String? {
    if !gate.connected { return "No vehicle is connected, so nothing will answer a motor test." }
    if gate.armed { return "The vehicle is armed. Disarm it before testing a motor." }
    if gate.contactKnownLost { return "The vehicle has stopped answering. Check the link before testing a motor." }
    return nil
}

func spin(_ motor: Int, _ percent: Int) {
    let seconds = percent == 0 ? 0 : TIMEOUT_SECONDS
    VehicleCommands.motorTest(motor, percent: percent, seconds: seconds, inOrder: true)
}

func motorLabel(_ motor: Int, _ letters: Bool) -> String {
    letters ? String(UnicodeScalar(UInt8(clamping: 64 + motor))) : String(motor)
}

private func spinAll(_ motors: Int, _ percent: Int) {
    offMainInOrder { (1...max(motors, 1)).forEach { spin($0, percent) } }
}

struct MotorsScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(FRAME_VIEW) private var frameJson
    @QgcPath(SETUP) private var setupJson
    @State private var propsOff = false
    @State private var throttle: Float = 0

    var body: some View {
        let reported = reportedMotors(frameJson)
        let gate = motorGate(frameJson)
        let motors = motorCount(reported)
        let letters = setupReadiness(setupJson)?.firmware == "apm"
        let testable = canTest(gate, propsOff)
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 12) {
                    Image(.warning)
                    Text("Make sure you remove all props.").font(.labelLarge)
                }
                .foregroundStyle(theme.aircast.warning)
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme.aircast.warningContainer, in: Capsule())
                if let notice = motorCountNotice(reported) {
                    Text(notice).font(.bodyMedium).foregroundStyle(theme.aircast.warning)
                }
                if let refused = motorRefusal(gate) {
                    Text(refused).font(.bodyMedium).foregroundStyle(theme.colors.error)
                }
                HStack(spacing: 12) {
                    Toggle("", isOn: Binding(get: { propsOff }, set: { on in
                        propsOff = on
                        if !on { throttle = 0 }
                    }))
                    .labelsHidden()
                    Text(propsOff ? "Careful: motors are enabled" : "Propellers are removed - enable slider and motors")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.aircast.warning)
                }
                HStack {
                    Text("Throttle").font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                    Text("\(Int(throttle))%").font(.labelMedium).foregroundStyle(theme.colors.primary)
                }
                Slider(value: $throttle, in: 0...100).disabled(!testable)
                ForEach(Array(stride(from: 1, through: motors, by: 2)), id: \.self) { first in
                    HStack(spacing: 12) {
                        ForEach(Array(first...min(first + 1, motors)), id: \.self) { motor in
                            MotorTile(label: "Motor \(motorLabel(motor, letters))", testable: testable) {
                                let percent = Int(throttle)
                                offMainInOrder { spin(motor, percent) }
                            }
                        }
                        if first == motors {
                            Color.clear.frame(maxWidth: .infinity, maxHeight: 1)
                        }
                    }
                }
                HStack(spacing: 8) {
                    Button("All") { spinAll(motors, Int(throttle)) }
                        .buttonStyle(.bordered)
                        .disabled(!testable)
                    Button("Stop") { spinAll(motors, 0) }
                        .buttonStyle(.bordered)
                        .disabled(!canStop(gate))
                }
            }
            .padding(16)
        }
    }
}

private struct MotorTile: View {
    let label: String
    let testable: Bool
    let onSpin: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onSpin) {
            VStack {
                Text(label).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                Text(testable ? "Tap to spin" : "Locked").font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.vertical, 20)
            .frame(maxWidth: .infinity)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.large))
        }
        .buttonStyle(.plain)
        .disabled(!testable)
    }
}
