import Foundation


// The core decides whether a plan can be saved or sent, and says why, so both heads give the
// operator the same sentence. This only carries the answer across.
struct PlanReadiness: Equatable {
    let ready: Bool
    let reason: String

    // Mission.swift takes this when the core served no readiness object at all. The decoder one
    // line up refuses to call a missing object ready; this constant used to say ready: true and
    // undo that at the call site. Nothing drawn changes -- the banner keys on the reason being
    // non-empty -- but a later control gated on `ready` would have opened on an unread view.
    static let unknown = PlanReadiness(ready: false, reason: "")

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

    // WHICH STATE WARRANTS THE ALERT, kept here rather than at the call site in Mission.swift,
    // which swift-checks does not compile. It is canSend and not the heading: the core's upload
    // sentences are becoming null when there is nothing to say, and a gate reading "there is a
    // heading, so warn" would then stop presenting real refusals the moment a sentence went absent.
    // This head never showed the wrong title Android saw for the same reason -- the alert is not
    // presented at all when a plan can be sent, so the state the core's sentence was wrong about
    // was one this screen never reached.
    var warrantsWarning: Bool { !canSend }

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
