import Foundation

let PAUSE = "pause"

let BAR_ACTIONS: Set<String> = ["arm", "disarm", "takeoff", "land", "rtl", "changeSpeed", "changeAltitude"]

let SHEET_ACTIONS = [
    "startMission", "continueMission", "resumeMission", "cancelRoi", PAUSE,
    "landAbort", "release", "grab", "hold", "vtolTransitionToFixedWing",
    "vtolTransitionToMultiRotor", "forceArm",
]

let GRIPPER_RELEASE = 0
private let GRIPPER_GRAB = 1
private let GRIPPER_HOLD = 2
private let LAND_ABORT_CLIMB_METERS = 50.0

func moreActions(_ offers: [String: GuidedOffer]) -> [GuidedOffer] {
    offers.values
        .filter { $0.shown && !BAR_ACTIONS.contains($0.id) && $0.id != EMERGENCY_STOP && SHEET_ACTIONS.contains($0.id) }
        .sorted { $0.order < $1.order }
}

private let LAND_ABORT = "landAbort"

let AUTO_POPUP_ACTIONS = [LAND_ABORT, "startMission", "continueMission"]

func popupReplacesOpenConfirm(_ id: String) -> Bool { id == LAND_ABORT }

func autoMissionPopup(_ wasReady: Set<String>, _ offers: [String: GuidedOffer], _ enabled: Bool) -> GuidedOffer? {
    AUTO_POPUP_ACTIONS.compactMap { offers[$0] }.first { (enabled || $0.id == LAND_ABORT) && $0.ready && !wasReady.contains($0.id) }
}

func guidedCommand(_ id: String, _ resumeFrom: Int?) -> (() async -> String?)? {
    switch id {
    case "startMission", "continueMission": { await offMain { VehicleCommands.startMissionRefusal() } }
    case "landAbort": { await offMain { VehicleCommands.abortLandingRefusal(LAND_ABORT_CLIMB_METERS) } }
    case "grab": { await offMain { VehicleCommands.gripperRefusal(GRIPPER_GRAB) } }
    case "release": { await offMain { VehicleCommands.gripperRefusal(GRIPPER_RELEASE) } }
    case "hold": { await offMain { VehicleCommands.gripperRefusal(GRIPPER_HOLD) } }
    case "cancelRoi": { await offMain { VehicleCommands.stopRoiRefusal() } }
    case "vtolTransitionToFixedWing": { await offMain { VehicleCommands.forwardFlightRefusal(true) } }
    case "vtolTransitionToMultiRotor": { await offMain { VehicleCommands.forwardFlightRefusal(false) } }
    case "forceArm": { await offMain { VehicleCommands.forceArmRefusal() } }
    case "resumeMission": resumeFrom.map { at in { await offMain { PlanCommands.resumeMissionRefusal(at) } } }
    default: nil
    }
}

let AIRCRAFT_SECTION = "Aircraft"
let SCREEN_SECTION = "Screen"
let CALIBRATE = "Calibrate"
let AUX_SWITCHES_VIEW = "view.auxSwitches"
let AUX_SWITCH = "vehicle.auxSwitch"

struct AuxSwitch: Equatable {
    let id: String
    let on: Bool
    let action: String
    let confirm: PursuitConfirm?
}

func auxSwitchReadings(_ view: JSON?) -> [AuxSwitch] {
    (view?["switches"].objects ?? []).map {
        AuxSwitch(
            id: $0["id"].string,
            on: $0["on"].bool,
            action: $0["action"].string,
            confirm: $0["confirm"].objectOrNil.map { PursuitConfirm(title: $0["title"].string, text: $0["text"].string) }
        )
    }
}

func auxSwitchIcon(_ id: String) -> Icon {
    id == "avoidance" ? .shield : .straighten
}
