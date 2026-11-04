import SwiftUI

struct ParameterRows: Equatable {
    let title: String
    let facts: [Fact]
    let note: String
    var calculators: [String: PowerCalculator] = [:]
    var image: String = ""
    var keywords: [String] = []
}

func sectionMatches(_ rows: ParameterRows, _ search: String) -> Bool {
    let query = search.trimmed.lowercased()
    return query.isEmpty || ([rows.title] + rows.keywords).contains { $0.lowercased().contains(query) }
}

func factFromParameter(_ name: String, _ json: JSON, _ quantity: String? = nil) -> Fact? {
    guard json["kind"].string == "fact", !json["name"].string.isEmpty else { return nil }
    var fact = Qgc.factAt(parameterPath(name, quantity), json)
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
        ? control["bits"].objects.compactMap { bit in Int64(bit["raw"].string).map { (bit["label"].string, $0) } }
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
        let controls = section["controls"].objects
        let facts = controls.compactMap(factFromControl)
        if facts.isEmpty { return nil }
        let note = [section["note"].string, readOnlyNote(facts) ?? ""].filter { !$0.isBlank }
        let calculators = Dictionary(
            controls.compactMap { row in powerCalculator(row["calculator"].objectOrNil).map { (row["path"].string, $0) } },
            uniquingKeysWith: { _, last in last }
        )
        return ParameterRows(
            title: section["title"].string,
            facts: facts,
            note: note.joined(separator: " "),
            calculators: calculators,
            image: section["image"].string,
            keywords: section["keywords"].strings
        )
    }
}

let SETUP_PAGE_OPENED = "setup.pageOpened"

let READING_PARAMETERS = "Reading parameters from the vehicle."

func parameterFormWait(_ setup: JSON?) -> String? {
    parametersReady(setup) ? nil : (setup?["parametersText"].string ?? "").trimmingCharacters(in: .whitespacesAndNewlines).ifBlank(READING_PARAMETERS)
}

let EMPTY_PAGE_TEXTS = ["Gimbal": "Gimbal settings are not available for this firmware version."]

// A setup page's parameter sections. Pushed as a page (inside a host's nativePage) it is a NativeForm, a Section per
// parameter group; placed inside another NativeForm it is one row with the groups stacked; in a fly-view sheet it
// keeps its hand-laid-out column.
struct ParameterForm: View {
    let page: String
    var highlighted: Set<String> = []
    var section: String? = nil
    var header: AnyView? = nil
    var footer: AnyView? = nil
    @Environment(\.theme) private var theme
    @Environment(\.LocalPageHeading) private var pageHost
    @Environment(\.LocalNativeList) private var inForm
    @QgcPath(SETUP) private var setupJson
    @State private var rows: [ParameterRows] = []
    @State private var loaded = false
    @State private var reloads = 0
    @State private var calculating: PowerCalculator? = nil
    @State private var calibratingEscs = false

    private struct Load: Equatable {
        let page: String
        let section: String?
        let reloads: Int
        let ready: Bool
    }

    // The loading and the dialogs hang on one view. Inside a host's form that is a single row: modifiers on a group
    // of Sections would run once per section.
    var body: some View {
        if inForm {
            Section { hosted(formColumn) }
        } else if pageHost != nil {
            hosted(NativeForm { nativeSections })
        } else {
            hosted(content)
        }
    }

