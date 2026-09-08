import Foundation

struct GuidedState: Equatable {
    var connected = false
    var armed = false
    var flying = false
    var hasGripper = false
    var guidedSupported = false
    var takeoffSupported = false
    var pauseSupported = false
    var fixedWing = false
    var forwardFlight = false
    var speedLimitsAvailable = false
    var landing = false
    var readyToArm = false
    var flightMode = ""
    var rtlMode = ""
    var landMode = ""
    var missionMode = ""
    var missionAvailable = false
    var missionItemCount = 0
    var currentMissionIndex = -1

    var inRTL: Bool { !rtlMode.isEmpty && flightMode == rtlMode }
    var inLand: Bool { !landMode.isEmpty && flightMode == landMode }
    var inMission: Bool { !missionMode.isEmpty && flightMode == missionMode }
    var missionActive: Bool { armed && (inLand || inRTL || inMission) }
    var onApproach: Bool { fixedWing && landing }
    var hasMoreMission: Bool { currentMissionIndex < missionItemCount - 1 }
}

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

    var title: String {
        switch self {
        case .arm: return "Arm"
        case .takeoff: return "Takeoff"
        case .startMission: return "Start Mission"
        case .continueMission: return "Continue Mission"
        case .pause: return "Pause"
        case .changeAltitude: return "Change Altitude"
        case .changeSpeed: return "Change Speed"
        case .landAbort: return "Abort Landing"
        case .land: return "Land"
        case .rtl: return "Return"
        case .disarm: return "Disarm"
        case .grab: return "Grab"
        case .release: return "Release"
        case .emergencyStop: return "Emergency Stop"
        }
    }

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

    var destructive: Bool { self == .emergencyStop }

    var prompt: String {
        switch self {
        case .arm: return "Arm the vehicle. Propellers will be live."
        case .takeoff: return "Take off and climb to the height you set."
        case .startMission: return "Fly the mission from the beginning."
        case .continueMission: return "Fly the rest of the mission from the current item."
        case .pause: return "Hold position, at the height you set."
        case .changeAltitude: return "Climb or descend to a new height."
        case .changeSpeed: return "Fly at a new speed."
        case .landAbort: return "Break off the landing and climb away."
        case .land: return "Land where it is."
        case .rtl: return "Fly home and land."
        case .disarm: return "Disarm the vehicle."
        case .grab: return "Close the gripper and hold the cargo."
        case .release: return "Open the gripper and drop the cargo."
        case .emergencyStop: return "Stop the motors immediately. The vehicle will fall."
        }
    }

    enum Offer: Equatable {
        case hidden
        case ready
        case blocked(String)
    }

    static let prearmReason = "Prearm checks are failing"

    func shown(in state: GuidedState) -> Bool {
        guard state.connected else { return false }
        switch self {
        case .arm: return !state.armed
        case .disarm: return state.armed && !state.flying
        case .grab, .release: return state.hasGripper && !state.armed
        case .rtl: return state.armed && state.guidedSupported && state.flying && !state.inRTL
        case .takeoff: return state.takeoffSupported && !state.flying
        case .land: return state.guidedSupported && state.armed && !state.fixedWing && !state.inLand
        case .startMission: return state.missionAvailable && !state.missionActive && !state.flying
        case .continueMission:
            return state.missionAvailable && !state.missionActive && state.armed
                && state.flying && state.hasMoreMission
        case .pause:
            return state.armed && state.pauseSupported && state.flying && !state.onApproach
        case .landAbort: return state.flying && state.onApproach
        case .changeAltitude:
            return state.armed && state.guidedSupported && state.flying && !state.missionActive
        case .changeSpeed:
            return state.armed && state.guidedSupported && state.flying
                && !state.missionActive && state.speedLimitsAvailable
        case .emergencyStop: return state.armed && state.flying
        }
    }

    var carriesValue: Bool {
        self == .takeoff || self == .changeAltitude || self == .changeSpeed || self == .pause
    }

    var needsPrearm: Bool {
        self == .arm || self == .takeoff || self == .startMission
    }

    func offer(in state: GuidedState) -> Offer {
        guard shown(in: state) else { return .hidden }
        if needsPrearm, !state.readyToArm { return .blocked(GuidedAction.prearmReason) }
        return .ready
    }

    func available(in state: GuidedState) -> Bool {
        offer(in: state) == .ready
    }

    static func offered(in state: GuidedState) -> [GuidedAction] {
        allCases.filter { $0.shown(in: state) }
    }

    static func available(in state: GuidedState) -> [GuidedAction] {
        allCases.filter { $0.available(in: state) }
    }
}
