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
        return measure.text(metres)
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

// The core decides whether a plan can be saved or sent, and says why, so both heads give the
// operator the same sentence. This only carries the answer across.
struct PlanReadiness: Equatable {
    let ready: Bool
    let reason: String

    static let unknown = PlanReadiness(ready: true, reason: "")

    init(ready: Bool, reason: String) {
        self.ready = ready
        self.reason = reason
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        ready = (json["ready"] as? NSNumber)?.boolValue ?? false
        reason = (json["reason"] as? String) ?? ""
    }
}

struct PlanUpload: Equatable {
    static let uncheckable =
        "The plan was not sent: this head could not read whether the vehicle will accept it."

    let canSend: Bool
    let refusal: String
    let heading: String
    let proceedTitle: String
    let canProceed: Bool
    let pausesFirst: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        canSend = (json["canSend"] as? NSNumber)?.boolValue ?? false
        refusal = (json["refusal"] as? String) ?? ""
        heading = (json["heading"] as? String) ?? ""
        proceedTitle = (json["proceedTitle"] as? String) ?? ""
        canProceed = (json["canProceed"] as? NSNumber)?.boolValue ?? false
        pausesFirst = (json["pausesFirst"] as? NSNumber)?.boolValue ?? false
    }
}

enum PlanDirtyBadge {
    static let unsent = "Unsent"
    static let unsaved = "Unsaved"

    static func text(connected: Bool) -> String { connected ? unsent : unsaved }
}
