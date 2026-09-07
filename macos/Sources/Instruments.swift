import Foundation

final class InstrumentsStore: ObservableObject, Probeable {
    static let probeID = "instruments"

    @Published private(set) var selections = InstrumentSelection.defaults
    @Published private(set) var values: [InstrumentValue] = []
    @Published private(set) var groups: [InstrumentGroup] = []
    @Published private(set) var editingSlot: Int?
    @Published var showingEditor = false
    @Published var chosenGroup = InstrumentSelection.vehicleGroup

    func refresh() {
        let groups = Set(selections.map(\.path))
        let facts = groups.reduce(into: [String: [[String: Any]]]()) { store, path in
            store[path] = (Bridge.group(path)["facts"] as? [[String: Any]]) ?? []
        }
        let read = selections.map { InstrumentValue.resolve($0, in: facts[$0.path] ?? []) }
        if read != values { values = read }
    }

    var chosenFacts: [InstrumentFact] {
        groups.first(where: { $0.group == chosenGroup })?.facts ?? []
    }

    var editingLabel: String {
        guard let slot = editingSlot, values.indices.contains(slot) else { return "" }
        return "Currently \(values[slot].label)"
    }

    func edit(slot: Int) {
        guard selections.indices.contains(slot) else { return }
        editingSlot = slot
        chosenGroup = selections[slot].group
        discoverGroups()
        showingEditor = true
    }

    func cancelEdit() {
        showingEditor = false
        editingSlot = nil
    }

    func assign(group: String, factName: String) {
        guard let slot = editingSlot, selections.indices.contains(slot) else { return }
        selections[slot] = InstrumentSelection(group: group, factName: factName)
        refresh()
        cancelEdit()
    }

    func discoverGroups() {
        let vehicle = Bridge.group("vehicle")
        let children = (vehicle["children"] as? [String]) ?? []
        let candidates = [InstrumentSelection.vehicleGroup] + children + ["batteries.0"]
        let read = candidates.map { group in
            (group: group,
             json: group == InstrumentSelection.vehicleGroup
                 ? vehicle
                 : Bridge.group("vehicle.\(group)"))
        }
        let assembled = InstrumentGroup.assemble(read)
        if assembled != groups { groups = assembled }
    }

    func clear() {
        if !values.isEmpty { values = [] }
    }

    func probeState() -> [String: Any] {
        ["count": values.count, "editingSlot": editingSlot ?? -1,
         "editorOpen": showingEditor, "chosenGroup": chosenGroup,
         "groups": groups.map { ["group": $0.group, "title": $0.title, "facts": $0.facts.count] },
         "values": values.map { ["label": $0.label, "value": $0.value, "units": $0.units,
                                 "fact": $0.selection.id] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "groups": discoverGroups()
        case "edit":
            guard let slot = Int(args["slot"] ?? ""), selections.indices.contains(slot) else {
                return ["ok": false, "error": "no slot \(args["slot"] ?? "")"]
            }
            edit(slot: slot)
        case "chooseGroup": chosenGroup = args["group"] ?? InstrumentSelection.vehicleGroup
        case "assign":
            guard editingSlot != nil else {
                return ["ok": false, "error": "no slot is being edited"]
            }
            guard let factName = args["fact"], !factName.isEmpty else {
                return ["ok": false, "error": "assign needs a fact"]
            }
            assign(group: args["group"] ?? InstrumentSelection.vehicleGroup, factName: factName)
        case "cancelEdit": cancelEdit()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
