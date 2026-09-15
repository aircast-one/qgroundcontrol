import Foundation

struct MissionPoint: Equatable {
    let latitude: Double
    let longitude: Double
}

struct MissionItem: Identifiable, Equatable {
    let index: Int
    let sequence: Int
    let command: String
    let description: String
    let latitude: Double?
    let longitude: Double?
    let exitLatitude: Double?
    let exitLongitude: Double?
    let altitude: Double?
    let altitudeMetres: Double?
    let positionAltitudeMetres: Double?
    let isSelected: Bool
    let specifiesAltitude: Bool
    let commandId: Int
    let isSimpleItem: Bool
    let kind: String
    let movable: Bool
    let category: String
    let altitudeUnits: String
    let altitudeEditUnits: String

    let altitudeText: String
    let altitudeBandText: String
    let altitudeFrame: String
    let frameWord: String?

    let azimuthText: String
    let distanceText: String
    let altitudeChangeText: String

    let blocked: Bool
    let blockedReason: String?
    let speedChangeText: String?
    let extraSeconds: Double?
    let foldedCommands: Int?
    let awaitingTerrain: Bool

    let flownLeg: Bool

    let endsRoute: Bool

    var id: Int { sequence }

    var hasPosition: Bool { latitude != nil && longitude != nil }

    var unready: Bool { blocked || awaitingTerrain }

    var canRemove: Bool { index > 0 }

    // The launch position is written as a QGeoCoordinate, whose altitude is metres by definition.
    // This used to read plan.missionController.visualItems.0.plannedHomePositionAltitude.rawValue
    // directly, which is the core's FALLBACK source and not its answer: for the home item the core
    // prefers the `altitude` fact and only falls back to plannedHomePositionAltitude when that one
    // reports nothing. Two ways to reach the same number is how they come to disagree, and the one
    // the head had chosen was the one that loses. It lives here rather than at the call site
    // because the call site is a store file swift-checks does not compile, so the choice of WHICH
    // field feeds a coordinate would have been unpinnable there.
    static func launchAltitudeMetres(_ items: [MissionItem]) -> Double? {
        items.first { $0.index == 0 }?.altitudeMetres
    }

    // A map drag produces a latitude and a longitude and nothing else. QGC's own drag handler
    // spends one statement on exactly this -- MissionItemIndicatorDrag.qml:57 assigns
    // coordinate.altitude = itemCoordinate.altitude before it writes the coordinate back -- and
    // without it the height is simply gone. The launch item is where it showed: it is movable --
    // the core serves movable true for it and says so in its own test -- and MissionSettingsItem
    // stores the whole coordinate, which the plan file then saves with its altitude at
    // MissionController.cc:1137. SimpleMissionItem::setCoordinate discards the altitude and would
    // not have cared.
    // An unknown altitude does NOT refuse the move. A null there means the served coordinate is
    // two-dimensional, so there is no height to lose, and every item but the launch one discards
    // the altitude anyway -- refusing would block ordinary drags to protect nothing. Omitting the
    // key is also SAFE rather than merely correct since 3dcb49394: the bridge used to read an
    // absent altitude as toDouble() on an empty QVariant, which is 0.0, and it now builds the
    // two-dimensional QGeoCoordinate instead. An explicit null failed the same way and is fixed
    // with it, which was the core's finding and not mine.
    static func dragPayload(_ item: MissionItem,
                            latitude: Double, longitude: Double) -> [String: Any] {
        let place: [String: Any] = ["latitude": latitude, "longitude": longitude]
        guard let altitude = item.positionAltitudeMetres else { return place }
        return place.merging(["altitude": altitude]) { _, new in new }
    }

    var canMove: Bool { movable }

    var frameSuffix: String {
        if let spelled = frameWord, !spelled.isEmpty { return " " + spelled }
        guard !altitudeFrame.isEmpty, altitudeFrame != MissionItem.launchFrame else { return "" }
        return " " + altitudeFrame.uppercased()
    }

    var altitudeFieldUnits: String { altitudeEditUnits + frameSuffix }

    var altitudeDecimals: Int { Measure.decimals(matching: altitudeText) ?? 0 }

    var mapSubtitle: String? {
        altitudeReading == MissionItem.noAltitude ? nil : altitudeReading
    }

    var altitudeReading: String {
        let measure = specifiesAltitude || kind == MissionItem.settingsKind
            ? altitudeText
            : (altitudeBandText.isEmpty ? MissionItem.noAltitude : altitudeBandText)
        return measure == MissionItem.noAltitude ? measure : measure + frameSuffix
    }

    static let launchFrame = "launch"

    var speedReading: String? { speedChangeText.map { "Flies at \($0)" } }

    var holdReading: String? {
        guard let extraSeconds, extraSeconds.isFinite, extraSeconds > 0 else { return nil }
        let whole = extraSeconds.rounded() == extraSeconds
        return "Holds for "
            + String(format: whole ? "%.0f" : "%.1f", extraSeconds) + " s"
    }

    var foldedReading: String? {
        guard let foldedCommands, foldedCommands > 0 else { return nil }
        return "\(foldedCommands) more command" + (foldedCommands == 1 ? "" : "s")
    }

    var doings: String? {
        let all = [speedReading, holdReading, foldedReading].compactMap { $0 }
        return all.isEmpty ? nil : all.joined(separator: MissionItem.between)
    }

