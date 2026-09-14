import Foundation

struct FactBound: Equatable {
    let limit: Double
    let text: String

    // Already gated by the producer, so the only question left is whether a number arrived.
    init?(served limit: Any?, text: Any?) {
        guard let value = (limit as? NSNumber)?.doubleValue, value.isFinite else { return nil }
        self.limit = value
        self.text = (text as? String) ?? ""
    }

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

    // view.control has already made the judgement the raw initialiser makes here: it serves
    // minimum/maximum null where the fact declares no bound, and minimumText/maximumText null
    // through the SAME gate, so the pair can never disagree. The bridge fills minString whatever
    // minIsDefaultForType says -- a fact with no floor carries the smallest number its type can
    // hold -- so reading the raw strings off a view would have spelled "at least -3.4e38".
    // Nothing to decide here, which is the point: the decision moved to the producer.
    init(control: [String: Any], title: String) {
        self.title = title
        lowest = FactBound(served: control["minimum"], text: control["minimumText"])
        highest = FactBound(served: control["maximum"], text: control["maximumText"])
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

struct ItemFactOption: Identifiable, Equatable {
    let label: String
    let raw: String

    var id: String { raw }
}

struct ItemFact: Identifiable, Equatable {
    let pathSuffix: String
    let name: String
    let title: String
    let isBool: Bool
    let value: String
    let units: String
    let display: String
    let options: [ItemFactOption]
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
        // The RAW value, because this is what the picker selects by and what setFact writes, and
        // setFact refuses anything Measure.numberRefusal cannot read as a number. The LABEL lives
        // on the option beside it -- two fields, two jobs, the same split ParameterOption uses.
        value = (object["valueString"] as? String) ?? ""
        display = (object["enumOrValueString"] as? String) ?? value
        units = (object["units"] as? String) ?? ""
        // Paired rather than labels alone. A picker tagged with labels binds a selection the write
        // path cannot accept: setFact runs numberRefusal first, so choosing "Take photo" answered
        // "Take photo is not a number." and set nothing. Guarded on equal counts for the reason
        // ParameterModel is -- FactMetaData::_parseEnums refuses a mismatch outright, so this
        // cannot fire through the known path, and pairing a wrong label to a wrong value in an
        // editor that writes a mission item is worse than offering no list at all.
        let labels = (object["enumStrings"] as? [String]) ?? []
        let raws = (object["enumValues"] as? [Any]) ?? []
        options = labels.count == raws.count
            ? zip(labels, raws).map { ItemFactOption(label: $0.0, raw: Parameter.rawText($0.1)) }
            : []
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
