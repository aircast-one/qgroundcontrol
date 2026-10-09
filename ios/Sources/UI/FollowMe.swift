import Foundation

let FOLLOW_ME_VIEW = "view.followMe"

struct FollowMeReading: Equatable {
    var mode: String
    var enabled: Bool
    var wouldSend: Bool
    var reason: String
    var following: Int
    var vehicles: Int
}

func followMeReading(_ view: JSON?) -> FollowMeReading? {
    guard let view, view["class"].string == "FollowMe" else { return nil }
    let vehicles = view["vehicles"].objects
    return FollowMeReading(
        mode: view["mode"].string,
        enabled: view["enabled"].bool,
        wouldSend: view["wouldSend"].bool,
        reason: view["reason"].string,
        following: vehicles.filter { $0["following"].bool }.count,
        vehicles: vehicles.count
    )
}

func followMeTrouble(_ reason: String) -> String {
    switch reason {
    case "modeUnknown": "the Follow Me setting is not one this app understands"
    case "modeNever": "Follow Me is switched off"
    case "noVehicles": "no vehicle is connected"
    case "noVehicleInFollowMode": "no vehicle is in Follow Me mode"
    case "noFix": "this phone has no position yet"
    case "fixInvalid": "this phone's position is not valid"
    case "fixStale": "this phone's position has stopped updating"
    case "fixUnusable": "this phone's position cannot be used"
    case "allVehiclesRefused": "the vehicle refused the position"
    default: "the position is not being sent"
    }
}

func followMeAsked(_ mode: String) -> Bool { mode == "always" || mode == "followMe" }

func followMeResting(_ reason: String) -> Bool { reason == "noVehicleInFollowMode" }

func followMeLabel(_ reading: FollowMeReading?) -> String? {
    guard let reading, followMeAsked(reading.mode), reading.vehicles != 0, !followMeResting(reading.reason) else { return nil }
    guard reading.wouldSend else { return "Not following you \u{2014} \(followMeTrouble(reading.reason))" }
    return reading.following <= 1 ? "Following you" : "Following you \u{00B7} \(reading.following) vehicles"
}
