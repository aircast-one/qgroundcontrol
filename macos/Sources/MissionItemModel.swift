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
    let isCurrent: Bool
    let specifiesAltitude: Bool
    let commandId: Int
    let isSimpleItem: Bool
    let kind: String
    let movable: Bool
    let category: String
    let altitudeUnits: String

    let altitudeText: String
    let altitudeBandText: String

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

    var stopsSave: Bool { blocked || awaitingTerrain }

    var canRemove: Bool { sequence > 0 }

    var canMove: Bool { movable }

    var altitudeReading: String {
        if specifiesAltitude || kind == MissionItem.settingsKind { return altitudeText }
        return altitudeBandText.isEmpty ? MissionItem.noAltitude : altitudeBandText
    }

    var speedReading: String? { speedChangeText.map { "Flies at \($0)" } }

    var holdReading: String? {
        guard let extraSeconds, extraSeconds.isFinite, extraSeconds > 0 else { return nil }
        let whole = extraSeconds.rounded() == extraSeconds
        return "Holds for "
            + (whole ? String(format: "%.0f", extraSeconds) : String(extraSeconds)) + " s"
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
    var canChangeCommand: Bool { isSimpleItem && sequence > 0 && !isLaunch }

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

    static func routePoints(_ items: [MissionItem]) -> [MissionPoint] {
        route(items).flatMap(\.legPoints)
    }

    static func route(_ items: [MissionItem]) -> [MissionItem] {
        items.prefix(routeEnd(items)).filter { $0.flownLeg && $0.hasPosition }
    }

    static func unreached(_ items: [MissionItem]) -> Set<Int> {
        Set(items.dropFirst(routeEnd(items) + 1).map(\.index))
    }

    static let afterRoute = "Never flown to"

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
        let leaves = json["exitCoordinate"] as? [String: Any]
        exitLatitude = (leaves?["latitude"] as? NSNumber)?.doubleValue
        exitLongitude = (leaves?["longitude"] as? NSNumber)?.doubleValue

        altitude = (json["altitude"] as? NSNumber)?.doubleValue
        altitudeUnits = (json["altitudeUnits"] as? String) ?? Measure.metres.units
        altitudeText = (json["altitudeText"] as? String) ?? MissionItem.noAltitude
        altitudeBandText = (json["altitudeBandText"] as? String) ?? ""

        azimuthText = (json["azimuthText"] as? String) ?? MissionItem.noAltitude
        distanceText = (json["distanceText"] as? String) ?? MissionItem.noAltitude
        altitudeChangeText = (json["altitudeChangeText"] as? String) ?? MissionItem.noAltitude
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
