import Foundation

struct PlanSummary: Equatable {
    let distanceMetres: Double
    let seconds: Double
    let maxTelemetryMetres: Double

    static let empty = PlanSummary(distanceMetres: 0, seconds: 0, maxTelemetryMetres: 0)

    var hasFlight: Bool { distanceMetres > 0 || seconds > 0 }

    var distanceText: String { PlanSummary.distance(distanceMetres) }
    var telemetryText: String { PlanSummary.distance(maxTelemetryMetres) }
    var durationText: String { PlanSummary.duration(seconds) }

    static func distance(_ metres: Double) -> String {
        guard metres.isFinite, metres > 0 else { return "—" }
        return metres < 1000
            ? String(format: "%.0f m", metres)
            : String(format: "%.1f km", metres / 1000)
    }

    static func duration(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds > 0 else { return "—" }
        let whole = Int(seconds.rounded())
        let hours = whole / 3600
        let minutes = (whole % 3600) / 60
        let remainder = whole % 60
        return hours > 0
            ? String(format: "%d:%02d:%02d", hours, minutes, remainder)
            : String(format: "%d:%02d", minutes, remainder)
    }
}

enum PlanReadiness {
    static let readyForSave = 0
    static let notReadyForSaveData = 1
    static let notReadyForSaveTerrain = 2

    static func reason(for state: Int) -> String {
        switch state {
        case notReadyForSaveData:
            return "An item is still being drawn, so the plan cannot be saved or sent."
        case notReadyForSaveTerrain:
            return "Waiting for terrain heights before the plan can be saved or sent."
        default:
            return ""
        }
    }
}
