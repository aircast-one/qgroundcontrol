import SwiftUI

private let PARAMETER_MANAGER = "vehicle.parameterManager"
private let DEFAULT_COMPONENT = -1

private let AUTOPILOT_COMPONENT = 1

private func afterColon(_ key: String) -> String {
    key.firstIndex(of: ":").map { String(key[key.index(after: $0)...]) } ?? key
}

func parameterPath(_ key: String, _ quantity: String? = nil) -> String {
    let component = key.firstIndex(of: ":").flatMap { Int(key[..<$0]) }
    return "\(PARAMETER_MANAGER).getParameter(\(component ?? DEFAULT_COMPONENT),\(component == nil ? key : afterColon(key))\(quantity.map { ",\($0)" } ?? ""))"
}

func parameterKeys(_ namesByComponent: [(Int, [String])]) -> [String] {
    namesByComponent.flatMap { component, names in
        names.sorted().map { component == AUTOPILOT_COMPONENT || component == DEFAULT_COMPONENT ? $0 : "\(component):\($0)" }
    }
}

private struct ParameterLoad: Equatable {
    let ready: Bool
    let reads: Int
}

private struct ParameterQuery: Equatable {
    let loads: Int
    let search: String
    let modifiedOnly: Bool
    let category: String?
    let group: String?
}

struct ParametersScreen: View {
    var initialSearch: String = ""
    @QgcPath(SETUP) private var setupJson
    @State private var search: String
    @State private var names: [String] = []
    @State private var descriptions: [String: [String]] = [:]
    @State private var modified: Set<String> = []
    @State private var modifiedChosen = false
    @State private var placement: [String: (String, String)] = [:]
    @State private var tree: [ParameterCategory] = []
    @State private var matches: [String] = []
    @State private var loads = 0
    @State private var chosenCategory: String?
    @State private var chosenGroup: String?
    @State private var reads = 0

    init(initialSearch: String = "") {
        self.initialSearch = initialSearch
        _search = State(initialValue: initialSearch)
    }

    var body: some View {
        let ready = parametersReady(setupJson)
        let px4 = isPx4(setupReadiness(setupJson))
        let modifiedOnly = modifiedFilterOn(modifiedChosen, px4)
        let browsing = search.isBlank && !modifiedOnly
        let category = tree.first { $0.name == chosenCategory } ?? tree.first
        let group = category.flatMap { shown in shown.groups.first { $0 == chosenGroup } ?? shown.groups.first }
        let query = ParameterQuery(loads: loads, search: search, modifiedOnly: modifiedOnly, category: category?.name, group: group)
        List {
            if !ready {
                Section {
                    HStack(spacing: Space.s3) {
                        ProgressView()
                        Text("Waiting for parameters from the vehicle.").foregroundStyle(Color.secondary)
                    }
                }
            } else {
                Section {
                    LabeledContent {
                        ParameterToolsMenu(onRefreshed: { reads += 1 })
                    } label: {
                        Text(parameterCountLine(matches.count, modified.count)).foregroundStyle(Color.secondary)
                    }
                    if px4 {
                        Picker("Show", selection: $modifiedChosen) {
                            Text("All").tag(false)
                            Text("Changed").tag(true)
                        }
                        .pickerStyle(.segmented)
                    }
                    if browsing, let category {
                        Picker("Category", selection: Binding(get: { category.name }, set: { picked in
                            chosenCategory = picked
                            chosenGroup = nil
                        })) {
                            ForEach(tree.map(\.name), id: \.self) { Text($0).tag($0) }
                        }
                        if let group {
                            Picker("Group", selection: Binding(get: { group }, set: { chosenGroup = $0 })) {
                                ForEach(category.groups, id: \.self) { Text($0).tag($0) }
                            }
                        }
                    }
                }
                Section {
                    ForEach(matches, id: \.self) { ParameterRow(name: $0, offersRcToParam: px4) }
                }
            }
        }
        .parameterListStyle()
        .environment(\.LocalNativeList, true)
        .searchable(text: $search, prompt: "Search parameters")
        .onChange(of: query, initial: true) {
            matches = names
                .filter { parameterShown($0, descriptions[$0] ?? [], search, modifiedOnly, modified) }
                .filter { !browsing || inGroup($0, placement, category?.name, group) }
        }
        .task(id: ParameterLoad(ready: ready, reads: reads)) {
            let loaded = ready ? await offMain { parameterNames() } : []
            guard !Task.isCancelled else { return }
            names = loaded
            loads += 1
            guard !loaded.isEmpty else {
                tree = []
                return
            }
            let (summary, built) = await offMain {
                let summary = parameterSummary(loaded)
                return (summary, parameterTree(loaded, summary.placement))
            }
            guard !Task.isCancelled else { return }
            descriptions = summary.descriptions
            modified = summary.modified
            placement = summary.placement
            tree = built
            loads += 1
        }
    }
}

private struct ParameterRowKey: Equatable {
    let name: String
    let revision: Int
    let live: JSON?
}

private struct ParameterRow: View {
    let name: String
    let offersRcToParam: Bool
    @QgcPath private var live: JSON?
    @State private var revision = 0
    @State private var mapping = false
    @State private var forcing = false
    @State private var fact: Fact?
    @Environment(\.theme) private var theme

    init(name: String, offersRcToParam: Bool) {
        self.name = name
        self.offersRcToParam = offersRcToParam
        _live = QgcPath(parameterPath(name))
    }

    var body: some View {
        VStack(spacing: 0) {
            if let loaded = fact {
                row(loaded)
            } else {
                Text(name)
                    .foregroundStyle(Color.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, Space.s2)
            }
        }
        .task(id: ParameterRowKey(name: name, revision: revision, live: live)) {
            let name = name
            let loaded = await offMain { parameterFact(name) }
            guard !Task.isCancelled else { return }
            fact = loaded
        }
    }

