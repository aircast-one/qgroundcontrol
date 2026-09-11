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

    static let distance = "Distance"
    static let time = "Time"
    static let furthest = "Furthest from launch"

    func value(_ label: String) -> String? {
        rows.first { $0.label == label }?.value
    }

    var timeText: String { value(MissionSummary.time) ?? MissionSummary.unknown }

    static let unknown = "\u{2014}"
    static let unknownTimeHelp = "How long the mission takes -- not known for this aircraft"

    var describes: Bool { available && !rows.isEmpty }

    var extraRows: [MissionSummaryRow] {
        let shown = [MissionSummary.distance, MissionSummary.time, MissionSummary.furthest]
        let figures = Set(shown.compactMap(value))
        return rows.filter { !shown.contains($0.label) && !figures.contains($0.value) }
    }
}
