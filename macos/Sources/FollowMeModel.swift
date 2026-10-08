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
    let vehicles: [FollowMeVehicle]

    static let absent = FollowMe()

    private init() {
        mode = ""
        enabled = false
        wouldSend = false
        reason = ""
        fixValid = nil
        fixFresh = nil
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
        vehicles = ((json["vehicles"] as? [Any]) ?? []).compactMap(FollowMeVehicle.init)
    }

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

    var sentence: String {
        guard !reason.isEmpty else {
            return wouldSend ? "Sending your position." : ""
        }
        return FollowMe.sentences[reason] ?? "Follow Me is not sending: \(reason)."
    }

    var fixKnown: Bool { fixValid != nil }

    var following: [FollowMeVehicle] { vehicles.filter(\.following) }

    var sending: Bool { wouldSend && reason.isEmpty }

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
