import Foundation

struct MissionSummaryRow: Equatable, Identifiable {
    let label: String
    let value: String

    var id: String { label }

    init(label: String, value: String) {
        self.label = label
        self.value = value
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let label = json["label"] as? String, !label.isEmpty,
              let value = json["value"] as? String, !value.isEmpty else { return nil }
        self.label = label
        self.value = value
    }
}

struct MissionSummary: Equatable {
    let available: Bool
    let rows: [MissionSummaryRow]
    let reason: String
    let altitudeRange: String

    static let empty = MissionSummary(available: false, rows: [], reason: "", altitudeRange: "")

    init(available: Bool, rows: [MissionSummaryRow], reason: String, altitudeRange: String) {
        self.available = available
        self.rows = rows
        self.reason = reason
        self.altitudeRange = altitudeRange
    }

    init(_ json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        rows = ((json["rows"] as? [Any]) ?? []).compactMap(MissionSummaryRow.init)
        reason = (json["reason"] as? String) ?? ""
        altitudeRange = ((json["altitudeRange"] as? [String: Any])?["text"] as? String) ?? ""
    }

    // The labels the header strip pulls out by name. The core emits every row it can compute and
    // leaves out the ones it cannot, so a row that is absent here is a fact the controller does
    // not have rather than a zero -- needing no batteries and having no battery model are
    // different things, and neither should read as "0".
    static let distance = "Distance"
    static let time = "Time"
    static let furthest = "Furthest from launch"

    func value(_ label: String) -> String? {
        rows.first { $0.label == label }?.value
    }

    // Absent when the controller could not work it out: a VTOL's speed changes as the walk passes
    // a transition item, so a single-speed answer is right before it and wrong after, and the core
    // declines rather than guessing. Drawn as unknown instead of omitted, because a chip that
    // simply vanishes leaves the operator unable to tell "not known" from "I did not look" -- the
    // same reading every other fallback in this head already takes.
    var timeText: String { value(MissionSummary.time) ?? MissionSummary.unknown }

    static let unknown = "\u{2014}"
    static let unknownTimeHelp = "How long the mission takes -- not known for this aircraft"

    var describes: Bool { available && !rows.isEmpty }

    // Everything the strip does not already show. Dedup was by label, which cannot see that a
    // multirotor's Hover distance IS its total distance and its Planned distance is the same
    // figure again -- the strip printed 14.11 km four times and wrapped every cell doing it.
    // A row whose figure differs stays, whatever it is. The core no longer sends a regime the
    // airframe does not have, so a multirotor arrives with nothing left over here at all.
    var extraRows: [MissionSummaryRow] {
        let shown = [MissionSummary.distance, MissionSummary.time, MissionSummary.furthest]
        let figures = Set(shown.compactMap(value))
        return rows.filter { !shown.contains($0.label) && !figures.contains($0.value) }
    }
}
