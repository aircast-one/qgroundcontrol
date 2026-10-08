import Foundation

func summaryRow(_ view: JSON?, _ label: String) -> String? {
    guard let rows = view?["rows"].arrayOrNil else { return nil }
    return rows.first { $0.object != nil && $0["label"].string == label }
        .map { $0["value"].string }
        .flatMap { $0.isBlank ? nil : $0 }
}

func missionSummaryText(_ view: JSON?) -> String {
    [
        summaryRow(view, "Distance").map { "Distance \($0)" },
        summaryRow(view, "Time").map { "Time \($0)" },
        summaryRow(view, "Furthest from launch").map { "Max telem \($0)" },
    ].compactMap { $0 }.joined(separator: " · ")
}
