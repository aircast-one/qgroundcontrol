import SwiftUI

let APM_SERVOS_VIEW = "view.apmServos"
let APM_SERVOS_SCREEN = "apmServos"
private let POLL_MS = 200

struct ApmServo: Equatable {
    let index: Int
    let pwm: Int?
    let position: Float?
    let function: Fact?
    let min: Fact?
    let trim: Fact?
    let max: Fact?
    let reversed: Fact?
}

func apmServos(_ view: JSON?) -> [ApmServo] {
    guard let servos = view?["servos"].arrayOrNil else { return [] }
    return servos.filter { $0.object != nil }.map { servo in
        let fact = { (key: String) in servo[key].object != nil ? factFromControl(servo[key]) : nil }
        return ApmServo(
            index: servo["index"].int(0),
            pwm: servo["pwm"].isNull ? nil : servo["pwm"].int(0),
            position: servo["position"].isNull ? nil : Float(servo["position"].double(.nan)),
            function: fact("function"),
            min: fact("min"),
            trim: fact("trim"),
            max: fact("max"),
            reversed: fact("reversed")
        )
    }
}

func stepped(_ fact: Fact, _ direction: Int) -> Double? {
    rawNumber(fact).map { $0 + Double(direction) }
}

struct ApmServosScreen: View {
    @State private var revision = 0
    @State private var servos: [ApmServo]?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let read = servos {
                if read.isEmpty {
                    NativeForm {
                        Section { Text("This vehicle exposes no servo outputs.").foregroundStyle(Color.secondary) }
                    }
                } else {
                    NativeForm {
                        FormNote(text: "Configure ArduPilot servo outputs.")
                        ForEach(read, id: \.index) { servo in ServoSection(servo: servo) }
                    }
                }
            } else {
                NativeForm {
                    Section { Text("Reading servo outputs.").foregroundStyle(Color.secondary) }
                }
            }
        }
        .task(id: revision) {
            let read = await offMain { apmServos(Qgc.get(APM_SERVOS_VIEW)) }
            servos = read
            try? await Task.sleep(for: .milliseconds(POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
    }
}

private struct ServoSection: View {
    let servo: ApmServo

    var body: some View {
        Section {
            LabeledContent("Position") {
                HStack(spacing: Space.s3) {
                    ProgressView(value: Double(Swift.min(Swift.max(servo.position ?? 0, 0), 1)))
                        .frame(maxWidth: SERVO_BAR_WIDTH)
                    Text(servo.pwm.map(String.init) ?? "-").monospacedDigit().frame(width: 48, alignment: .trailing)
                }
            }
            if let function = servo.function {
                FactRow(fact: function, title: "Function")
            }
            if let min = servo.min {
                ServoValueRow(title: "Min", fact: min)
            }
            if let trim = servo.trim {
                ServoValueRow(title: "Trim", fact: trim)
            }
            if let max = servo.max {
                ServoValueRow(title: "Max", fact: max)
            }
            if let reversed = servo.reversed {
                FactRow(fact: reversed, title: "Reversed")
            }
        } header: {
            SectionHeader(text: "Servo \(servo.index)")
        }
    }
}

private let SERVO_BAR_WIDTH: CGFloat = 160
private let SERVO_FIELD_WIDTH: CGFloat = 80

// A PWM value: typed on the right, or stepped by one with the native stepper (which repeats while held).
private struct ServoValueRow: View {
    let title: String
    let fact: Fact
    @Environment(\.theme) private var theme
    @State private var refusal: String?
    @State private var pending: Double?
    @State private var latest: Fact?
    @State private var typed = ""
    @FocusState private var focused: Bool

    private var shown: String { pending.map { JSON.format($0.rounded(.towardZero)) } ?? fact.valueString }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Stepper(onIncrement: { step(1) }, onDecrement: { step(-1) }) {
                HStack(spacing: Space.s3) {
                    Text(title).frame(maxWidth: .infinity, alignment: .leading)
                    TextField(title, text: $typed)
                        .labelsHidden()
                        .multilineTextAlignment(.trailing)
                        .keyboardType(.numbersAndPunctuation)
                        .submitLabel(.done)
                        .focused($focused)
                        .onSubmit(commit)
                        .frame(width: SERVO_FIELD_WIDTH)
                }
            }
            if let refusal {
                Text(refusal).font(.footnote).foregroundStyle(theme.colors.error)
            }
        }
        .onChange(of: shown, initial: true) { typed = shown }
        .onChange(of: focused) { if !focused { commit() } }
        .onChange(of: fact, initial: true) { latest = fact }
        .onChange(of: fact.valueString) { pending = nil }
        .onChange(of: fact.path) {
            refusal = nil
            pending = nil
        }
    }

    private func write(_ next: Double) {
        pending = next
        let path = fact.path
        Task {
            let refused = await offMain { Qgc.writeRefusal(path, next) }
            refusal = refused
            if refused != nil { pending = nil }
        }
    }

    private func step(_ direction: Int) {
        guard let next = (pending ?? stepped(latest ?? fact, 0)).map({ $0 + Double(direction) }) else { return }
        write(next)
    }

    private func commit() {
        let number = typedNumber(typed).flatMap { $0 == $0.rounded(.down) ? $0 : nil }
        guard let number else {
            refusal = "Enter a whole number."
            typed = shown
            return
        }
        if number != typedNumber(shown) { write(number) }
    }
}
