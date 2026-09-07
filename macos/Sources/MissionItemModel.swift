import Foundation

struct MissionItem: Identifiable {
    let sequence: Int
    let command: String
    let description: String
    let latitude: Double?
    let longitude: Double?
    let altitude: Double?
    let isCurrent: Bool

    var id: Int { sequence }

    var hasPosition: Bool { latitude != nil && longitude != nil }

    // Waypoint altitude lives in the item's Altitude fact, not in the coordinate:
    // the coordinate's altitude is NaN for most commands, which arrives as null.
    init(json: [String: Any]) {
        sequence = (json["sequenceNumber"] as? NSNumber)?.intValue ?? 0
        command = (json["commandName"] as? String) ?? ""
        description = (json["commandDescription"] as? String) ?? ""
        isCurrent = (json["isCurrentItem"] as? NSNumber)?.boolValue ?? false

        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        let facts = (json["facts"] as? [[String: Any]]) ?? []
        let altitudeFact = facts.first { ($0["name"] as? String) == "Altitude" }
        if let value = (altitudeFact?["value"] as? NSNumber)?.doubleValue, value.isFinite {
            altitude = value
        } else {
            altitude = (coordinate?["altitude"] as? NSNumber)?.doubleValue
        }
    }

    var positionText: String {
        guard let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    var altitudeText: String {
        guard let altitude, altitude.isFinite else { return "—" }
        return String(format: "%.1f m", altitude)
    }
}
