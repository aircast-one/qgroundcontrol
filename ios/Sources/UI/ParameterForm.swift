import SwiftUI

struct ParameterRows: Equatable {
    let title: String
    let facts: [Fact]
    let note: String
    var calculators: [String: PowerCalculator] = [:]
    var image: String = ""
    var keywords: [String] = []
}

private func toDoubleOrNull(_ text: String) -> Double? { Double(text.trimmed) }

private func jsonNumber(_ value: JSON) -> Double? {
    if case .number(let number) = value { return number }
    return nil
}

func sectionMatches(_ rows: ParameterRows, _ search: String) -> Bool {
    let query = search.trimmed.lowercased()
    return query.isEmpty || ([rows.title] + rows.keywords).contains { $0.lowercased().contains(query) }
}

func factFromParameter(_ name: String, _ json: JSON) -> Fact? {
    guard json["kind"].string == "fact" else { return nil }
    var fact = Qgc.factAt(parameterPath(name), json)
    fact.name = name
    return fact
}

func readOnlyNote(_ facts: [Fact]) -> String? {
    let locked = facts.filter(\.readOnly)
    if locked.isEmpty { return nil }
    if locked.count == facts.count { return "This firmware reports all of these as read-only, so they are shown for reference." }
    return "This firmware reports " + locked.map(\.name).joined(separator: ", ") + " as read-only, so they are shown but cannot be changed here."
}

func factFromControl(_ control: JSON) -> Fact? {
    let label = control["label"].string
    let name = control["name"].string
    if label.isBlank && name.isBlank { return nil }
    let kind = control["control"].string
    let options = control["options"].array
    let labels = options.map { $0["label"].string }
    let bitEntries: [(String, Int64)] = kind == "bitmask"
        ? control["bits"].array.filter { $0.object != nil }.compactMap { bit in Int64(bit["raw"].string).map { (bit["label"].string, $0) } }
        : []
    return Fact(
        path: control["path"].string,
        name: name,
        description: label,
        units: control["units"].string,
        valueString: control["valueString"].string,
        value: control["value"],
        enumStrings: labels,
        enumValues: options.map { $0["raw"].string },
        enumIndex: labels.firstIndex(of: control["display"].string) ?? -1,
        controlKind: kind,
        bitmaskStrings: bitEntries.map(\.0),
        bitmaskValues: bitEntries.map(\.1),
        isBool: kind == "toggle",
        isString: kind == "text",
        wholeNumbersOnly: control["wholeNumbersOnly"].bool,
        readOnly: control["readOnly"].bool,
        enabled: control["enabled"].bool(true),
        disabledReason: control["disabledReason"].string,
        minString: control["minimumText"].string,
        maxString: control["maximumText"].string,
        minIsDefaultForType: control["minimumText"].isNull,
        maxIsDefaultForType: control["maximumText"].isNull,
        defaultValueString: control["defaultText"].string,
        vehicleRebootRequired: control["vehicleRebootRequired"].bool,
        qgcRebootRequired: control["applicationRestartRequired"].bool,
        warning: control["warning"].bool,
        optional: control["optional"].bool,
        slider: control["slider"].object != nil ? factSlider(control["slider"], control["description"].string) : nil,
        firstEntryIsAll: control["firstEntryIsAll"].bool,
        indent: control["indent"].bool,
        smallFont: control["smallFont"].bool,
        shortLabel: control["shortLabel"].string,
        keywords: control["keywords"].string,
        inverted: control["inverted"].bool,
        rawChoice: control["rawChoice"].bool,
        valueDetails: control["valueDetails"].string,
        problem: control["problem"].string,
        enumGroups: options.map { $0["group"].string }
    )
}

