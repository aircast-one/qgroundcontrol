import Foundation

struct FactBound: Equatable {
    let limit: Double
    let text: String

    init?(_ limit: Any?, text: Any?, filledIn: Any?) {
        guard (filledIn as? NSNumber)?.boolValue == false,
              let value = (limit as? NSNumber)?.doubleValue, value.isFinite else { return nil }
        self.limit = value
        self.text = (text as? String) ?? ""
    }
}

struct ItemFact: Identifiable, Equatable {
    let pathSuffix: String
    let name: String
    let title: String
    let isBool: Bool
    let value: String
    let units: String
    let options: [String]
    let readOnly: Bool
    let lowest: FactBound?
    let highest: FactBound?
    var group = ItemFact.itemGroup

    var id: String { pathSuffix }

    static let itemGroup = "Settings"
    static let cameraGroup = "Camera"

    // A survey is decided by how high it flies, how finely it sees the ground and how much
    // the images overlap. The rest of cameraCalc describes the camera itself, which choosing
    // one from the catalogue supplies -- except for Custom Camera, where choosing IS
    // specifying and the operator has to type the optics in.
    static let surveyProperties = ["distanceToSurface", "imageDensity", "frontalOverlap", "sideOverlap"]
    static let opticsProperties = ["sensorWidth", "sensorHeight", "imageWidth", "imageHeight",
                                   "focalLength", "landscape", "minTriggerInterval"]

    static func cameraProperties(custom: Bool) -> [String] {
        custom ? opticsProperties + surveyProperties : surveyProperties
    }

    private init?(json: Any?, pathSuffix: String, label: (String) -> String) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String, !name.isEmpty else { return nil }
        self.pathSuffix = pathSuffix
        self.name = name

        // Survey settings are named as identifiers -- TurnAroundDistanceMultiRotor --
        // so the description QGC already writes for them is the label to show.
        let described = (object["shortDescription"] as? String) ?? ""
        title = described.isEmpty ? label(name) : described
        isBool = (object["typeIsBool"] as? NSNumber)?.boolValue ?? false
        value = (object["valueString"] as? String) ?? ""
        units = (object["units"] as? String) ?? ""
        options = (object["enumStrings"] as? [String]) ?? []
        readOnly = (object["readOnly"] as? NSNumber)?.boolValue ?? false
        lowest = FactBound(object["min"], text: object["minString"],
                           filledIn: object["minIsDefaultForType"])
        highest = FactBound(object["max"], text: object["maxString"],
                            filledIn: object["maxIsDefaultForType"])
    }

    func refusal(_ entry: String) -> String? {
        guard let typed = Double(entry) else { return nil }
        let under = lowest.map { typed < $0.limit } ?? false
        let over = highest.map { typed > $0.limit } ?? false
        guard under || over else { return nil }
        switch (lowest, highest) {
        case let (low?, high?): return "\(title) must be within \(low.text) and \(high.text)."
        case let (low?, nil): return "\(title) must be at least \(low.text)."
        case let (nil, high?): return "\(title) must be at most \(high.text)."
        default: return nil
        }
    }

    // A fact in one of the item's fact lists is addressed by its position; a fact that
    // is a property of the item is addressed by that property's name.
    static func from(_ elements: [Any], list: String,
                     label: (String) -> String) -> [ItemFact] {
        elements.enumerated().compactMap {
            ItemFact(json: $0.element, pathSuffix: "\(list).\($0.offset)", label: label)
        }
    }

    static let launchAltitudeProperty = "plannedHomePositionAltitude"

    static func owned(_ elements: [Any], label: (String) -> String) -> [ItemFact] {
        elements.compactMap { element in
            guard let object = element as? [String: Any],
                  let property = object["property"] as? String, !property.isEmpty,
                  property != launchAltitudeProperty else { return nil }
            return ItemFact(json: element, pathSuffix: property, label: label)
        }
    }

    static func camera(_ elements: [Any], custom: Bool,
                       label: (String) -> String) -> [ItemFact] {
        cameraProperties(custom: custom).compactMap { wanted in
            elements.compactMap { element -> ItemFact? in
                guard let object = element as? [String: Any],
                      (object["property"] as? String) == wanted else { return nil }
                var fact = ItemFact(json: element, pathSuffix: "cameraCalc.\(wanted)",
                                    label: label)
                fact?.group = cameraGroup
                return fact
            }.first
        }
    }

    static let lists = ["textFieldFacts", "comboboxFacts"]
}
