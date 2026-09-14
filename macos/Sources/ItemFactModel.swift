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

struct FactRange: Equatable {
    let title: String
    let lowest: FactBound?
    let highest: FactBound?

    init(_ json: [String: Any], title: String) {
        self.title = title
        lowest = FactBound(json["min"], text: json["minString"],
                           filledIn: json["minIsDefaultForType"])
        highest = FactBound(json["max"], text: json["maxString"],
                            filledIn: json["maxIsDefaultForType"])
    }

    func refusal(_ entry: String) -> String? {
        guard let typed = Double(entry) else { return nil }
        return refusal(typed)
    }

    func refusal(_ typed: Double) -> String? {
        let under = lowest.map { typed < $0.limit } ?? false
        let over = highest.map { typed > $0.limit } ?? false
        guard under || over else { return nil }
        return FactRange.sentence(title, lowest: lowest?.text, highest: highest?.text)
    }

    static func sentence(_ what: String, lowest: String?, highest: String?) -> String? {
        switch (lowest, highest) {
        case let (low?, high?): return "\(what) must be within \(low) and \(high)."
        case let (low?, nil): return "\(what) must be at least \(low)."
        case let (nil, high?): return "\(what) must be at most \(high)."
        default: return nil
        }
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
    let range: FactRange
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
        // enumOrValueString, not valueString, for the same reason FlyDetailModel prefers it: on an
        // ENUM fact valueString is the raw number and this is the label. The item editor binds a
        // Picker's selection to this value and tags each row with its enumStrings entry, so with
        // the number the selection matched NO tag and a camera action rendered with nothing chosen.
        // The core's own fixture spells it out -- cameraAction carries valueString "6" beside
        // enumOrValueString "Take photo" -- and enumStrings[6] is "Stop recording video", which is
        // what a head indexing by that number would have drawn instead. On a fact with no enum the
        // two are identical (gimbalPitch is "-90" in both), so this is right for every fact.
        value = (object["enumOrValueString"] as? String)
            ?? (object["valueString"] as? String) ?? ""
        units = (object["units"] as? String) ?? ""
        options = (object["enumStrings"] as? [String]) ?? []
        readOnly = (object["readOnly"] as? NSNumber)?.boolValue ?? false
        range = FactRange(object, title: title)
    }

    func refusal(_ entry: String) -> String? { range.refusal(entry) }

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
