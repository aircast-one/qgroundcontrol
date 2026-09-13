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

    // view.tlog takes its path INSIDE the view name, and view::split splits that on commas at
    // paren depth zero and counts parens for nesting. So a file whose name contains one of those
    // arrives at the core as the wrong path -- truncated at the comma -- and comes back
    // readable:false, which is a WRONG answer rather than a failure: the file is fine and the
    // encoding lost it. Refusing up front says so instead.
    static func encodable(_ path: String) -> Bool {
        !path.isEmpty && !path.contains(",") && !path.contains("(") && !path.contains(")")
    }
}