    private func hosted<Shown: View>(_ shown: Shown) -> some View {
        shown
            .task(id: Load(page: page, section: section, reloads: reloads, ready: parameterFormWait(setupJson) == nil)) {
                guard parameterFormWait(setupJson) == nil else { return }
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

    // Inside another NativeForm: the groups stacked in one row, each under its small heading, the rows divided.
    private var formColumn: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            header
            if let waiting = parameterFormWait(setupJson) {
                Text(waiting).foregroundStyle(Color.secondary)
            } else if !loaded {
                HStack(spacing: Space.s3) {
                    ProgressView()
                    Text(READING_PARAMETERS).foregroundStyle(Color.secondary)
                }
            } else if rows.isEmpty {
                Text(EMPTY_PAGE_TEXTS[page] ?? "This vehicle exposes none of these parameters.").foregroundStyle(Color.secondary)
            } else {
                ForEach(rows, id: \.title) { section in
                    VStack(alignment: .leading, spacing: Space.s2) {
                        if !section.title.isBlank {
                            Text(sentenceCase(section.title)).font(.subheadline.weight(.semibold)).foregroundStyle(Color.secondary)
                        }
                        ForEach(section.facts) { fact in
                            factRow(fact, section)
                            Divider()
                        }
                        if !section.note.isBlank {
                            Text(section.note).font(.footnote).foregroundStyle(Color.secondary)
                        }
                    }
                }
            }
            footer
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder private var nativeSections: some View {
        if let header {
            Section { header }
        }
        if let waiting = parameterFormWait(setupJson) {
            Section { Text(waiting).foregroundStyle(Color.secondary) }
        } else if !loaded {
            Section {
                HStack(spacing: Space.s3) {
                    ProgressView()
                    Text(READING_PARAMETERS).foregroundStyle(Color.secondary)
                }
            }
        } else if rows.isEmpty {
            EmptyState(icon: .tune, title: "Nothing to set here", text: EMPTY_PAGE_TEXTS[page] ?? "This vehicle exposes none of these parameters.")
                .listRowBackground(Color.clear)
        } else {
            ForEach(rows, id: \.title) { section in
                Section {
                    ForEach(section.facts) { fact in
                        factRow(fact, section)
                    }
                } header: {
                    if !section.title.isBlank { SectionHeader(text: sentenceCase(section.title), image: section.image) }
                } footer: {
                    if !section.note.isBlank { Text(section.note) }
                }
            }
        }
        if let footer {
            Section { footer }
        }
    }

    @ViewBuilder private var content: some View {
        if let waiting = parameterFormWait(setupJson) {
            Text(waiting).padding(16).frame(maxWidth: .infinity, alignment: .leading)
        } else if !loaded {
            Text(READING_PARAMETERS).padding(16).frame(maxWidth: .infinity, alignment: .leading)
        } else if rows.isEmpty {
            EmptyState(icon: .tune, title: "Nothing to set here", text: EMPTY_PAGE_TEXTS[page] ?? "This vehicle exposes none of these parameters.")
        } else {
            ScrollView {
                header
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
        if fact.controlKind == DIALOG_CONTROL {
            Button(fact.title) { calibratingEscs = true }
                .modifier(RowActionLook(inForm: inForm || pageHost != nil))
                .disabled(!fact.enabled)
                .rowPadding(horizontal: 20, vertical: 8)
                .padding(.leading, fact.indent ? INDENT : 0)
        } else if fact.controlKind == BUTTON_CONTROL {
            Button(fact.title) {
                let path = fact.path
                Task { @MainActor in
                    let _: Bool = await offMain { Qgc.set(path, true) }
                    reloads += 1
                }
            }
            .modifier(RowActionLook(inForm: inForm || pageHost != nil))
            .disabled(!fact.enabled)
            .rowPadding(horizontal: 20, vertical: 8)
            .padding(.leading, fact.indent ? INDENT : 0)
        } else if fact.controlKind == LABEL_CONTROL {
            Text(fact.title)
                .font(fact.smallFont ? .bodySmall : .bodyMedium)
                .foregroundStyle((fact.warning ? theme.colors.error : theme.colors.onSurfaceVariant).opacity(fact.enabled ? 1 : DISABLED_ALPHA))
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(horizontal: 20, vertical: 8)
                .padding(.leading, fact.indent ? INDENT : 0)
        } else if let slider = fact.slider {
            FactSliderRow(fact: fact, slider: slider, onWrite: { reloads += 1 })
                .padding(.leading, fact.indent ? INDENT : 0)
        } else {
            FactRow(fact: fact, titleColor: highlighted.contains(fact.name) ? theme.aircast.warning : nil, onWrite: { reloads += 1 })
                .padding(.leading, fact.indent ? INDENT : 0)
            if let calculator = section.calculators[fact.path] {
                CalculatorRow(calculator: calculator) { calculating = calculator }
                    .padding(.leading, fact.indent ? INDENT : 0)
            }
        }
    }
}

// A content button: a tinted text row inside a form, a bordered button on a hand-laid-out page.
private struct RowActionLook: ViewModifier {
    let inForm: Bool

    func body(content: Content) -> some View {
        if inForm {
            content
        } else {
            content.buttonStyle(.bordered)
        }
    }
}

// The row under a battery fact that opens its calculator: a native row in a form, a card on a hand-laid-out page.
private struct CalculatorRow: View {
    let calculator: PowerCalculator
    let onOpen: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            HStack(spacing: Space.s3) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(sentenceCase(calculator.title))
                    Text("Measure the \(calculator.measure) with a meter").font(.footnote).foregroundStyle(Color.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Button("Calibrate", action: onOpen).buttonStyle(.bordered)
            }
        } else {
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(sentenceCase(calculator.title)).font(.bodyLarge)
                    Text("Measure the \(calculator.measure) with a meter").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Button("Calibrate", action: onOpen).buttonStyle(.bordered)
            }
            .padding(16)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
    }
}

extension PowerCalculator: Identifiable {
    var id: Self { self }
}

func bitmaskRaw(_ fact: Fact) -> Int64 {
    (fact.value.numberOrNil ?? fact.valueString.doubleOrNil).flatMap { $0.isFinite ? Int64(exactly: $0.rounded(.towardZero)) : nil } ?? 0
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

    private var held: Double { fact.value.numberOrNil ?? fact.valueString.doubleOrNil ?? slider.from }

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
        .rowPadding(horizontal: 16, vertical: 8)
        .onChange(of: held) { shown = nil }
        .onChange(of: fact.path) { shown = nil }
    }
}

func sliderTitle(_ fact: Fact) -> String { sentenceCase(fact.description.ifBlank(fact.heading)) }

func sliderValue(_ value: Double, _ decimals: Int, _ units: String) -> String {
    [String(format: "%.\(min(max(decimals, 0), 6))f", value), units].filter { !$0.isBlank }.joined(separator: " ")
}
