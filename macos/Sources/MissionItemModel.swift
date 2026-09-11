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
    let commandId: Int
    let isSimpleItem: Bool
    let kind: String
    let movable: Bool
    let category: String
    let altitudeUnits: String

    // The core's own sentence for the altitude. The head spelled this itself until the core's
    // altitude formatting went through the same format_measure as every other measure it serves.
    let altitudeText: String

    // The leg that reaches this item, spelled by the core in the unit that leg is measured in.
    // Distance is horizontal and the altitude change is vertical, and an operator can set those
    // units differently; formatting either here would be a second copy of that choice drawn
    // directly under a summary strip that used the core's.
    let azimuthText: String
    let distanceText: String
    let altitudeChangeText: String

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

    // The core's answer, not a rule of this head's. Writing a coordinate to an unplaced takeoff
    // leaves it unplaced and moves the launch point instead, through
    // TakeoffMissionItem::setCoordinate; the plan's own settings entry has a coordinate and is
    // not on the map at all. Both are facts about the item, and the head that derives them is the
    // head that eventually derives them differently from the other one.
    var canMove: Bool { movable }

    var isLaunch: Bool { kind == MissionItem.takeoffKind || kind == MissionItem.settingsKind }
    var isSurveyItem: Bool { kind == MissionItem.surveyKind }

    var canChangeCommand: Bool { isSimpleItem && sequence > 0 && !isLaunch }

    static let settingsKind = "settings"
    static let takeoffKind = "takeoff"
    static let surveyKind = "survey"

    // Where the route stops. Shared by the legs drawn on the map and the rows marked in the
    // list so the two cannot answer differently: the map already declined to draw a leg past a
    // return to launch while the list went on presenting those items as ordinary waypoints.
    static func routeEnd(_ items: [MissionItem]) -> Int {
        items.firstIndex(where: \.endsRoute) ?? items.count
    }

    // The legs the vehicle actually flies: the placed ones it visits, up to wherever the mission
    // ends. Not the same list as the markers -- a region of interest earns a pin and no leg.
    static func route(_ items: [MissionItem]) -> [MissionItem] {
        items.prefix(routeEnd(items)).filter { $0.flownLeg && $0.hasPosition }
    }

    // QGC lets an operator add items after a return to launch and uploads every one of them, so
    // this is not an error to block on -- it is a fact the row has to carry, because the vehicle
    // turns for home at the item above and never arrives at any of these.
    static func unreached(_ items: [MissionItem]) -> Set<Int> {
        Set(items.dropFirst(routeEnd(items) + 1).map(\.index))
    }

    // Short because the row truncates: "Uploaded but never flown to" rendered as "Uploaded but
    // never..." and lost the only two words that mattered. The caption answers the operator's
    // question -- will the aircraft go there -- and being uploaded anyway is implied by the item
    // still being in the list.
    static let afterRoute = "Never flown to"

    // Which items were reached by a leg worth spelling. The core serves 0 degrees over 0 m for
    // every item that is not, so this cannot be derived from the figures themselves: due north is
    // a real bearing of exactly 0, and reading the zero as "no leg" would hide a leg flown due
    // north. It is the route's own rule instead -- flown to, placed, before the mission ends --
    // with its first point dropped, because nothing flies a leg to where the route begins.
    static func legs(_ items: [MissionItem]) -> Set<Int> {
        Set(route(items).dropFirst().map(\.index))
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

    // The editor's selection is one index on the view, not a flag on each item: the core stopped
    // serving a per-item "current" because it invited a head to draw a marker meaning "the
    // aircraft is here", which is a different fact QGC only sets in the fly view. Read through
    // here so every caller keeps asking the item, and so the comparison is pinned by a test.
    init(view json: [String: Any], selected: Int) {
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        sequence = (json["sequence"] as? NSNumber)?.intValue ?? 0
        command = (json["name"] as? String) ?? ""
        description = (json["description"] as? String) ?? ""
        isCurrent = index == selected
        specifiesAltitude = (json["specifiesAltitude"] as? NSNumber)?.boolValue ?? false
        commandId = (json["command"] as? NSNumber)?.intValue ?? 0
        isSimpleItem = (json["simple"] as? NSNumber)?.boolValue ?? false
        category = (json["category"] as? String) ?? ""
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
        awaitingTerrain = (json["awaitingTerrain"] as? NSNumber)?.boolValue ?? false
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        flownLeg = (json["flownLeg"] as? NSNumber)?.boolValue ?? false
        endsRoute = (json["endsRoute"] as? NSNumber)?.boolValue ?? false

        kind = (json["kind"] as? String) ?? ""
        movable = (json["movable"] as? NSNumber)?.boolValue ?? false

        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        // The number and its unit travel apart because the row's altitude editor needs them
        // apart; the sentence travels with them because nothing else here should be spelling one.
        altitude = (json["altitude"] as? NSNumber)?.doubleValue
        altitudeUnits = (json["altitudeUnits"] as? String) ?? Measure.metres.units
        altitudeText = (json["altitudeText"] as? String) ?? MissionItem.noAltitude

        azimuthText = (json["azimuthText"] as? String) ?? ""
        distanceText = (json["distanceText"] as? String) ?? ""
        altitudeChangeText = (json["altitudeChangeText"] as? String) ?? ""
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
