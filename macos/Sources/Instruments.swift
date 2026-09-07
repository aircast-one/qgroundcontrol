import Foundation

final class InstrumentsStore: ObservableObject, Probeable {
    static let probeID = "instruments"

    @Published private(set) var selections = InstrumentSelection.defaults
    @Published private(set) var values: [InstrumentValue] = []

    func refresh() {
        let groups = Set(selections.map(\.path))
        let facts = groups.reduce(into: [String: [[String: Any]]]()) { store, path in
            store[path] = (Bridge.group(path)["facts"] as? [[String: Any]]) ?? []
        }
        let read = selections.map { InstrumentValue.resolve($0, in: facts[$0.path] ?? []) }
        if read != values { values = read }
    }

    func clear() {
        if !values.isEmpty { values = [] }
    }

    func probeState() -> [String: Any] {
        ["count": values.count,
         "values": values.map { ["label": $0.label, "value": $0.value, "units": $0.units,
                                 "fact": $0.selection.id] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
