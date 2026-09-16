import Foundation

struct MissionSummaryRow: Equatable, Identifiable {
    let id: String
    let label: String
    let value: String

    init(id: String = "", label: String, value: String) {
        self.id = id.isEmpty ? label : id
        self.label = label
        self.value = value
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let label = json["label"] as? String, !label.isEmpty,
              let value = json["value"] as? String, !value.isEmpty else { return nil }
        self.init(id: (json["id"] as? String) ?? "", label: label, value: value)
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

    // The plan's "nothing here" sentence is the CORE's: missionsummary.rs:268 serves "" once the
    // controller reports items and "This plan has no items yet." when it does not. The head used to
    // spell its own "This plan has no items." -- one word apart, in two uncompiled files, derived
    // from the head's OWN decoded item list rather than from the controller's containsItems. Two
    // sources for one question agree until they disagree, and then the screen picks the wrong one.
    //
    // noController is the one sentence that stays the head's, because it is a fact about this app
    // rather than about the plan: there is no mission controller on the bridge at all. It wins over
    // the served reason, since a plan you cannot reach is not a plan with nothing in it.
    //
    // It used to read "No vehicle is connected." -- a different fact, and a false instruction. The
    // plan editor needs no vehicle; planning on the ground before an aircraft is powered is the
    // normal way to use it, and every other sentence in this window says so. The comment above and
    // the assertion labels had the condition right the whole time and the string said something
    // else, which nothing could catch: both assertions compared planStatus's output to this same
    // constant, so they pinned the wiring and left the words free.
    //
    // Reachable without a third state: reload() reads view.missionSummary synchronously before it
    // sets this, so the reason is in hand rather than waiting on the watch.
    static func planStatus(controller: Bool, reason: String) -> String {
        controller ? reason : MissionSummary.noController
    }

    static let noController = "The plan editor is unavailable."

    static let distance = "distance"
    static let time = "time"
    static let furthest = "furthest"

    func value(_ identifier: String) -> String? {
        rows.first { $0.id == identifier }?.value
    }

    var timeText: String { value(MissionSummary.time) ?? MissionSummary.unknown }

    static let unknown = "\u{2014}"
    static let unknownTimeHelp = "How long the mission takes -- not known for this aircraft"

    var describes: Bool { available && !rows.isEmpty }

    // The strip already refuses to repeat a figure the three named rows are drawing. It did not
    // refuse to repeat one EXTRA with another: a copter plan measured here has Planned and Hover
    // both at 41.74 km, and both were drawn. Two labels over one number is the same figure said
    // twice, and on a strip that has to fit they cost the width that truncated them BOTH to
    // "P..." and "...". First one wins, because the core's order is the answer to which label
    // that figure belongs to.
    var extraRows: [MissionSummaryRow] {
        let shown = [MissionSummary.distance, MissionSummary.time, MissionSummary.furthest]
        var figures = Set(shown.compactMap(value))
        return rows.filter { row in
            guard !shown.contains(row.id), figures.insert(row.value).inserted else { return false }
            return true
        }
    }
}
