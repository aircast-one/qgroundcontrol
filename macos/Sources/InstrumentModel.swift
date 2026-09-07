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

    static let defaults: [InstrumentSelection] = [
        .vehicle("altitudeRelative"),
        .vehicle("groundSpeed"),
        .vehicle("climbRate"),
        .vehicle("distanceToHome"),
        .vehicle("heading"),
        .vehicle("altitudeAMSL"),
    ]
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
            units: (fact["units"] as? String) ?? "")
    }

    static func label(for factName: String) -> String {
        let spaced = factName.reduce(into: "") { result, character in
            if character.isUppercase, !result.isEmpty, result.last != " " {
                result.append(" ")
            }
            result.append(character)
        }
        return spaced.prefix(1).uppercased() + spaced.dropFirst()
    }
}
