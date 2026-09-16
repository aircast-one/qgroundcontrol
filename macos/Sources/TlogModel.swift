import Foundation

struct TlogSummary: Equatable {
    let path: String
    let readable: Bool
    let bytes: Int
    let frames: Int
    let undecodable: Int
    let spanSeconds: Double
    // Optional, not defaulted to empty: ABSENT means a producer that does not serve this yet, and
    // EMPTY is a measured answer -- the recording names no aircraft. Collapsing them would let a
    // stale core make this row assert "None" about a log full of vehicles, which is the defaulting
    // decoder turning absent into a verdict.
    let vehicleSystemIds: [Int]?
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
        vehicleSystemIds = (json["vehicleSystemIds"] as? [Any])?.compactMap {
            ($0 as? NSNumber)?.intValue
        }
        byName = ((json["byName"] as? [String: Any]) ?? [:]).compactMapValues {
            ($0 as? NSNumber)?.intValue
        }
    }

    // A log the core could not open carries no counts at all -- it does not report zero frames,
    // it reports that it never got to look. Zero frames is a DIFFERENT answer: a file that
    // opened and holds nothing. Only the second one says anything about the flight.
    var empty: Bool { readable && frames == 0 && bytes == 0 }

    // Every frame failing to decode is not a quiet partial success: a log the parser cannot
    // read at all still reports readable:true, because the FILE opened.
    //
    // Gated on BYTES rather than on undecodable, because undecodable cannot answer this.
    // tlog.rs:66 skips a byte and CONTINUES when no frame header is found there, counting
    // nothing; only a byte where a header WAS found and the body then failed is counted
    // (:78). Measured on three files built for this: 200 KB of random bytes gives
    // undecodable 1597, but 175 KB of plain text and 200 KB of zeros both give 0 -- and
    // took the "holds no frames" arm, which is what an operator should see for a real
    // recording that captured nothing. A file that is not a recording at all was calling
    // itself an empty flight.
    var whollyUndecodable: Bool { readable && frames == 0 && bytes > 0 }

    var messageKinds: Int { byName.count }

    // A recording carries no vehicle NAMES, only the system ids that sent the frames -- so this
    // is the only thing in the file that answers "whose flight is this". The panel could say how
    // long the log was and how many frames it held and never which aircraft flew it.
    //
    // THIS JOINED systemIds, WHICH IS NOT A LIST OF VEHICLES. gcsMavlinkSystemID defaults to 255
    // and sendGCSHeartbeat defaults true, so QGC's own heartbeat lands in every log it records:
    // measured, the row was wrong on 24 of 24 logs here -- "1, 255" on 17, and on 7 it named 255
    // for a recording holding no aircraft at all, only the station heartbeating at nothing.
    // vehicleSystemIds applies the same test hub.rs:1238 applies to a live heartbeat, so a log
    // names the same aircraft replayed as it did in flight.
    var vehiclesText: String {
        guard let vehicleSystemIds else { return "" }
        return vehicleSystemIds.sorted().map(String.init).joined(separator: ", ")
    }

    // The row used to be drawn only when it had ids, so the seven GCS-only logs simply lost it --
    // on a card of six labelled rows that reads as a head that forgot, not as an answer. NONE is
    // an answer, and it is the one an operator opening an unfamiliar recording most needs: this
    // file is not a flight.
    //
    // Gated on the field being SERVED rather than on the text being non-empty, so a producer that
    // has not got this yet keeps the row hidden instead of asserting None about every log.
    var namesVehicles: Bool { vehicleSystemIds != nil }

    static let noVehicles = "None"

    var busiest: (name: String, count: Int)? {
        byName.max { left, right in
            left.value == right.value ? left.key > right.key : left.value < right.value
        }
        .map { (name: $0.key, count: $0.value) }
    }

    // With ONE kind this count IS the Frames row two lines above it. parse() increments frames
    // and the by_name entry in the same visit (tlog.rs:88-89), so the per-name counts sum to
    // frames and a single name carries all of them -- that is a property of the producer, not a
    // coincidence of one file. Measured on the all-HEARTBEAT log in the Telemetry folder:
    // Frames 78, Message kinds 1, Busiest "HEARTBEAT 78". And a superlative over a set of one
    // claims a comparison that never happened. The name is the only thing the row adds there,
    // so that is all it says, under a label that is true.
    var busiestRow: (label: String, value: String)? {
        guard let busiest = busiest else { return nil }
        guard messageKinds > 1 else { return (label: "Only kind", value: busiest.name) }
        return (label: "Busiest", value: "\(busiest.name)  \(busiest.count)")
    }

    // Seconds are seconds in every locale, so the head spells this one and the core does not
    // need to. Distances and altitudes are the ones that convert.
    // span is 0.0 for THREE states the core cannot tell apart (tlog.rs:104): no timestamps at
    // all, a first and last that are equal, and a last EARLIER than the first -- a clock that
    // stepped backwards mid-recording. Only the first has no duration; the others have one the
    // file cannot express. frames separates that case from the rest, and the table is not drawn
    // at all when frames is zero, so the empty string here is never on screen.
    //
    // It draws the panel's unreported mark rather than "0s", because 0s is a claim and two of
    // the three states would make it false.
    var spanText: String {
        guard readable else { return "" }
        guard spanSeconds > 0 else { return frames > 0 ? Measure.unreported : "" }
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
