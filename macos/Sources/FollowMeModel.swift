import Foundation

struct FollowMeVehicle: Equatable {
    let id: Int
    let following: Bool
    let refusal: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = (json["id"] as? NSNumber)?.intValue else { return nil }
        self.id = id
        following = (json["following"] as? NSNumber)?.boolValue ?? false
        refusal = (json["refusal"] as? String) ?? ""
    }
}

struct FollowMe: Equatable {
    let mode: String
    let enabled: Bool
    let wouldSend: Bool
    let reason: String
    let fixValid: Bool?
    let fixFresh: Bool?
    let fixAgeMs: Int?
    let vehicles: [FollowMeVehicle]

    static let absent = FollowMe()

    private init() {
        mode = ""
        enabled = false
        wouldSend = false
        reason = ""
        fixValid = nil
        fixFresh = nil
        fixAgeMs = nil
        vehicles = []
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        mode = (json["mode"] as? String) ?? ""
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
        wouldSend = (json["wouldSend"] as? NSNumber)?.boolValue ?? false
        reason = (json["reason"] as? String) ?? ""
        fixValid = (json["fixValid"] as? NSNumber)?.boolValue
        fixFresh = (json["fixFresh"] as? NSNumber)?.boolValue
        fixAgeMs = (json["fixAgeMs"] as? NSNumber)?.intValue
        vehicles = ((json["vehicles"] as? [Any]) ?? []).compactMap(FollowMeVehicle.init)
    }

    // The core sends a TOKEN and this head spells it, which is the split everywhere else: the core
    // decides what is true, the head decides what an operator reads. Nine reasons, and each says a
    // different thing about whose problem it is -- a setting to change, a flight mode to select, or
    // a machine whose position nobody can get.
    static let sentences = [
        "modeUnknown": "Follow Me is set to something this build does not recognise.",
        "modeNever": "Follow Me is switched off.",
        "noVehicles": "No vehicle is connected to follow you.",
        "noVehicleInFollowMode": "No vehicle is in Follow mode.",
        "noFix": "This machine has no position to send.",
        "fixInvalid": "This machine's position has not resolved.",
        "fixStale": "This machine's position stopped updating.",
        "fixUnusable": "This machine's position is not accurate enough to send.",
        "allVehiclesRefused": "Every connected vehicle refused to follow.",
    ]

    // An unrecognised token is reported AS unrecognised. The core gaining a tenth reason while this
    // head prints "Follow Me is switched off" would hide a new failure behind an old sentence, and
    // the operator would read a settled state where there is a new one. A token nobody has spelled
    // yet is still more informative than a confident wrong sentence.
    var sentence: String {
        guard !reason.isEmpty else {
            return wouldSend ? "Sending your position." : ""
        }
        return FollowMe.sentences[reason] ?? "Follow Me is not sending: \(reason)."
    }

    // fixFresh is nullable because there may be no fix at all, and null is not fresh -- the same
    // rule the obstacle panel needed. A machine that has never reported a position and one
    // reporting on time must not render alike.
    var fixKnown: Bool { fixValid != nil }

    var following: [FollowMeVehicle] { vehicles.filter(\.following) }

    var sending: Bool { wouldSend && reason.isEmpty }

    // The Fly view says nothing when Follow Me is simply off, which is the state almost every
    // flight is in: a row reading "Follow Me is switched off" on every screen is noise, and noise
    // is what an operator learns to stop reading. It earns its place when it is sending, or when
    // it is meant to be on and something is stopping it -- the case where silence would leave the
    // operator believing the vehicle is following them.
    // THE PREMISE OF THAT GUARD WAS FALSE, and the default setting is what made it false.
    // followTarget defaults to 2, "When in Follow Me Flight Mode" (App.SettingsGroup.json), which
    // the core spells as mode "followMe" -- so on a factory install, with nobody having configured
    // anything, the reason is noVehicleInFollowMode and this drew an amber row on the primary
    // flight screen on every flight, forever. The row written to avoid noise WAS the noise. Found
    // by looking at the rendered window and then asking what the setting defaults to.
    //
    // A default is not an expressed intention. These two tokens are the ones where nothing has been
    // asked of Follow Me, and neither is a warning; every other reason means the operator asked and
    // something is stopping it, which is the case this row exists for.
    static let resting: Set<String> = ["modeNever", "noVehicleInFollowMode"]

    var worthShowing: Bool {
        sending || (!FollowMe.resting.contains(reason) && !sentence.isEmpty)
    }

    var level: FlyTelemetry.Level {
        guard !sending else { return .good }
        guard mode == "followMe" || mode == "always" else { return .unknown }
        return .warning
    }
}
