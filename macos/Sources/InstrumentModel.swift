import Foundation

struct InstrumentSelection: Equatable, Identifiable {
    let group: String
    let factName: String

    var id: String { "\(group)/\(factName)" }

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

    static let batteryGroup = "batteries"

    // The instrument editor offered the literal "batteries.0", so on a two-pack vehicle the one
    // pack an operator most wants on the Fly panel - the suspect one - could not be put there at
    // all, and nothing on the editor hinted a second pack existed. Upstream offers one fact group
    // per pack: VehicleBatteryFactGroup::_findOrAddBatteryGroupById registers battery<id> and
    // InstrumentValueData::factGroupNames() feeds the Group combo directly.
    //
    // STATED LIMIT: upstream keys those groups on the battery's ID, this head addresses packs by
    // their LIST POSITION, because vehicle.batteries.<n> is the only path the bridge offers. The
    // two coincide whenever ids run from zero, and where they do not, "Battery 2" here means the
    // second pack in the list rather than the pack whose id is 1. Label and address agree with
    // each other, which is what a selection needs; they do not agree with QGC's numbering.
    static func batteryGroups(count: Int) -> [String] {
        (0..<max(0, count)).map { "\(batteryGroup).\($0)" }
    }

    static func batteryPosition(of group: String) -> Int? {
        let parts = group.split(separator: ".")
        guard parts.count == 2, parts[0] == batteryGroup else { return nil }
        return Int(parts[1])
    }

    static func title(for group: String, label: (String) -> String) -> String {
        guard !group.isEmpty else { return vehicleTitle }
        if let position = batteryPosition(of: group) { return "Battery \(position + 1)" }
        return label(group)
    }

    // The two fields have different jobs and this used to give both to the fact's own name. The
    // selection is handed back as view.instruments(<group>/<name>), which resolves it as a PATH,
    // so the key has to be the Q_PROPERTY; sixteen facts on a copter disagree with their property
    // -- every ESC rpm and current, and goodAttitudeEstimate against goodAttitudeEsimate -- and
    // each of those stored a key that silently resolved to nothing. The LABEL humanises the fact
    // name, because an operator numbers motors 1 to 4 rather than First to Fourth.
    static func facts(in json: [String: Any], label: (String) -> String) -> [InstrumentFact] {
        ((json["facts"] as? [[String: Any]]) ?? []).compactMap { fact in
            let named = (fact["name"] as? String) ?? ""
            let property = (fact["property"] as? String) ?? ""
            let key = property.isEmpty ? named : property
            guard !key.isEmpty else { return nil }
            let reads = named.isEmpty ? key : named
            let described = (fact["shortDescription"] as? String) ?? ""
            return InstrumentFact(name: key, label: described.isEmpty ? label(reads) : described)
        }
    }

    // view.instrumentGroups has already made both choices -- property as the key, shortDescription
    // else the humanised fact name as the label -- so this decodes rather than re-deriving. It
    // covers the vehicle's CHILDREN only: the group an operator sees as "Vehicle" and the per-pack
    // battery groups are named by this head and are assembled above.
    static func served(_ json: Any?) -> [InstrumentGroup] {
        ((json as? [Any]) ?? []).compactMap { entry in
            guard let entry = entry as? [String: Any],
                  let group = entry["group"] as? String, !group.isEmpty,
                  let title = entry["title"] as? String else { return nil }
            let listed = ((entry["facts"] as? [[String: Any]]) ?? []).compactMap { fact -> InstrumentFact? in
                guard let name = fact["name"] as? String, !name.isEmpty,
                      let label = fact["label"] as? String else { return nil }
                return InstrumentFact(name: name, label: label)
            }
            guard !listed.isEmpty else { return nil }
            return InstrumentGroup(group: group, title: title, facts: listed)
        }
    }

    static let vehicleAlias = "vehicle"

    static func assemble(_ read: [(group: String, json: [String: Any])],
                         label: (String) -> String) -> [InstrumentGroup] {
        read.compactMap { entry in
            guard entry.group != vehicleAlias else { return nil }
            let listed = facts(in: entry.json, label: label)
            guard !listed.isEmpty else { return nil }
            return InstrumentGroup(group: entry.group,
                                   title: title(for: entry.group, label: label), facts: listed)
        }
    }
}

struct InstrumentValue: Identifiable, Equatable {
    let id: String
    let group: String
    let name: String
    let label: String
    let value: String
    let units: String
    let missing: Bool
    let missingReason: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String, !id.isEmpty,
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.id = id
        self.name = name
        group = (json["group"] as? String) ?? ""
        label = (json["label"] as? String) ?? ""
        value = (json["value"] as? String) ?? ""
        units = (json["units"] as? String) ?? ""
        missing = (json["missing"] as? NSNumber)?.boolValue ?? false
        missingReason = (json["missingReason"] as? String) ?? ""
    }

    // A blank is the right drawing for a fact the aircraft has and has not reported yet -- it will
    // fill in. It is the WRONG drawing for a fact this aircraft does not have at all, which is
    // what a selection stored against a different airframe leaves behind: the row waits forever
    // for a value that is never coming and says nothing about why. Only noSuchFact earns a
    // sentence; an unrecognised reason keeps the blank, because a reason this head cannot read is
    // not evidence that the fact is absent.
    var absentHere: Bool { missing && missingReason == "noSuchFact" }

    static let absentText = "not on this aircraft"

    static func list(_ json: Any?) -> [InstrumentValue] {
        ((json as? [Any]) ?? []).compactMap(InstrumentValue.init)
    }
}