    func subtitle(unreached: Bool) -> String {
        blockedReason ?? (unreached ? MissionItem.afterRoute : nil) ?? doings ?? ""
    }

    static let between = " \u{00B7} "

    var isLaunch: Bool { kind == MissionItem.takeoffKind || kind == MissionItem.settingsKind }
    var canChangeCommand: Bool { isSimpleItem && !isLaunch }

    static let settingsKind = "settings"
    static let takeoffKind = "takeoff"

    static func routeEnd(_ items: [MissionItem]) -> Int {
        items.firstIndex(where: \.endsRoute) ?? items.count
    }

    var legPoints: [MissionPoint] {
        guard let latitude, let longitude else { return [] }
        let entry = MissionPoint(latitude: latitude, longitude: longitude)
        guard let exitLatitude, let exitLongitude else { return [entry] }
        return [entry, MissionPoint(latitude: exitLatitude, longitude: exitLongitude)]
    }

    static func routePoints(_ items: [MissionItem], linkedToHome: Bool) -> [MissionPoint] {
        route(items, linkedToHome: linkedToHome).flatMap(\.legPoints)
    }

    // The settings row carries the planned home position and a coordinate, so it passed this
    // filter and the line was drawn from home to the first item on EVERY plan. QGC draws that
    // segment only when the mission starts from the ground -- a takeoff before any coordinate
    // item, or a rover -- and suppresses it otherwise with
    // `lastFlyThroughVI != _settingsItem || (homePositionValid && linkStartToHome)`
    // (MissionController.cc:1392). A mission that begins at a waypoint is flown TO, not launched
    // from home, and the leg claimed a flight nobody planned. hasPosition above is already the
    // homePositionValid half, so only the flag was missing.
    static func route(_ items: [MissionItem], linkedToHome: Bool) -> [MissionItem] {
        let flown = items.prefix(routeEnd(items)).filter { $0.flownLeg && $0.hasPosition }
        guard !linkedToHome else { return Array(flown) }
        return Array(flown.drop { $0.kind == MissionItem.settingsKind })
    }

    static func unreached(_ items: [MissionItem]) -> Set<Int> {
        Set(items.dropFirst(routeEnd(items) + 1).map(\.index))
    }

    static let afterRoute = "Never flown to"

    static func legs(_ items: [MissionItem], linkedToHome: Bool) -> Set<Int> {
        Set(route(items, linkedToHome: linkedToHome).dropFirst().map(\.index))
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

    init(view json: [String: Any], selected: Int) {
        index = (json["index"] as? NSNumber)?.intValue ?? 0
        sequence = (json["sequence"] as? NSNumber)?.intValue ?? 0
        command = (json["name"] as? String) ?? ""
        description = (json["description"] as? String) ?? ""
        isSelected = index == selected
        specifiesAltitude = (json["specifiesAltitude"] as? NSNumber)?.boolValue ?? false
        commandId = (json["command"] as? NSNumber)?.intValue ?? 0
        isSimpleItem = (json["simple"] as? NSNumber)?.boolValue ?? false
        category = (json["category"] as? String) ?? ""
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
        awaitingTerrain = (json["awaitingTerrain"] as? NSNumber)?.boolValue ?? false
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        speedChangeText = (json["speedChangeText"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        extraSeconds = (json["extraSeconds"] as? NSNumber)?.doubleValue
        foldedCommands = (json["foldedCommands"] as? NSNumber)?.intValue
        flownLeg = (json["flownLeg"] as? NSNumber)?.boolValue ?? false
        endsRoute = (json["endsRoute"] as? NSNumber)?.boolValue ?? false

        kind = (json["kind"] as? String) ?? ""
        movable = (json["movable"] as? NSNumber)?.boolValue ?? false

        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue
        positionAltitudeMetres = (coordinate?["altitude"] as? NSNumber)?.doubleValue
        let leaves = json["exitCoordinate"] as? [String: Any]
        exitLatitude = (leaves?["latitude"] as? NSNumber)?.doubleValue
        exitLongitude = (leaves?["longitude"] as? NSNumber)?.doubleValue

        altitude = (json["altitude"] as? NSNumber)?.doubleValue
        altitudeMetres = (json["altitudeMetres"] as? NSNumber)?.doubleValue
        altitudeUnits = (json["altitudeUnits"] as? String) ?? Measure.defaultUnits
        altitudeEditUnits = (json["altitudeEditUnits"] as? String) ?? ""
        altitudeText = (json["altitudeText"] as? String) ?? MissionItem.noAltitude
        altitudeBandText = (json["altitudeBandText"] as? String) ?? ""
        altitudeFrame = (json["altitudeFrame"] as? String) ?? ""
        frameWord = json["altitudeFrameText"] as? String

        azimuthText = (json["azimuthText"] as? String) ?? MissionItem.noAltitude
        distanceText = (json["distanceText"] as? String) ?? MissionItem.noAltitude
        altitudeChangeText = (json["altitudeChangeText"] as? String) ?? MissionItem.noAltitude
    }

    var positionText: String {
        guard hasPosition, let latitude, let longitude else { return "—" }
        return GeoPoint.text(latitude, longitude)
    }

    static let noAltitude = "\u{2014}"
}

extension String {
    var lowercasedFirst: String {
        guard let first else { return self }
        return first.lowercased() + dropFirst()
    }
}