func readPage(_ page: String) -> [ParameterRows] {
    (Qgc.get(setupPagePath(page))["sections"].arrayOrNil ?? []).filter { $0.object != nil }.compactMap { section in
        let controls = section["controls"].array.filter { $0.object != nil }
        let facts = controls.compactMap(factFromControl)
        if facts.isEmpty { return nil }
        let note = [section["note"].string.isBlank ? nil : section["note"].string, readOnlyNote(facts)].compactMap { $0 }
        let calculators = Dictionary(
            controls.compactMap { row in powerCalculator(row["calculator"].object != nil ? row["calculator"] : nil).map { (row["path"].string, $0) } },
            uniquingKeysWith: { _, last in last }
        )
        return ParameterRows(
            title: section["title"].string,
            facts: facts,
            note: note.joined(separator: " "),
            calculators: calculators,
            image: section["image"].string,
            keywords: section["keywords"].array.map(\.string)
        )
    }
}

let SETUP_PAGE_OPENED = "setup.pageOpened"

let EMPTY_PAGE_TEXTS = ["Gimbal": "Gimbal settings are not available for this firmware version."]

struct ParameterForm: View {
    let page: String
    var highlighted: Set<String> = []
    var section: String? = nil
    var footer: AnyView? = nil
    @Environment(\.theme) private var theme
    @State private var rows: [ParameterRows] = []
    @State private var loaded = false
    @State private var reloads = 0
    @State private var calculating: PowerCalculator? = nil
    @State private var calibratingEscs = false

    private struct Load: Equatable {
        let page: String
        let section: String?
        let reloads: Int
    }

    var body: some View {
        content
            .task(id: Load(page: page, section: section, reloads: reloads)) {
                let page = page, section = section, first = reloads == 0
                rows = await offMain {
                    if first { Qgc.invoke(SETUP_PAGE_OPENED, page) }
                    return readPage(page).filter { section == nil || $0.title == section }
                }
                loaded = true
            }
            .queuedSheet(isPresented: $calibratingEscs) {
                EscCalibrationDialog(onClose: { calibratingEscs = false })
            }
            .queuedSheet(item: $calculating, onDismiss: { reloads += 1 }) { calculator in
                PowerCalcDialog(calculator: calculator, onDone: { calculating = nil })
            }
    }

    @ViewBuilder private var content: some View {
        if !loaded {
            Text("Reading parameters from the vehicle.").padding(16).frame(maxWidth: .infinity, alignment: .leading)
        } else if rows.isEmpty {
            EmptyState(icon: .tune, title: "Nothing to set here", text: EMPTY_PAGE_TEXTS[page] ?? "This vehicle exposes none of these parameters.")
        } else {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(rows, id: \.title) { section in
                        SectionHeader(text: sentenceCase(section.title), image: section.image)
                        if !section.note.isBlank {
                            Text(section.note)
                                .font(.bodySmall)
                                .foregroundStyle(theme.colors.onSurfaceVariant)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, 20)
                                .padding(.bottom, 12)
                        }
                        ForEach(section.facts) { fact in
                            factRow(fact, section)
                        }
                    }
                }
                footer
            }
        }
    }

    @ViewBuilder private func factRow(_ fact: Fact, _ section: ParameterRows) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            if fact.controlKind == DIALOG_CONTROL {
                Button(fact.title) { calibratingEscs = true }
                    .buttonStyle(.bordered)
                    .disabled(!fact.enabled)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 8)
            } else if fact.controlKind == BUTTON_CONTROL {
                Button(fact.title) {
                    let path = fact.path
                    Task { @MainActor in
                        let _: Bool = await offMain { Qgc.set(path, true) }
                        reloads += 1
                    }
                }
                .buttonStyle(.bordered)
                .disabled(!fact.enabled)
                .padding(.horizontal, 20)
                .padding(.vertical, 8)
            } else if fact.controlKind == LABEL_CONTROL {
                Text(fact.title)
                    .font(fact.smallFont ? .bodySmall : .bodyMedium)
                    .foregroundStyle((fact.warning ? theme.colors.error : theme.colors.onSurfaceVariant).opacity(fact.enabled ? 1 : DISABLED_LABEL_ALPHA))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 8)
            } else if let slider = fact.slider {
                FactSliderRow(fact: fact, slider: slider, onWrite: { reloads += 1 })
            } else {
                FactRow(fact: fact, titleColor: highlighted.contains(fact.name) ? theme.aircast.warning : nil, onWrite: { reloads += 1 })
                if let calculator = section.calculators[fact.path] {
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(sentenceCase(calculator.title)).font(.bodyLarge)
                            Text("Measure the \(calculator.measure) with a meter").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        Button("Calibrate") { calculating = calculator }.buttonStyle(.bordered)
                    }
                    .padding(16)
                    .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                }
            }
        }
        .padding(.leading, fact.indent ? INDENT : 0)
    }
}

