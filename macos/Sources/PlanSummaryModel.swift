import Foundation


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

    static let unknown = PlanUpload(canSend: false, refusal: PlanUpload.uncheckable,
                                    heading: "", proceedTitle: "", canProceed: false,
                                    pausesFirst: false)

    init(canSend: Bool, refusal: String, heading: String, proceedTitle: String,
         canProceed: Bool, pausesFirst: Bool) {
        self.canSend = canSend
        self.refusal = refusal
        self.heading = heading
        self.proceedTitle = proceedTitle
        self.canProceed = canProceed
        self.pausesFirst = pausesFirst
    }

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

enum PlanClear {
    static let local = "Clear Plan"
    static let vehicleWord = "Mission"
}
