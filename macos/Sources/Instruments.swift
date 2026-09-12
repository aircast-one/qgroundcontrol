import Foundation

final class InstrumentsStore: ObservableObject, Probeable {
    static let probeID = "instruments"

    static let storageKey = "instrumentSelections"
    static let maximumSlots = 10

    @Published private(set) var selections =
        InstrumentSelection.restore(UserDefaults.standard.stringArray(forKey: InstrumentsStore.storageKey))
    @Published private(set) var values: [InstrumentValue] = []
    @Published private(set) var groups: [InstrumentGroup] = []
    @Published private(set) var editingSlot: Int?
    @Published var showingEditor = false
    @Published var chosenGroup = InstrumentSelection.vehicleGroup

    func refresh() {
        let asked = selections.map(\.stored).joined(separator: ",")
        let read = InstrumentValue.list(Bridge.group("view.instruments(\(asked))")["items"])
        if read != values { values = read }
    }

    var chosenFacts: [InstrumentFact] {
        groups.first(where: { $0.group == chosenGroup })?.facts ?? []
    }

    var editingLabel: String {
        guard let slot = editingSlot, values.indices.contains(slot) else { return "" }
        return "Currently \(values[slot].label)"
    }

    var canAdd: Bool { selections.count < InstrumentsStore.maximumSlots }

    var canRemove: Bool { selections.count > 1 }

    func addSlot() {
        guard canAdd else { return }
        selections.append(InstrumentSelection.firstUnused(in: selections))
        save()
        refresh()
    }

    func removeSlot(_ slot: Int) {
        guard canRemove, selections.indices.contains(slot) else { return }
        selections.remove(at: slot)
        save()
        refresh()
    }

    func resetSlots() {
        selections = InstrumentSelection.defaults
        save()
        refresh()
    }

    private func save() {
        UserDefaults.standard.set(selections.map(\.stored), forKey: InstrumentsStore.storageKey)
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
        save()
        refresh()
        cancelEdit()
    }

    func discoverGroups() {
        let vehicle = Bridge.group("vehicle")
        let children = (vehicle["children"] as? [String]) ?? []
        let packs = ((Bridge.group("vehicle.batteries")["elements"] as? [Any]) ?? []).count
        let candidates = [InstrumentSelection.vehicleGroup] + children
            + InstrumentGroup.batteryGroups(count: packs)
        let read = candidates.map { group in
            (group: group,
             json: group == InstrumentSelection.vehicleGroup
                 ? vehicle
                 : Bridge.group("vehicle.\(group)"))
        }
        let assembled = InstrumentGroup.assemble(read, label: Labels.humanise)
        if assembled != groups { groups = assembled }
    }

    func clear() {
        if !values.isEmpty { values = [] }
    }

    func probeState() -> [String: Any] {
        ["count": values.count, "editingSlot": editingSlot ?? -1,
         "editorOpen": showingEditor, "chosenGroup": chosenGroup,
         "groups": groups.map { ["group": $0.group, "title": $0.title, "facts": $0.facts.count] },
         "stored": selections.map(\.stored), "canAdd": canAdd, "canRemove": canRemove,
         "values": values.map { ["label": $0.label, "value": $0.value, "units": $0.units,
                                 "fact": $0.id, "missing": $0.missing] }]
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
        case "add": addSlot()
        case "remove":
            guard let slot = Int(args["slot"] ?? ""), selections.indices.contains(slot) else {
                return ["ok": false, "error": "no slot \(args["slot"] ?? "")"]
            }
            removeSlot(slot)
        case "reset": resetSlots()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
