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
    let supported: Bool?
    let enabled: Bool?
    let stale: Bool?
    let sectors: Int
    let nearest: ObstacleReading?

    static let none = ObstacleDistance()

    private init() {
        available = false
        supported = nil
        enabled = nil
        stale = nil
        sectors = 0
        nearest = nil
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        supported = (json["supported"] as? NSNumber)?.boolValue
        enabled = (json["enabled"] as? NSNumber)?.boolValue
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
        return avoidanceOff ? "Avoidance off" : "Nothing detected"
    }

    // `enabled` says whether the AUTOPILOT will steer around what the sensor sees. It does not
    // say whether the sensor sees anything, and it must never suppress a reading: with avoidance
    // off the operator needs the distance MORE, because nothing is going to turn for them.
    var level: FlyTelemetry.Level {
        guard available else { return .unknown }
        if stale == true { return .caution }
        guard let nearest else { return enabled == true ? .good : .unknown }
        return nearest.close ? .warning : .caution
    }

    // obstacle.rs:97 answers supported null until parametersReady and false on an airframe with
    // no CP_DIST at all, and answers enabled ONLY when supported -- so a head reading enabled
    // while the answer is unknown gets nothing rather than a lie. The row is drawn on supported
    // being true and on nothing else: before b21d161d4 it said "Avoidance Off" on aircraft that
    // have never had the feature, which invites an operator to go looking for a switch that is
    // not anywhere.
    var avoidanceText: String {
        guard supported == true else { return "" }
        return enabled == true ? "On" : "Off"
    }

    // And the same claim one layer up: "Avoidance off" was the summary for a live sensor finding
    // nothing whenever enabled was not true, which included every unsupported airframe.
    var avoidanceOff: Bool { supported == true && enabled != true }

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
                DetailRow(label: "Avoidance", value: avoidanceText)]
            .filter { !$0.value.isEmpty }
    }
}
