import SwiftUI

let PLAN_DEFAULT_KEYS = ["altitude", "cruise", "hover", "ascent", "descent"]

private let SPEEDS_SET_BY_FLIGHT_SPEED: Set<String> = ["cruise", "hover"]
private let FLIGHT_SPEED_USED = "The mission flight speed is used instead"
private let DEFAULT_ALTITUDE_SUFFIX = ".defaultMissionItemAltitude"

func planDefaults(_ view: JSON?) -> [Fact] {
    guard let served = view?["defaults"], served.object != nil else { return [] }
    let flightSpeedSpecified = served["flightSpeed"]["specified"].bool
    return PLAN_DEFAULT_KEYS.compactMap { key in
        guard served[key].object != nil, let fact = factFromControl(served[key]) else { return nil }
        guard flightSpeedSpecified, SPEEDS_SET_BY_FLIGHT_SPEED.contains(key) else { return fact }
        return withChanges(fact) {
            $0.enabled = false
            $0.disabledReason = FLIGHT_SPEED_USED
        }
    }
}

func planDefaultsNote(_ view: JSON?, _ facts: [Fact]) -> String {
    facts.isEmpty ? "This core does not report the plan's defaults." : view?["defaults"]["speedNote"].string ?? ""
}

struct PlanDefaultsDialog: View {
    let view: JSON?
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme
    @State private var refusal: String?

    var body: some View {
        let facts = planDefaults(view)
        let grouped = Dictionary(grouping: facts) { $0.path.hasSuffix(DEFAULT_ALTITUDE_SUFFIX) }
        let altitude = grouped[true] ?? []
        let speeds = grouped[false] ?? []
        let flightSpeed = speedSectionOf(view?["defaults"]["flightSpeed"])
        PlanDialog(title: "Plan defaults", onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s1) {
                let note = planDefaultsNote(view, facts)
                if !note.isBlank { Text(note).font(.bodySmall) }
                ForEach(altitude) { fact in FactRow(fact: fact, onWrite: {}) }
                if let flightSpeed {
                    SpeedSectionRow(speed: flightSpeed, withSlider: true) { written in
                        Task { refusal = await offMain { written() } }
                    }
                }
                if let refusal { Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error) }
                ForEach(speeds) { fact in FactRow(fact: fact, onWrite: {}) }
            }
        } buttons: {
            Button("Done", action: onDismiss)
        }
    }
}
