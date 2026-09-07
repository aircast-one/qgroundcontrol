import Foundation

struct ItemFact: Identifiable, Equatable {
    let pathSuffix: String
    let name: String
    let title: String
    let isBool: Bool
    let value: String
    let units: String
    let options: [String]
    let readOnly: Bool

    var id: String { pathSuffix }

    private init?(json: Any?, pathSuffix: String) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.pathSuffix = pathSuffix
        self.name = name

        // Survey settings are named as identifiers -- TurnAroundDistanceMultiRotor --
        // so the description QGC already writes for them is the label to show.
        let described = (object["shortDescription"] as? String) ?? ""
        title = described.isEmpty ? Fact.humanise(name) : described
        isBool = (object["typeIsBool"] as? NSNumber)?.boolValue ?? false
        value = (object["valueString"] as? String) ?? ""
        units = (object["units"] as? String) ?? ""
        options = (object["enumStrings"] as? [String]) ?? []
        readOnly = (object["readOnly"] as? NSNumber)?.boolValue ?? false
    }

    // A fact in one of the item's fact lists is addressed by its position; a fact that
    // is a property of the item is addressed by that property's name.
    static func from(_ elements: [Any], list: String) -> [ItemFact] {
        elements.enumerated().compactMap {
            ItemFact(json: $0.element, pathSuffix: "\(list).\($0.offset)")
        }
    }

    static func owned(_ elements: [Any]) -> [ItemFact] {
        elements.compactMap { element in
            guard let object = element as? [String: Any],
                  let property = object["property"] as? String, !property.isEmpty else { return nil }
            return ItemFact(json: element, pathSuffix: property)
        }
    }

    static let lists = ["textFieldFacts", "comboboxFacts"]
}
