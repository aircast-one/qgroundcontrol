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
        preferences: view["vehiclePreferences"].array.filter { $0.object != nil }.compactMap(factFromControl),
        unitsHeading: view["unitsHeading"].string,
        unitsDescription: view["unitsDescription"].string
    )
}

private let FIRST_RUN_WIDTH: CGFloat = 560

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
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                Text("System of units").font(.bodyLarge)
                HStack(spacing: 8) {
                    ForEach(Array(FIRST_RUN_SYSTEMS.enumerated()), id: \.offset) { index, label in
                        PlanChip(label: label, selected: index == chosen) {
                            let writes = firstRunSystemWrites(index == 0, rows)
                            offMainInOrder { writes.forEach { path, value in Qgc.set(path, value) } }
                        }
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            FactRuns(facts: rows)
        }
    }
}

let WELCOME_NOTE = "Two questions so numbers and controls match your drone."

private var appName: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String ?? "Aircast" }

struct FirstRunDialog: View {
    @QgcPath(FIRST_RUN_PATH) private var view

    var body: some View {
        let prompt = firstRun(view)
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
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
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                VStack(spacing: 16) {
                    Image(.flight)
                        .font(.system(size: 40))
                        .foregroundStyle(theme.colors.onPrimaryContainer)
                        .frame(width: 96, height: 96)
                        .background(theme.colors.primaryContainer, in: Circle())
                    Text("Welcome to \(appName)").font(.headlineSmall).multilineTextAlignment(.center)
                    Text(WELCOME_NOTE)
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity)
                SectionHeader(text: sentenceCase(prompt.unitsHeading))
                Text(prompt.unitsDescription)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, 16)
                FirstRunUnits()
                if !prompt.preferences.isEmpty {
                    SectionHeader(text: sentenceCase(prompt.vehicleHeading))
                    Text(prompt.vehicleDescription)
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .padding(.horizontal, 16)
                    ForEach(prompt.preferences) { fact in FactRow(fact: fact) }
                }
                HStack {
                    Spacer()
                    Button("Continue", action: onClose).buttonStyle(.filled)
                }
                .padding(.horizontal, 16)
            }
            .frame(maxWidth: FIRST_RUN_WIDTH)
            .padding(.horizontal, 8)
            .padding(.vertical, 48)
            .frame(maxWidth: .infinity)
        }
        .clipped()
        .background(theme.colors.surface.ignoresSafeArea())
    }
}
