import Foundation

struct PlanSummary: Equatable {
    let distanceMetres: Double
    let seconds: Double
    let maxTelemetryMetres: Double
    let measure: Measure

    static let empty = PlanSummary(distanceMetres: 0, seconds: 0, maxTelemetryMetres: 0,
                                   measure: .metres)

    var hasFlight: Bool { distanceMetres > 0 || seconds > 0 }

    var distanceText: String { PlanSummary.distance(distanceMetres, measure) }
    var telemetryText: String { PlanSummary.distance(maxTelemetryMetres, measure) }
    var durationText: String { PlanSummary.duration(seconds) }

    static func distance(_ metres: Double, _ measure: Measure) -> String {
        guard metres.isFinite, metres > 0 else { return "—" }
        return String(format: "%.0f %@", measure.convert(metres), measure.suffix)
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
    // Ordered as VisualMissionItem.h declares them: ReadyForSave, NotReadyForSaveTerrain,
    // NotReadyForSaveData. These were swapped, so a plan waiting on terrain told the operator
    // an item was unfinished, and an unfinished item blamed the terrain.
    static let readyForSave = 0
    static let notReadyForSaveTerrain = 1
    static let notReadyForSaveData = 2

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

enum PlanUpload: Int {
    case ok = 0
    case noVehicle = 1
    case firmwareMismatch = 2
    case flyingThisMission = 3

    var refusal: String {
        switch self {
        case .ok: return ""
        case .noVehicle: return "No vehicle is connected, so there is nowhere to send this plan."
        case .firmwareMismatch:
            return "This plan was made for a different firmware or vehicle type. "
                + "Uploading it can make the vehicle behave incorrectly."
        case .flyingThisMission:
            return "The vehicle is flying this mission. It has to be paused before a new plan goes up."
        }
    }

    var heading: String {
        canProceed ? "Upload this plan?" : "This plan cannot be uploaded"
    }

    var proceedTitle: String {
        switch self {
        case .firmwareMismatch: return "Upload anyway"
        case .flyingThisMission: return "Pause and upload"
        case .ok, .noVehicle: return ""
        }
    }

    var canProceed: Bool {
        self == .firmwareMismatch || self == .flyingThisMission
    }

    var pausesFirst: Bool { self == .flyingThisMission }

    static func state(_ raw: Int) -> PlanUpload {
        PlanUpload(rawValue: raw) ?? .ok
    }

}

