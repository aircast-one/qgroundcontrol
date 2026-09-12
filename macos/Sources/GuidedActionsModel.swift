import Foundation

enum GuidedAction: String, CaseIterable, Identifiable {
    case arm
    case takeoff
    case startMission
    case continueMission
    case resumeMission
    case pause
    case changeAltitude
    case changeSpeed
    case landAbort
    case land
    case rtl
    case disarm
    case grab
    case release
    case emergencyStop
    case vtolTransitionToFixedWing
    case vtolTransitionToMultiRotor
    case forceArm

    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .arm: return "bolt.circle"
        case .takeoff: return "arrow.up.circle"
        case .startMission: return "play.circle"
        case .continueMission: return "forward.circle"
        case .resumeMission: return "arrow.trianglehead.clockwise"
        case .pause: return "pause.circle"
        case .changeAltitude: return "arrow.up.arrow.down.circle"
        case .changeSpeed: return "speedometer"
        case .landAbort: return "arrow.uturn.up.circle"
        case .land: return "arrow.down.circle"
        case .rtl: return "house.circle"
        case .disarm: return "bolt.slash.circle"
        case .grab: return "hand.raised.fill"
        case .release: return "hand.point.down.fill"
        case .emergencyStop: return "exclamationmark.octagon"
        case .vtolTransitionToFixedWing: return "airplane"
        case .vtolTransitionToMultiRotor: return "fan.desk"
        case .forceArm: return "exclamationmark.shield"
        }
    }
}

extension GuidedOffer {
    static func resumeSequence(_ json: Any?) -> Int? {
        guard let sequence = (json as? NSNumber)?.intValue, sequence > 0 else { return nil }
        return sequence
    }
}

struct GuidedOffer: Identifiable, Equatable {
    let action: GuidedAction
    let title: String
    let prompt: String
    let reason: String
    let destructive: Bool
    let carriesValue: Bool
    let shown: Bool
    let ready: Bool

    var id: String { action.rawValue }
    var blocked: Bool { shown && !ready }
    var explanation: String { reason.isEmpty ? prompt : reason }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let action = GuidedAction(rawValue: (json["id"] as? String) ?? ""),
              let offer = json["offer"] as? String else { return nil }
        self.action = action
        title = (json["title"] as? String) ?? ""
        prompt = (json["prompt"] as? String) ?? ""
        reason = (json["reason"] as? String) ?? ""
        destructive = (json["destructive"] as? NSNumber)?.boolValue ?? false
        carriesValue = (json["carriesValue"] as? NSNumber)?.boolValue ?? false
        shown = offer != "hidden"
        ready = offer == "ready"
    }

    static func list(_ json: Any?) -> [GuidedOffer] {
        ((json as? [Any]) ?? []).compactMap(GuidedOffer.init)
    }
}
