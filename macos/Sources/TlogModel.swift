import Foundation

struct TlogSummary: Equatable {
    let path: String
    let readable: Bool
    let bytes: Int
    let frames: Int
    let undecodable: Int
    let spanSeconds: Double
    let systemIds: [Int]
    let byName: [String: Int]

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object",
              let path = json["path"] as? String else { return nil }
        self.path = path
        readable = (json["readable"] as? NSNumber)?.boolValue ?? false
        bytes = (json["bytes"] as? NSNumber)?.intValue ?? 0
        frames = (json["frames"] as? NSNumber)?.intValue ?? 0
        undecodable = (json["undecodable"] as? NSNumber)?.intValue ?? 0
        spanSeconds = (json["spanSeconds"] as? NSNumber)?.doubleValue ?? 0
        systemIds = ((json["systemIds"] as? [Any]) ?? []).compactMap {
            ($0 as? NSNumber)?.intValue
        }
        byName = ((json["byName"] as? [String: Any]) ?? [:]).compactMapValues {
            ($0 as? NSNumber)?.intValue
        }
    }

    // A log the core could not open carries no counts at all -- it does not report zero frames,
    // it reports that it never got to look. Zero frames is a DIFFERENT answer: a file that
    // opened and holds nothing. Only the second one says anything about the flight.
    var empty: Bool { readable && frames == 0 }

    // Every frame failing to decode is not a quiet partial success: a log the parser cannot
    // read at all still reports readable:true, because the FILE opened.
    var whollyUndecodable: Bool { readable && frames == 0 && undecodable > 0 }

    var messageKinds: Int { byName.count }

    var busiest: (name: String, count: Int)? {
        byName.max { left, right in
            left.value == right.value ? left.key > right.key : left.value < right.value
        }
        .map { (name: $0.key, count: $0.value) }
    }

    // Seconds are seconds in every locale, so the head spells this one and the core does not
    // need to. Distances and altitudes are the ones that convert.
    var spanText: String {
        guard readable, spanSeconds > 0 else { return "" }
        let whole = Int(spanSeconds.rounded())
        let minutes = whole / 60
        let seconds = whole % 60
        return minutes > 0 ? "\(minutes)m \(seconds)s" : "\(seconds)s"
    }

    var sizeText: String {
        guard readable else { return "" }
        let units = ["bytes", "KB", "MB", "GB"]
        let step = bytes <= 0 ? 0 : min(units.count - 1, Int(log2(Double(bytes)) / 10))
        let size = Double(bytes) / pow(1024, Double(step))
        let printed = step == 0
            ? String(bytes)
            : Measure.settled(String(format: "%.1f", size))
        return "\(printed) \(units[step])"
    }
}

enum TelemetryLog {
    // The only content page in either window that stated no purpose, and the one that most needed
    // to: in QGC, Analyze's "Telemetry Log" is the REPLAY -- it builds a LogReplayLink and flies
    // the recording through the app. This page opens the file, counts what is in it and closes it.
    // An operator arriving from QGC picks a log here expecting a replay, gets a table of frame
    // counts, and has no way to tell whether the replay failed or was never on offer.
    static let note = "Opens a flight recording and counts what is in it. "
        + "This reads the file; it does not replay it."
}
