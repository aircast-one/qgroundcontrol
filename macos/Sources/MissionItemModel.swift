import Foundation

struct MissionItem: Identifiable, Equatable {
    let index: Int
    let sequence: Int
    let command: String
    let description: String
    let latitude: Double?
    let longitude: Double?
    let altitude: Double?
    let isCurrent: Bool
    let specifiesAltitude: Bool
    let isLaunch: Bool
    let commandId: Int
    let isSimpleItem: Bool
    let isSurveyItem: Bool
    let category: String
    let altitudeUnits: String

    // The core's own sentence for the altitude. The head spelled this itself until the core's
    // altitude formatting went through the same format_measure as every other measure it serves.
    let altitudeText: String

    let blocked: Bool
    let blockedReason: String?
    let awaitingTerrain: Bool

    // Whether the vehicle flies a leg to this item. A region of interest has a position and draws
    // a marker, and the aircraft never goes there; the route used to detour through it because
    // one list answered both questions.
    let flownLeg: Bool

    // A return to launch or a landing is where the mission stops. Items can sit after one -- the
    // plan uploads them and the vehicle never reaches them -- so a route drawn through them is a
    // line across the map to somewhere the aircraft does not go.
    let endsRoute: Bool

    var id: Int { sequence }

    // The core decides this: an item with no place of its own carries no coordinate at all, so
    // the 0,0 that once put the Gulf of Guinea in the map's bounding box never arrives here.
    var hasPosition: Bool { latitude != nil && longitude != nil }

    // Both stop the plan being saved, so both earn the row's amber seal and its sentence. Only
    // one of them is the operator's to fix, which is what blocked means on its own -- the banner
    // that names an item and selects it uses that, because a terrain wait has nowhere to send them.
    var stopsSave: Bool { blocked || awaitingTerrain }

    var canRemove: Bool { sequence > 0 }

    var canChangeCommand: Bool { isSimpleItem && sequence > 0 && !isLaunch }

    static let settingsKind = "settings"
    static let takeoffKind = "takeoff"
    static let surveyKind = "survey"

    // The legs the vehicle actually flies: the placed ones it visits, up to wherever the mission
    // ends. Not the same list as the markers -- a region of interest earns a pin and no leg.
    static func route(_ items: [MissionItem]) -> [MissionItem] {
        let ends = items.firstIndex(where: \.endsRoute) ?? items.count
        return items.prefix(ends).filter { $0.flownLeg && $0.hasPosition }
    }

    static func blockedItem(_ items: [MissionItem]) -> MissionItem? {
        let blocked = items.filter(\.blocked)
        guard blocked.count == 1, let only = blocked.first,
              !only.awaitingTerrain else { return nil }
        return only
    }

    static func blockedBanner(_ items: [MissionItem], reason: String) -> String {
        guard let only = blockedItem(items), let why = only.blockedReason else { return reason }
        return "\(only.command) (\(only.sequence)): \(why.lowercasedFirst)"
    }

    init(view json: [String: Any]) {
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        sequence = (json["sequence"] as? NSNumber)?.intValue ?? 0
        command = (json["name"] as? String) ?? ""
        description = (json["description"] as? String) ?? ""
        isCurrent = (json["current"] as? NSNumber)?.boolValue ?? false
        specifiesAltitude = (json["specifiesAltitude"] as? NSNumber)?.boolValue ?? false
        commandId = (json["command"] as? NSNumber)?.intValue ?? 0
        isSimpleItem = (json["simple"] as? NSNumber)?.boolValue ?? false
        category = (json["category"] as? String) ?? ""
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
        awaitingTerrain = (json["awaitingTerrain"] as? NSNumber)?.boolValue ?? false
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        flownLeg = (json["flownLeg"] as? NSNumber)?.boolValue ?? false
        endsRoute = (json["endsRoute"] as? NSNumber)?.boolValue ?? false

        let kind = (json["kind"] as? String) ?? ""
        isLaunch = kind == MissionItem.takeoffKind || kind == MissionItem.settingsKind
        isSurveyItem = kind == MissionItem.surveyKind

        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        // The number and its unit travel apart because the row's altitude editor needs them
        // apart; the sentence travels with them because nothing else here should be spelling one.
        altitude = (json["altitude"] as? NSNumber)?.doubleValue
        altitudeUnits = (json["altitudeUnits"] as? String) ?? Measure.metres.units
        altitudeText = (json["altitudeText"] as? String) ?? MissionItem.noAltitude
    }

    var positionText: String {
        guard hasPosition, let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    static let noAltitude = "\u{2014}"
}

extension String {
    var lowercasedFirst: String {
        guard let first else { return self }
        return first.lowercased() + dropFirst()
    }
}
