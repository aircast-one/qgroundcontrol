import SwiftUI
import UIKit

let APM_SUB_MOTORS_SCREEN = "apmSubMotors"
let APM_SUB_MOTORS_VIEW = "view.apmSubMotors"
private let SUB_MOTORS_POLL_MS = 500
private let SUB_MOTOR_TEST_MS = 50
private let SUB_COOL_DOWN_MS = 11_000
let SUB_NEUTRAL: Float = 50

let MOTOR_DISPLAY_FRAMES: Set<Int> = [0, 1, 2, 4, 5]

func motorDisplayFrame(_ frames: SubFrames?) -> Int? {
    frames?.selected.flatMap { MOTOR_DISPLAY_FRAMES.contains($0) ? $0 : nil }
}

struct SubMotor: Equatable {
    let motor: Int
    let reversed: Bool
}

struct SubMotors: Equatable {
    let armed: Bool
    let detecting: Bool
    let canRunManualTest: Bool
    let motors: [SubMotor]
    let warning: String
    let offersAutoDetect: Bool
    let autoDetectHelp: String
    let detectionMessages: String
}

func subMotors(_ view: JSON?) -> SubMotors? {
    guard let it = view, it["available"].bool else { return nil }
    return SubMotors(
        armed: it["armed"].bool,
        detecting: it["detecting"].bool,
        canRunManualTest: it["canRunManualTest"].bool,
        motors: it["motors"].array.filter { $0.object != nil }.map { SubMotor(motor: $0["motor"].int(0), reversed: $0["reversed"].bool) },
        warning: it["warning"].string,
        offersAutoDetect: it["offersAutoDetect"].bool,
        autoDetectHelp: it["autoDetectHelp"].string,
        detectionMessages: it["detectionMessages"].string
    )
}

private func refused(_ path: String, _ args: [Any?]) -> String? {
    refusal(Qgc.call(path, arguments: args))
}

struct ApmSubMotorsScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: SubMotors?
    @State private var refusal: String?
    @State private var shouldRunManualTest = false
    @State private var coolingDown = false
    @State private var lastIndex = 0
    @State private var sliders: [Int: Float] = [:]
    @State private var frameDisplay: UIImage?

    private var armed: Bool { read?.armed == true }
    private var testing: Bool { read?.canRunManualTest == true && shouldRunManualTest }

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let state = read {
                screen(state)
            } else {
                Text("This page is for an ArduSub vehicle.").padding(16)
            }
        }
        .task {
            _ = await offMain { SetupCommands.openSubMotors() }
        }
        .task {
            frameDisplay = await offMain { motorDisplayFrame(subFrames(Qgc.get(APM_SUB_FRAME_VIEW))).flatMap(frameImage) }
        }
        .task(id: revision) {
            let motors = await offMain { subMotors(Qgc.get(APM_SUB_MOTORS_VIEW)) }
            read = motors
            try? await Task.sleep(for: .milliseconds(SUB_MOTORS_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
        .task(id: armed) {
            sliders = [:]
            shouldRunManualTest = armed
            guard !armed, read != nil else { return }
            coolingDown = true
            try? await Task.sleep(for: .milliseconds(SUB_COOL_DOWN_MS))
            if !Task.isCancelled { coolingDown = false }
        }
        .task(id: testing) {
            guard testing else { return }
            while !Task.isCancelled {
                let index = lastIndex
                let value = Double(sliders[index] ?? SUB_NEUTRAL)
                _ = await offMain { SetupCommands.testSubMotor(index, value) }
                try? await Task.sleep(for: .milliseconds(SUB_MOTOR_TEST_MS))
            }
        }
    }

    private func screen(_ state: SubMotors) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                if let frameDisplay {
                    Image(uiImage: frameDisplay)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: .infinity, maxHeight: 240)
                        .accessibilityLabel("Frame")
                }
                ForEach(Array(state.motors.enumerated()), id: \.offset) { index, motor in
                    HStack {
                        Text("\(motor.motor)").frame(width: 24, alignment: .leading)
                        Slider(
                            value: Binding(get: { sliders[index] ?? SUB_NEUTRAL }, set: { value in
                                lastIndex = index
                                sliders[index] = value
                            }),
                            in: 0...100,
                            onEditingChanged: { editing in if !editing { sliders[index] = SUB_NEUTRAL } }
                        )
                        .disabled(!testing)
                        .frame(maxWidth: .infinity)
                        Toggle("", isOn: Binding(get: { motor.reversed }, set: { on in
                            sliders[index] = SUB_NEUTRAL
                            act("apmSubMotors.reverse", motor.motor, on)
                        }))
                        .labelsHidden()
                        .disabled(!testing)
                    }
                }
                Text("Reverse motor direction").font(.bodySmall)
                Text(state.warning).font(.bodySmall)
                HStack(spacing: 8) {
                    Toggle("", isOn: Binding(get: { state.armed }, set: { act("apmSubMotors.arm", $0) }))
                        .labelsHidden()
                        .disabled(coolingDown)
                    Text(coolingDown ? "A 10 second coooldown is required before testing again, please stand by..." : "Slide this switch to arm the vehicle and enable the motor test (CAUTION!)")
                        .foregroundStyle(theme.colors.error)
                }
                if state.offersAutoDetect {
                    Text("Automatic motor direction detection").font(.titleMedium)
                    Text(state.autoDetectHelp).font(.bodySmall)
                    Button("Auto-Detect Directions") { act("apmSubMotors.autoDetect") }
                        .buttonStyle(.borderedProminent)
                        .disabled(state.detecting)
                    if !state.detectionMessages.isBlank {
                        Text(state.detectionMessages).font(.bodySmall)
                    }
                }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            }
            .padding(16)
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task { refusal = await offMain { refused(path, args) } }
    }
}
