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

    var describes: Bool { available && !rows.isEmpty }

    // Everything the strip does not already show, so the same fact is not printed twice.
    var extraRows: [MissionSummaryRow] {
        let shown = [MissionSummary.distance, MissionSummary.time, MissionSummary.furthest]
        return rows.filter { !shown.contains($0.label) }
    }
}