    private func row(_ loaded: Fact) -> some View {
        HStack(spacing: Space.s3) {
            VStack(alignment: .leading, spacing: 2) {
                Text(loaded.name).lineLimit(1)
                if !loaded.description.isBlank {
                    Text(loaded.description).font(.subheadline).foregroundStyle(Color.secondary).lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(parameterValueText(loaded))
                .foregroundStyle(loaded.changedFromDefault ? theme.colors.primary : Color.secondary)
                .lineLimit(2)
                .multilineTextAlignment(.trailing)
                .frame(maxWidth: 160, alignment: .trailing)
            if offersRcToParam && !loaded.readOnly && !name.contains(":") {
                Button("RC") { mapping = true }.buttonStyle(.borderless)
            }
        }
        .contentShape(Rectangle())
        .macHover()
        .onTapGesture { forcing = true }
        .background {
            if forcing {
                ParameterEditDialog(name: name) {
                    forcing = false
                    revision += 1
                }
            }
        }
        .sheet(isPresented: $mapping) {
            RcToParamDialog(fact: loaded) { mapping = false }
        }
    }
}

func modifiedFilterOn(_ chosen: Bool, _ px4: Bool) -> Bool { chosen && px4 }

func parameterShown(_ name: String, _ descriptions: [String], _ search: String, _ modifiedOnly: Bool, _ modified: Set<String>) -> Bool {
    parameterMatches(name, descriptions, search) && (!modifiedOnly || modified.contains(name))
}

func parameterMatches(_ name: String, _ descriptions: [String], _ search: String) -> Bool {
    search.split(separator: " ").map(String.init).allSatisfy { word in
        let pattern = try? NSRegularExpression(pattern: word, options: .caseInsensitive)
        return ([afterColon(name)] + descriptions).contains { text in
            pattern.map { $0.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) != nil }
                ?? (text.range(of: word, options: .caseInsensitive) != nil)
        }
    }
}

func parameterCountLine(_ shown: Int, _ changed: Int) -> String {
    let us = Locale(identifier: "en_US")
    return [
        "\(shown.formatted(.number.locale(us))) parameter\(shown == 1 ? "" : "s")",
        changed > 0 ? "\(changed.formatted(.number.locale(us))) changed" : nil,
    ].compactMap { $0 }.joined(separator: " \u{00b7} ")
}

func parameterValueText(_ fact: Fact) -> String {
    fact.isBitmask
        ? bitmaskSummary(fact)
        : [enumLabel(fact), fact.isEnum ? "" : fact.units].filter { !$0.isBlank }.joined(separator: " ")
}

func parameterSubtitle(_ description: String, _ units: String) -> String {
    [description, units].filter { !$0.isBlank }.joined(separator: " · ")
}

func parameterNames() -> [String] {
    let names: (Int) -> [String] = { component in
        Qgc.invokeResult("\(PARAMETER_MANAGER).parameterNames", component).strings
    }
    let components = Qgc.invokeResult("\(PARAMETER_MANAGER).componentIds").arrayOrNil?.map { $0.int(0) } ?? []
    return parameterKeys(components.isEmpty ? [(DEFAULT_COMPONENT, names(DEFAULT_COMPONENT))] : components.map { ($0, names($0)) })
}

struct ParameterSummary {
    var descriptions: [String: [String]]
    var modified: Set<String>
    var placement: [String: (String, String)] = [:]
}

struct ParameterCategory: Equatable {
    var name: String
    var groups: [String]
}

private let STANDARD_CATEGORY = "Standard"
private let DEFAULT_CATEGORY = "Other"
private let DEFAULT_GROUP = "Misc"

func parameterTree(_ names: [String], _ placement: [String: (String, String)]) -> [ParameterCategory] {
    let placed = names.compactMap { placement[$0] }
    let categories = placed.map(\.0).distinct()
    let ordered = categories.filter { $0 == STANDARD_CATEGORY }
        + categories.filter { $0 != STANDARD_CATEGORY && $0 != DEFAULT_CATEGORY }
        + categories.filter { $0 == DEFAULT_CATEGORY }
    return ordered.map { category in
        let groups = placed.filter { $0.0 == category }.map(\.1).distinct()
        return ParameterCategory(name: category, groups: groups.filter { $0 != DEFAULT_GROUP } + groups.filter { $0 == DEFAULT_GROUP })
    }
}

func inGroup(_ name: String, _ placement: [String: (String, String)], _ category: String?, _ group: String?) -> Bool {
    guard let category, let group else { return true }
    return placement[name].map { $0.0 == category && $0.1 == group } ?? false
}

private func parameterSummary(_ names: [String]) -> ParameterSummary {
    let facts = names.compactMap { name in parameterFact(name).map { (name, $0) } }
    return ParameterSummary(
        descriptions: Dictionary(facts.map { name, fact in (name, [fact.description, fact.longDescription].filter { !$0.isBlank }) }, uniquingKeysWith: { _, last in last }),
        modified: Set(facts.filter { $0.1.changedFromDefault }.map(\.0)),
        placement: Dictionary(facts.map { name, fact in (name, (fact.category, fact.group)) }, uniquingKeysWith: { _, last in last })
    )
}

func parameterFact(_ name: String, _ quantity: String? = nil) -> Fact? {
    factFromParameter(name, Qgc.get(parameterPath(name, quantity)), quantity)
}

extension View {
    // The parameter list: a lazy List (thousands of rows, each watching its value), inset-grouped like a NativeForm.
    fileprivate func parameterListStyle() -> some View {
        #if os(macOS)
        return listStyle(.inset)
        #else
        return listStyle(.insetGrouped)
        #endif
    }
}
