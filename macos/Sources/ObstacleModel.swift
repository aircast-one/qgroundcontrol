import Foundation

struct ObstacleReading: Equatable {
    let distanceText: String
    let bearing: Double
    let sector: String
    let sectorText: String
    let close: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        distanceText = (json["distanceText"] as? String) ?? ""
        bearing = (json["bearing"] as? NSNumber)?.doubleValue ?? 0
        sector = (json["sector"] as? String) ?? ""
        sectorText = (json["sectorText"] as? String) ?? ""
        close = (json["close"] as? NSNumber)?.boolValue ?? false
    }
}

struct ObstacleDistance: Equatable {
    let available: Bool
    let enabled: Bool
    let stale: Bool?
    let sectors: Int
    let nearest: ObstacleReading?

    static let none = ObstacleDistance()

    private init() {
        available = false
        enabled = false
        stale = nil
        sectors = 0
        nearest = nil
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
        stale = (json["stale"] as? NSNumber)?.boolValue
        sectors = (json["sectors"] as? NSNumber)?.intValue ?? 0
        nearest = ObstacleReading(json["nearest"])
    }

    // The core sends stale as a nullable: null means it has no msSinceUpdate to judge by, which
    // is NOT the same as judging the reading fresh. A sensor that has never reported and one
    // reporting on time would otherwise look identical.
    var freshnessKnown: Bool { stale != nil }

    // Four states, and a blank panel would collapse them. No sensor at all; a sensor present but
    // switched off; a sensor reporting and finding nothing, which is the only good news; and a
    // sensor whose readings have stopped arriving, which reads as clear air and is not.
    var summary: String {
        guard available else { return "No sensor" }
        if stale == true { return "Readings stopped" }
        if let nearest {
            return nearest.distanceText.isEmpty ? nearest.sectorText : nearest.distanceText
        }
        return enabled ? "Nothing detected" : "Avoidance off"
    }

    // `enabled` says whether the AUTOPILOT will steer around what the sensor sees. It does not
    // say whether the sensor sees anything, and it must never suppress a reading: with avoidance
    // off the operator needs the distance MORE, because nothing is going to turn for them.
    var level: FlyTelemetry.Level {
        guard available else { return .unknown }
        if stale == true { return .caution }
        guard let nearest else { return enabled ? .good : .unknown }
        return nearest.close ? .warning : .caution
    }

    func rows() -> [DetailRow] {
        guard available else {
            return [DetailRow(label: "Obstacle", value: "No sensor reporting")]
        }
        guard let nearest else {
            return [DetailRow(label: "Obstacle", value: summary)]
        }
        return [DetailRow(label: "Nearest", value: nearest.distanceText),
                DetailRow(label: "Direction", value: nearest.sectorText),
                DetailRow(label: "Sectors reporting", value: String(sectors)),
                DetailRow(label: "Avoidance", value: enabled ? "" : "Off")]
            .filter { !$0.value.isEmpty }
    }
}
