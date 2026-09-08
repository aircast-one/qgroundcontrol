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
    let distanceFromStart: Double
    let amslAltitude: Double?
    let terrainAltitude: Double?
    let terrainCollision: Bool
    let commandId: Int
    let isSimpleItem: Bool
    let specifiesCoordinate: Bool
    let isSurveyItem: Bool
    let category: String
    let altitudeUnits: String

    var id: Int { sequence }

    // A command with no position of its own still reports a coordinate, and it is
    // 0,0 -- which put the Gulf of Guinea in the bounding box the map framed to.
    var hasPosition: Bool {
        guard specifiesCoordinate, let latitude, let longitude else { return false }
        return latitude != 0 || longitude != 0
    }

    var canRemove: Bool { sequence > 0 }

    var canChangeCommand: Bool { isSimpleItem && sequence > 0 && !isLaunch }

    init(json: [String: Any], index: Int, verticalMeasure: Measure = .metres) {
        self.index = index
        sequence = (json["sequenceNumber"] as? NSNumber)?.intValue ?? 0
        command = (json["commandName"] as? String) ?? ""
        description = (json["commandDescription"] as? String) ?? ""
        isCurrent = (json["isCurrentItem"] as? NSNumber)?.boolValue ?? false
        specifiesAltitude = (json["specifiesAltitude"] as? NSNumber)?.boolValue ?? false
        isLaunch = (json["isTakeoffItem"] as? NSNumber)?.boolValue ?? false
            || (json["homePosition"] as? NSNumber)?.boolValue ?? false
        distanceFromStart = (json["distanceFromStart"] as? NSNumber)?.doubleValue ?? 0
        amslAltitude = (json["amslEntryAlt"] as? NSNumber)?.doubleValue
        terrainAltitude = (json["terrainAltitude"] as? NSNumber)?.doubleValue
        terrainCollision = (json["terrainCollision"] as? NSNumber)?.boolValue ?? false
        commandId = (json["command"] as? NSNumber)?.intValue ?? 0
        isSimpleItem = (json["isSimpleItem"] as? NSNumber)?.boolValue ?? false
        specifiesCoordinate = (json["specifiesCoordinate"] as? NSNumber)?.boolValue ?? true
        isSurveyItem = (json["isSurveyItem"] as? NSNumber)?.boolValue ?? false
        category = (json["category"] as? String) ?? ""

        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        let facts = (json["facts"] as? [[String: Any]]) ?? []
        let altitudeFact = MissionItem.altitudeFactNames
            .lazy
            .compactMap { name in facts.first { ($0["name"] as? String) == name } }
            .first { ($0["value"] as? NSNumber)?.doubleValue.isFinite ?? false }

        if let altitudeFact {
            altitudeUnits = (altitudeFact["units"] as? String) ?? "m"
            altitude = (altitudeFact["value"] as? NSNumber)?.doubleValue
        } else {
            altitudeUnits = verticalMeasure.units
            altitude = ((coordinate?["altitude"] as? NSNumber)?.doubleValue)
                .map(verticalMeasure.convert)
        }
    }

    static let altitudeFactNames = ["Altitude", "PlannedHomePositionAltitude"]

    var positionText: String {
        guard hasPosition, let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    var altitudeText: String {
        guard let altitude, altitude.isFinite else { return "—" }
        return String(format: "%.1f %@", altitude, Measure.pretty(altitudeUnits))
    }
}
