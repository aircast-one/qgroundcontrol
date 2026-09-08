import Foundation

enum GuidedAction: String, CaseIterable, Identifiable {
    case arm
    case takeoff
    case startMission
    case continueMission
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

    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .arm: return "bolt.circle"
        case .takeoff: return "arrow.up.circle"
        case .startMission: return "play.circle"
        case .continueMission: return "forward.circle"
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
        }
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
    let blocked: Bool

    var id: String { action.rawValue }
    var ready: Bool { shown && !blocked }
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
        let carries = json["carriesValue"] ?? json["carries_value"]
        carriesValue = (carries as? NSNumber)?.boolValue ?? false
        shown = offer != "hidden"
        blocked = offer == "blocked"
    }

    static func list(_ json: Any?) -> [GuidedOffer] {
        ((json as? [Any]) ?? []).compactMap(GuidedOffer.init)
    }
}
