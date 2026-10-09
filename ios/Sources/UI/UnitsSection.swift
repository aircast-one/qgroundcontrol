import SwiftUI

private let UNITS_PATH = "settings.unitsSettings"

let UNIT_SYSTEM_CUSTOM = 2

let UNIT_SYSTEM_LABELS = ["Metric", "Imperial", "Custom"]

let PRESET_UNIT_FACTS = [
    "horizontalDistanceUnits",
    "verticalDistanceUnits",
    "areaUnits",
    "speedUnits",
    "temperatureUnits",
]

private let GENERAL_SETTINGS = "view.settings(General)"

func unitFacts(_ page: JSON?) -> [Fact] {
    guard let units = page?["sections"].arrayOrNil?.first(where: { $0["group"].string == "unitsSettings" }),
          let subsections = units["subsections"].arrayOrNil else { return [] }
    return subsections.flatMap { subsection in
        subsection["controls"].objects.compactMap { factFromControl($0) }
    }
}

func unitRowsFor(_ system: Int, _ facts: [Fact]) -> [Fact] {
    let custom = unitSystemLabel(system) == UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]
    return facts.filter { $0.name != "customUnits" && $0.name != "weightUnits" }
        .filter { custom || !PRESET_UNIT_FACTS.contains($0.name) }
}

func unitSystemLabel(_ system: Int) -> String {
    UNIT_SYSTEM_LABELS.indices.contains(system) ? UNIT_SYSTEM_LABELS[system] : UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]
}

func unitSystemNote(_ system: Int) -> String {
    unitSystemLabel(system) == UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]
        ? "Each measurement is set on its own below."
        : "Distance, area, speed and temperature all follow \(unitSystemLabel(system)). Choose Custom to set them one at a time."
}

private struct UnitSystemRow: View {
    let system: Int
    let onPick: (Int) -> Void

    var body: some View {
        let chosen = unitSystemLabel(system)
        VStack(alignment: .leading, spacing: Space.s1) {
            Text("Measurement system").font(.bodyLarge)
            Picker("Measurement system", selection: Binding(
                get: { UNIT_SYSTEM_LABELS.firstIndex(of: chosen) ?? UNIT_SYSTEM_CUSTOM },
                set: { index in if UNIT_SYSTEM_LABELS[index] != chosen { onPick(index) } }
            )) {
                ForEach(Array(UNIT_SYSTEM_LABELS.enumerated()), id: \.offset) { index, label in
                    Text(label).tag(index)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }
}

struct UnitsSection: View {
    @QgcPath(GENERAL_SETTINGS) private var page
    @QgcValue("\(UNITS_PATH).unitSystem") private var system

    var body: some View {
        if let chosen = system.int {
            VStack(alignment: .leading, spacing: 0) {
                UnitSystemRow(system: chosen) { picked in
                    offMain { _ = Qgc.invoke("\(UNITS_PATH).setUnitSystem", picked) }
                }
                FootNote(text: unitSystemNote(chosen))
                FactRuns(facts: unitRowsFor(chosen, unitFacts(page)))
            }
        }
    }
}
