import SwiftUI

let SET_RC_TO_PARAM = "parameters.setRcToParam"
private let OPEN_RC_TO_PARAM = "parameters.openRcToParam"
private let RC_TO_PARAM_READY = "parameters.rcToParamReady"
private let READY_POLL_MS = 250
private let DEFAULT_RC_SCALE = "1.0"
private let TUNING_IDS = [1, 2, 3]

struct RcToParam: Equatable {
    var scale: Double
    var center: Double
    var tuningIndex: Int
    var min: Double
    var max: Double
}

func rcToParam(_ scale: String, _ center: String, _ tuningIndex: Int, _ min: String, _ max: String) -> RcToParam? {
    let numbers = [scale, center, min, max].compactMap { Double($0.trimmed) }
    guard numbers.count == 4 else { return nil }
    return RcToParam(scale: numbers[0], center: numbers[1], tuningIndex: tuningIndex, min: numbers[2], max: numbers[3])
}

private func initialCenter(_ fact: Fact) -> String {
    if case .number(let value) = fact.value { return JSON.format(value) }
    return fact.valueString
}

struct RcToParamDialog: View {
    let fact: Fact
    let onDismiss: () -> Void
    @State private var scale = DEFAULT_RC_SCALE
    @State private var center: String
    @State private var min: String
    @State private var max: String
    @State private var tuningIndex = 0
    @State private var refusal: String?
    @State private var ready = false
    @Environment(\.theme) private var theme

    init(fact: Fact, onDismiss: @escaping () -> Void) {
        self.fact = fact
        self.onDismiss = onDismiss
        _center = State(initialValue: initialCenter(fact))
        _min = State(initialValue: fact.minString)
        _max = State(initialValue: fact.maxString)
    }

    var body: some View {
        let entered = rcToParam(scale, center, tuningIndex, min, max)
        SetupDialog(title: "RC To Param") {
            Text("Bind an RC Channel to a parameter value. Tuning IDs can be mapped to an RC Channel from Radio Setup page.").font(.bodySmall)
            if !ready { Text("Waiting on parameter update from Vehicle.") }
            Text("Parameter  \(fact.name)")
            Menu {
                ForEach(Array(TUNING_IDS.enumerated()), id: \.offset) { index, id in
                    Button("\(id)") { tuningIndex = index }
                }
            } label: {
                Text("Tuning ID \(TUNING_IDS[tuningIndex])")
            }
            .buttonStyle(.bordered)
            .disabled(!ready)
            field("Scale", $scale)
            field("Center Value", $center)
            field("Min Value", $min)
            field("Max Value", $max)
            Text("Double check that all values are correct prior to confirming dialog.").font(.bodySmall)
            if let refusal {
                Text(refusal).foregroundStyle(theme.colors.error)
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button("OK") {
                guard let chosen = entered else { return }
                let name = fact.name
                Task {
                    refusal = await offMain { Qgc.refusalOf(SET_RC_TO_PARAM, name, chosen.scale, chosen.center, chosen.tuningIndex, chosen.min, chosen.max) }
                    if refusal == nil { onDismiss() }
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(entered == nil)
        }
        .task(id: fact.name) {
            let name = fact.name
            _ = await offMain { Qgc.invoke(OPEN_RC_TO_PARAM, name) }
            while !ready, !Task.isCancelled {
                ready = await offMain {
                    if case .bool(let answer) = Qgc.invokeResult(RC_TO_PARAM_READY, name) { return answer }
                    return true
                }
                if !ready { try? await Task.sleep(for: .milliseconds(READY_POLL_MS)) }
            }
        }
    }

    private func field(_ label: String, _ value: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(label, text: value)
                .keyboardType(.numbersAndPunctuation)
                .textFieldStyle(.roundedBorder)
                .disabled(!ready)
        }
    }
}
