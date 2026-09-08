import Foundation

struct InstrumentSelection: Equatable, Identifiable {
    let group: String
    let factName: String

    var id: String { "\(group)/\(factName)" }

    var path: String { group.isEmpty ? "vehicle" : "vehicle.\(group)" }

    static let vehicleGroup = ""

    static func vehicle(_ factName: String) -> InstrumentSelection {
        InstrumentSelection(group: vehicleGroup, factName: factName)
    }

    static let separator: Character = "/"

    var stored: String { "\(group)\(InstrumentSelection.separator)\(factName)" }

    static func decode(_ stored: String) -> InstrumentSelection? {
        guard let slash = stored.firstIndex(of: separator) else { return nil }
        let factName = String(stored[stored.index(after: slash)...])
        guard !factName.isEmpty else { return nil }
        return InstrumentSelection(group: String(stored[stored.startIndex..<slash]), factName: factName)
    }

    static func decode(_ stored: [String]) -> [InstrumentSelection] {
        stored.compactMap(decode)
    }

    static func restore(_ stored: [String]?) -> [InstrumentSelection] {
        guard let stored, !stored.isEmpty else { return defaults }
        let listed = decode(stored)
        return listed.isEmpty ? defaults : listed
    }

    static func firstUnused(in used: [InstrumentSelection]) -> InstrumentSelection {
        let taken = Set(used.map(\.id))
        return defaults.first { !taken.contains($0.id) } ?? defaults[0]
    }

    static let defaults: [InstrumentSelection] = [
        .vehicle("altitudeRelative"),
        .vehicle("groundSpeed"),
        .vehicle("climbRate"),
        .vehicle("distanceToHome"),
        .vehicle("heading"),
        .vehicle("altitudeAMSL"),
    ]
}

struct InstrumentFact: Identifiable, Equatable {
    let name: String
    let label: String

    var id: String { name }
}

struct InstrumentGroup: Identifiable, Equatable {
    let group: String
    let title: String
    let facts: [InstrumentFact]

    var id: String { group }

    static let vehicleTitle = "Vehicle"

    static func title(for group: String) -> String {
        guard !group.isEmpty else { return vehicleTitle }
        if group == "batteries.0" { return "Battery 1" }
        return InstrumentValue.label(for: group)
    }

    static func facts(in json: [String: Any]) -> [InstrumentFact] {
        ((json["facts"] as? [[String: Any]]) ?? []).compactMap { fact in
            guard let name = fact["name"] as? String, !name.isEmpty else { return nil }
            let described = (fact["shortDescription"] as? String) ?? ""
            return InstrumentFact(name: name,
                                  label: described.isEmpty ? InstrumentValue.label(for: name) : described)
        }
    }

    static let vehicleAlias = "vehicle"

    static func assemble(_ read: [(group: String, json: [String: Any])]) -> [InstrumentGroup] {
        read.compactMap { entry in
            guard entry.group != vehicleAlias else { return nil }
            let listed = facts(in: entry.json)
            guard !listed.isEmpty else { return nil }
            return InstrumentGroup(group: entry.group, title: title(for: entry.group), facts: listed)
        }
    }
}

struct InstrumentValue: Identifiable, Equatable {
    let selection: InstrumentSelection
    let label: String
    let value: String
    let units: String

    var id: String { selection.id }

    var missing: Bool { value == InstrumentValue.absent }

    static let absent = "\u{2014}"

    static func resolve(_ selection: InstrumentSelection, in facts: [[String: Any]]) -> InstrumentValue {
        guard let fact = facts.first(where: { $0["name"] as? String == selection.factName }) else {
            return InstrumentValue(selection: selection, label: label(for: selection.factName),
                                   value: absent, units: "")
        }
        let described = (fact["shortDescription"] as? String) ?? ""
        let reading = (fact["valueString"] as? String) ?? ""
        return InstrumentValue(
            selection: selection,
            label: described.isEmpty ? label(for: selection.factName) : described,
            value: reading.isEmpty ? absent : reading,
            units: Units.display((fact["units"] as? String) ?? ""))
    }

    static func label(for factName: String) -> String { Fact.humanise(factName) }
}
