import Foundation

let PAUSE = "pause"

let BAR_ACTIONS: Set<String> = ["arm", "disarm", "takeoff", "land", "rtl", "changeSpeed", "changeAltitude"]

let SHEET_ACTIONS = [
    "startMission", "continueMission", "resumeMission", "cancelRoi", PAUSE,
    "landAbort", "release", "grab", "hold", "vtolTransitionToFixedWing",
    "vtolTransitionToMultiRotor", "forceArm",
]

private let GRIPPER_RELEASE = 0
private let GRIPPER_GRAB = 1
private let GRIPPER_HOLD = 2
private let LAND_ABORT_CLIMB_METERS = 50.0

func moreActions(_ offers: [String: GuidedOffer]) -> [GuidedOffer] {
    SHEET_ACTIONS.compactMap { offers[$0] }.filter { $0.shown && !BAR_ACTIONS.contains($0.id) && $0.id != EMERGENCY_STOP }
}

private let LAND_ABORT = "landAbort"

let AUTO_POPUP_ACTIONS = [LAND_ABORT, "startMission", "continueMission"]

func popupReplacesOpenConfirm(_ id: String) -> Bool { id == LAND_ABORT }

func autoMissionPopup(_ wasReady: Set<String>, _ offers: [String: GuidedOffer], _ enabled: Bool) -> GuidedOffer? {
    AUTO_POPUP_ACTIONS.compactMap { offers[$0] }.first { (enabled || $0.id == LAND_ABORT) && $0.ready && !wasReady.contains($0.id) }
}

func guidedCommand(_ id: String, _ resumeFrom: Int?) -> (() -> Void)? {
    switch id {
    case "startMission", "continueMission": { offMain { VehicleCommands.startMission() } }
    case "landAbort": { offMain { VehicleCommands.abortLanding(LAND_ABORT_CLIMB_METERS) } }
    case "grab": { offMain { VehicleCommands.gripper(GRIPPER_GRAB) } }
    case "release": { offMain { VehicleCommands.gripper(GRIPPER_RELEASE) } }
    case "hold": { offMain { VehicleCommands.gripper(GRIPPER_HOLD) } }
    case "cancelRoi": { offMain { VehicleCommands.stopRoi() } }
    case "vtolTransitionToFixedWing": { offMain { VehicleCommands.setForwardFlight(true) } }
    case "vtolTransitionToMultiRotor": { offMain { VehicleCommands.setForwardFlight(false) } }
    case "forceArm": { offMain { VehicleCommands.forceArm() } }
    case "resumeMission": resumeFrom.map { at in { offMain { PlanCommands.resumeMission(at) } } }
    default: nil
    }
}
