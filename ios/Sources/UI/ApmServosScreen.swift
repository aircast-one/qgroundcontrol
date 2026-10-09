import SwiftUI

let APM_SERVOS_VIEW = "view.apmServos"
let APM_SERVOS_SCREEN = "apmServos"
private let POLL_MS = 200
private let REPEAT_DELAY_MS = 350
private let REPEAT_MS = 80

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
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var servos: [ApmServo]?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let read = servos {
                if read.isEmpty {
                    Text("This vehicle exposes no servo outputs.").padding(16)
                } else {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            Text("Configure ArduPilot servo outputs.")
                                .font(.bodyMedium)
                                .foregroundStyle(theme.colors.onSurfaceVariant)
                                .padding(.horizontal, 20)
                                .padding(.vertical, 12)
                            ForEach(read, id: \.index) { servo in ServoCard(servo: servo) }
                        }
                    }
                }
            } else {
                Text("Reading servo outputs.").padding(16)
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

private struct ServoCard: View {
    let servo: ApmServo

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: "Servo \(servo.index)")
            HStack(spacing: 12) {
                Text("Position").font(.bodyMedium)
                ProgressView(value: Double(Swift.min(Swift.max(servo.position ?? 0, 0), 1)))
                    .frame(maxWidth: .infinity)
                Text(servo.pwm.map(String.init) ?? "-").font(.bodyMedium).frame(width: 48, alignment: .leading)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 4)
            if let function = servo.function {
                FactRow(fact: function, title: "Function")
            }
            if let min = servo.min {
                Stepper(title: "Min", fact: min)
            }
            if let trim = servo.trim {
                Stepper(title: "Trim", fact: trim)
            }
            if let max = servo.max {
                Stepper(title: "Max", fact: max)
            }
            if let reversed = servo.reversed {
                FactRow(fact: reversed, title: "Reversed")
            }
        }
        .padding(.vertical, 4)
    }
}

private struct Stepper: View {
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
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 12) {
                Text(title).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                RepeatButton(label: "-") { step(-1) }
                TextField("", text: $typed)
                    .font(.bodyLarge)
                    .textFieldStyle(.roundedBorder)
                    .keyboardType(.numbersAndPunctuation)
                    .submitLabel(.done)
                    .focused($focused)
                    .onSubmit(commit)
                    .frame(width: 96)
                RepeatButton(label: "+") { step(1) }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 4)
            if let refusal {
                Text(refusal).foregroundStyle(theme.colors.error).padding(.horizontal, 20)
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

private struct RepeatButton: View {
    let label: String
    let onStep: () -> Void
    @Environment(\.theme) private var theme
    @State private var repeating: Task<Void, Never>?
    @GestureState private var pressing = false

    var body: some View {
        Text(label)
            .font(.titleMedium)
            .frame(width: 40, height: 40)
            .background(theme.colors.secondaryContainer, in: RoundedRectangle(cornerRadius: Corner.small))
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($pressing) { _, state, _ in state = true }
                    .onChanged { _ in
                        guard repeating == nil else { return }
                        onStep()
                        repeating = Task {
                            try? await Task.sleep(for: .milliseconds(REPEAT_DELAY_MS))
                            while !Task.isCancelled {
                                onStep()
                                try? await Task.sleep(for: .milliseconds(REPEAT_MS))
                            }
                        }
                    }
                    .onEnded { _ in stop() }
            )
            .onChange(of: pressing) { if !pressing { stop() } }
            .onDisappear(perform: stop)
            .accessibilityAddTraits(.isButton)
            .accessibilityAction { onStep() }
    }

    private func stop() {
        repeating?.cancel()
        repeating = nil
    }
}
