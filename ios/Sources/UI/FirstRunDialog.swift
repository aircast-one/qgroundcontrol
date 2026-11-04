import SwiftUI

let FIRST_RUN_PATH = "view.firstRun"

struct FirstRun: Equatable {
    let title: String
    let vehicleHeading: String
    let vehicleDescription: String
    let preferences: [Fact]
    let unitsHeading: String
    let unitsDescription: String
}

func firstRun(_ view: JSON?) -> FirstRun? {
    guard let view, view["show"].bool else { return nil }
    return FirstRun(
        title: view["title"].string,
        vehicleHeading: view["vehicleHeading"].string,
        vehicleDescription: view["vehicleDescription"].string,
        preferences: view["vehiclePreferences"].objects.compactMap(factFromControl),
        unitsHeading: view["unitsHeading"].string,
        unitsDescription: view["unitsDescription"].string
    )
}

let FIRST_RUN_SYSTEMS = ["Metric System", "Imperial System"]

private let METERS = 1.0

private let FIRST_RUN_UNIT_VALUES: [String: (meters: Int, feet: Int)] = [
    "horizontalDistanceUnits": (1, 0),
    "verticalDistanceUnits": (1, 0),
    "areaUnits": (1, 0),
    "speedUnits": (1, 0),
    "temperatureUnits": (0, 1),
]

func firstRunUnitRows(_ facts: [Fact]) -> [Fact] { facts.filter { FIRST_RUN_UNIT_VALUES[$0.name] != nil } }

func firstRunSystemIndex(_ horizontalDistanceUnits: Double) -> Int { horizontalDistanceUnits == METERS ? 0 : 1 }

func firstRunSystemWrites(_ metric: Bool, _ rows: [Fact]) -> [(String, Int)] {
    rows.compactMap { fact in FIRST_RUN_UNIT_VALUES[fact.name].map { (fact.path, metric ? $0.meters : $0.feet) } }
}

private struct FirstRunUnits: View {
    @QgcPath("view.settings(General)") private var page
    @QgcDouble(settingControl("settings.unitsSettings.horizontalDistanceUnits")) private var horizontal

    var body: some View {
        let rows = firstRunUnitRows(unitFacts(page))
        let chosen = firstRunSystemIndex(horizontal)
        Picker("System of units", selection: Binding<Int>(get: { chosen }, set: { index in
            let writes = firstRunSystemWrites(index == 0, rows)
            offMainInOrder { writes.forEach { path, value in Qgc.set(path, value) } }
        })) {
            ForEach(Array(FIRST_RUN_SYSTEMS.enumerated()), id: \.offset) { index, label in
                Text(label).tag(index)
            }
        }
        FactRuns(facts: rows)
    }
}

let WELCOME_NOTE = "Two questions so numbers and controls match your drone."

private var appName: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String ?? "Aircast" }

struct FirstRunDialog: View {
    @QgcPath(FIRST_RUN_PATH) private var view

    var body: some View {
        let prompt = firstRun(view)
        Color.clear
            .invisibleAnchor()
            .fullScreenCover(isPresented: Binding(get: { prompt != nil }, set: { shown in if !shown { close() } })) {
                if let prompt { FirstRunPage(prompt: prompt, onClose: close) }
            }
    }

    private func close() { offMain { AppCommands.markFirstRunShown() } }
}

private struct FirstRunPage: View {
    let prompt: FirstRun
    let onClose: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        NavigationStack {
            NativeForm {
                Section {
                    VStack(spacing: Space.s3) {
                        Image(.flight)
                            .font(.system(size: 40))
                            .foregroundStyle(theme.colors.onPrimaryContainer)
                            .frame(width: 96, height: 96)
                            .background(theme.colors.primaryContainer, in: Circle())
                            .accessibilityHidden(true)
                        Text("Welcome to \(appName)").font(.title2.bold()).multilineTextAlignment(.center)
                        Text(WELCOME_NOTE)
                            .foregroundStyle(Color.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .listRowBackground(Color.clear)
                }
                Section {
                    FirstRunUnits()
                } header: {
                    SectionHeader(text: sentenceCase(prompt.unitsHeading))
                } footer: {
                    if !prompt.unitsDescription.isBlank { Text(prompt.unitsDescription) }
                }
                if !prompt.preferences.isEmpty {
                    Section {
                        ForEach(prompt.preferences) { fact in FactRow(fact: fact) }
                    } header: {
                        SectionHeader(text: sentenceCase(prompt.vehicleHeading))
                    } footer: {
                        if !prompt.vehicleDescription.isBlank { Text(prompt.vehicleDescription) }
                    }
                }
            }
            .navigationTitle(prompt.title.ifBlank("Welcome"))
            .navigationBarTitleDisplayMode(.inline)
            .pagePrimaryAction("Continue", action: onClose)
        }
    }
}