extension PowerCalculator: Identifiable {
    var id: Self { self }
}

func bitmaskRaw(_ fact: Fact) -> Int64 {
    (jsonNumber(fact.value) ?? toDoubleOrNull(fact.valueString)).flatMap { $0.isFinite ? Int64(exactly: $0.rounded(.towardZero)) : nil } ?? 0
}

func bitmaskSummary(_ fact: Fact) -> String {
    let raw = bitmaskRaw(fact)
    let set = fact.bitmaskValues.indices.filter { raw & fact.bitmaskValues[$0] != 0 }
    if set.isEmpty { return "None" }
    if set.count == fact.bitmaskStrings.count { return "All" }
    return set.filter { fact.bitmaskStrings.indices.contains($0) }.map { fact.bitmaskStrings[$0] }.joined(separator: ", ")
}

let LABEL_CONTROL = "label"
let BUTTON_CONTROL = "button"

private let INDENT: CGFloat = 16

private let DISABLED_LABEL_ALPHA = 0.38

let KNOWN_CONTROL_KINDS: Set<String> = ["toggle", "choice", "bitmask", "text", "number", LABEL_CONTROL, DIALOG_CONTROL, BUTTON_CONTROL]

func controlIsUnderstood(_ kind: String) -> Bool { kind.isBlank || KNOWN_CONTROL_KINDS.contains(kind) }

func factSlider(_ json: JSON, _ hint: String) -> FactSlider? {
    guard let from = json["from"].double, !from.isNaN, let to = json["to"].double, !to.isNaN, to > from else { return nil }
    return FactSlider(from: from, to: to, decimals: json["decimals"].int(2), hint: hint)
}

private struct FactSliderRow: View {
    let fact: Fact
    let slider: FactSlider
    let onWrite: () -> Void
    @Environment(\.theme) private var theme
    @State private var shown: Double? = nil

    private var held: Double { jsonNumber(fact.value) ?? toDoubleOrNull(fact.valueString) ?? slider.from }

    private var range: ClosedRange<Double> { min(slider.from, slider.to)...max(slider.from, slider.to) }

    var body: some View {
        let current = min(max(shown ?? held, range.lowerBound), range.upperBound)
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(sliderTitle(fact)).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                Text(sliderValue(current, slider.decimals, fact.units)).font(.labelLarge).foregroundStyle(theme.colors.primary)
            }
            Slider(value: Binding(get: { current }, set: { shown = $0 }), in: range) { editing in
                guard !editing else { return }
                let path = fact.path, value = min(max(shown ?? held, range.lowerBound), range.upperBound), onWrite = onWrite
                Task { @MainActor in
                    _ = await offMain { Qgc.writeRefusal(path, value) }
                    onWrite()
                }
            }
            .disabled(!(fact.enabled && !fact.readOnly))
            if !slider.hint.isBlank {
                Text(slider.hint).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .onChange(of: held) { shown = nil }
        .onChange(of: fact.path) { shown = nil }
    }
}

func sliderTitle(_ fact: Fact) -> String { sentenceCase(fact.description.ifBlank(fact.heading)) }

func sliderValue(_ value: Double, _ decimals: Int, _ units: String) -> String {
    [String(format: "%.\(min(max(decimals, 0), 6))f", value), units].filter { !$0.isBlank }.joined(separator: " ")
}
