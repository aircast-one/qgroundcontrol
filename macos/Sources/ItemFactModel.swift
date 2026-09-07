import Foundation

struct ItemFact: Identifiable, Equatable {
    let list: String
    let position: Int
    let name: String
    let value: String
    let units: String
    let options: [String]

    var id: String { "\(list).\(position)" }

    init?(json: Any?, list: String, position: Int) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.list = list
        self.position = position
        self.name = name
        value = (object["valueString"] as? String) ?? ""
        units = (object["units"] as? String) ?? ""
        options = (object["enumStrings"] as? [String]) ?? []
    }

    static func from(_ elements: [Any], list: String) -> [ItemFact] {
        elements.enumerated().compactMap { ItemFact(json: $0.element, list: list, position: $0.offset) }
    }

    static let lists = ["textFieldFacts", "comboboxFacts"]
}
