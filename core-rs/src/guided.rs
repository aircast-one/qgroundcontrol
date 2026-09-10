use serde::Serialize;
use serde_json::{Value, json};

use crate::read::{fact_flag, flag, integer, object, text, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.flyViewSettings.forwardFlightGoToLocationLoiterRad",
    "vehicles.activeVehicleAvailable",
    "vehicle.parameterManager.parametersReady",
    "vehicle.armed",
    "vehicle.flying",
    "vehicle.flightMode",
    "vehicle.landing",
    "vehicle.vtol",
    "vehicle.vtolInFwdFlight",
    "vehicle.initialConnectComplete",
    "vehicle.checkListState",
    "vehicle.healthAndArmingCheckReport.supported",
    "vehicle.healthAndArmingCheckReport.canArm",
    "vehicle.healthAndArmingCheckReport.canTakeoff",
    "vehicle.healthAndArmingCheckReport.canStartMission",
    "plan.missionController.containsItems",
    "planFly.missionController.missionItemCount",
    "planFly.missionController.currentMissionIndex",
    "settings.appSettings.useChecklist",
    "settings.appSettings.enforceChecklist",
];

const CHECKLIST_PASSED: i64 = 1;

#[derive(Default, PartialEq, Debug, Clone)]
pub struct GuidedState {
    pub connected: bool,
    pub armed: bool,
    pub flying: bool,
    pub guided_supported: bool,
    pub takeoff_supported: bool,
    pub pause_supported: bool,
    pub fixed_wing: bool,
    pub vtol: bool,
    pub vtol_in_fwd_flight: bool,
    pub forward_flight: bool,
    // What a goto has to carry in DO_REPOSITION param 3. Zero for anything that does not fly
    // forward, and the operator's setting for anything that does. QML derives this at the call
    // site; a head deriving it would be deriving vehicle knowledge, so it is answered here.
    pub goto_loiter_radius: f64,
    pub speed_limits: bool,
    pub landing: bool,
    pub has_gripper: bool,
    pub initial_connect_complete: bool,
    pub in_rtl: bool,
    pub in_land: bool,
    pub in_mission: bool,
    pub paused: bool,
    pub checklist_passed: bool,
    pub can_arm: bool,
    pub can_takeoff: bool,
    pub can_start_mission: bool,
    pub mission_available: bool,
    pub mission_item_count: i64,
    pub current_mission_index: i64,
}

impl GuidedState {
    fn mission_active(&self) -> bool {
        self.armed && (self.in_land || self.in_rtl || self.in_mission)
    }
    fn on_approach(&self) -> bool {
        self.fixed_wing && self.landing
    }
    fn has_more_mission(&self) -> bool {
        self.current_mission_index < self.mission_item_count - 1
    }
}

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Arm,
    Takeoff,
    StartMission,
    ContinueMission,
    Pause,
    ChangeAltitude,
    ChangeSpeed,
    LandAbort,
    Land,
    Rtl,
    Disarm,
    Grab,
    Release,
    EmergencyStop,
    VtolTransitionToFixedWing,
    VtolTransitionToMultiRotor,
    ForceArm,
}

pub const ACTIONS: &[Action] = &[
    Action::Arm,
    Action::Takeoff,
    Action::StartMission,
    Action::ContinueMission,
    Action::Pause,
    Action::ChangeAltitude,
    Action::ChangeSpeed,
    Action::LandAbort,
    Action::Land,
    Action::Rtl,
    Action::Disarm,
    Action::Grab,
    Action::Release,
    Action::EmergencyStop,
    Action::VtolTransitionToFixedWing,
    Action::VtolTransitionToMultiRotor,
    Action::ForceArm,
];

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub id: Action,
    pub title: &'static str,
    pub prompt: &'static str,
    pub offer: &'static str,
    pub reason: &'static str,
    pub destructive: bool,
    pub carries_value: bool,
}

impl Action {
    fn title(self) -> &'static str {
        match self {
            Action::Arm => "Arm",
            Action::Takeoff => "Takeoff",
            Action::StartMission => "Start Mission",
            Action::ContinueMission => "Continue Mission",
            Action::Pause => "Pause",
            Action::ChangeAltitude => "Change Altitude",
            Action::ChangeSpeed => "Change Speed",
            Action::LandAbort => "Abort Landing",
            Action::Land => "Land",
            Action::Rtl => "Return",
            Action::Disarm => "Disarm",
            Action::Grab => "Grab",
            Action::Release => "Release",
            Action::EmergencyStop => "Emergency Stop",
            Action::VtolTransitionToFixedWing => "Transition to Fixed Wing",
            Action::VtolTransitionToMultiRotor => "Transition to Multi-Rotor",
            Action::ForceArm => "Force Arm",
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Action::Arm => "Arm the vehicle. Propellers will be live.",
            Action::Takeoff => "Take off and climb to the height you set.",
            Action::StartMission => "Fly the mission from the beginning.",
            Action::ContinueMission => "Fly the rest of the mission from the current item.",
            Action::Pause => "Hold position, at the height you set.",
            Action::ChangeAltitude => "Climb or descend to a new height.",
            Action::ChangeSpeed => "Fly at a new speed.",
            Action::LandAbort => "Break off the landing and climb away.",
            Action::Land => "Land where it is.",
            Action::Rtl => "Fly home and land.",
            Action::Disarm => "Disarm the vehicle.",
            Action::Grab => "Close the gripper and hold the cargo.",
            Action::Release => "Open the gripper and drop the cargo.",
            Action::EmergencyStop => "Stop the motors immediately. The vehicle will fall.",
            Action::VtolTransitionToFixedWing => "Transition VTOL to fixed wing flight.",
            Action::VtolTransitionToMultiRotor => "Transition VTOL to multi-rotor flight.",
            Action::ForceArm => "Arm the vehicle without its safety checks. Propellers will be live.",
        }
    }

    fn carries_value(self) -> bool {
        matches!(self, Action::Takeoff | Action::ChangeAltitude | Action::ChangeSpeed | Action::Pause)
    }

    fn shown(self, s: &GuidedState) -> bool {
        s.connected
            && match self {
                Action::Arm => !s.armed,
                Action::Disarm => s.armed && !s.flying,
                Action::Grab | Action::Release => s.initial_connect_complete && s.has_gripper,
                Action::Rtl => s.armed && s.guided_supported && s.flying && !s.in_rtl,
                Action::Takeoff => s.takeoff_supported && !s.flying,
                Action::Land => s.guided_supported && s.armed && !s.fixed_wing && !s.in_land,
                Action::StartMission => s.mission_available && !s.mission_active() && !s.flying,
                Action::ContinueMission => {
                    s.mission_available && !s.mission_active() && s.armed && s.flying && s.has_more_mission()
                }
                Action::Pause => s.armed && s.pause_supported && s.flying && !s.paused && !s.on_approach(),
                Action::LandAbort => s.flying && s.on_approach(),
                Action::ChangeAltitude => s.armed && s.guided_supported && s.flying && !s.mission_active(),
                Action::ChangeSpeed => {
                    s.armed && s.guided_supported && s.flying && !s.mission_active() && s.speed_limits
                }
                Action::EmergencyStop => s.armed && s.flying,
                Action::VtolTransitionToFixedWing => s.vtol && s.flying && !s.vtol_in_fwd_flight,
                Action::VtolTransitionToMultiRotor => s.vtol && s.flying && s.vtol_in_fwd_flight,
                Action::ForceArm => !s.armed && !s.can_arm,
            }
    }

    fn gate(self, s: &GuidedState) -> Option<&'static str> {
        let passes = match self {
            Action::Arm => s.can_arm,
            Action::Takeoff => s.can_takeoff,
            Action::StartMission => s.can_start_mission,
            _ => true,
        };
        match (passes, s.checklist_passed) {
            (true, _) => None,
            (false, false) => Some("The pre-flight checklist has not been completed."),
            (false, true) => Some("The vehicle's arming checks are failing."),
        }
    }

    pub fn offer(self, s: &GuidedState) -> Offer {
        let (offer, reason) = match (self.shown(s), self.gate(s)) {
            (false, _) => ("hidden", ""),
            (true, None) => ("ready", ""),
            (true, Some(reason)) => ("blocked", reason),
        };
        Offer {
            id: self,
            title: self.title(),
            prompt: self.prompt(),
            offer,
            reason,
            destructive: matches!(self, Action::EmergencyStop | Action::ForceArm),
            carries_value: self.carries_value(),
        }
    }
}

pub fn guided_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let state = read_state(backend);
    let offers: Vec<Offer> = ACTIONS.iter().map(|a| a.offer(&state)).collect();
    json!({
        "kind": "object",
        "class": "GuidedActions",
        "connected": state.connected,
        "missionActive": state.mission_active(),
        "forwardFlight": state.forward_flight,
        "gotoLoiterRadius": state.goto_loiter_radius,
        "actions": offers,
    })
}

fn speed_limits_live(backend: &dyn Backend, px4: bool, apm: bool, forward_flight: bool) -> bool {
    let names: &[&str] = match (forward_flight, px4, apm) {
        (false, true, _) => &["MPC_XY_VEL_MAX"],
        (false, false, true) => &["WPNAV_SPEED"],
        (true, true, _) => &["FW_AIRSPD_MIN", "FW_AIRSPD_MAX"],
        (true, false, true) => &["r.AIRSPEED_MIN", "r.AIRSPEED_MAX"],
        _ => return false,
    };
    names.iter().all(|name| crate::read::result_flag(&backend.invoke("vehicle.parameterManager.parameterExists", &json!([-1, name]).to_string())))
}

fn read_state(backend: &dyn Backend) -> GuidedState {
    let vehicles = object(&backend.get_fields("vehicles", "activeVehicleAvailable"));
    if !flag(&vehicles, "activeVehicleAvailable") {
        return GuidedState::default();
    }
    let vehicle = object(&backend.get_fields(
        "vehicle",
        "armed,flying,guidedModeSupported,takeoffVehicleSupported,pauseVehicleSupported,fixedWing,vtol,vtolInFwdFlight,haveFWSpeedLimits,haveMRSpeedLimits,px4Firmware,apmFirmware,landing,hasGripper,initialConnectComplete,checkListState,flightMode,rtlFlightMode,smartRTLFlightMode,landFlightMode,missionFlightMode,pauseFlightMode",
    ));
    let report = object(&backend.get_fields("vehicle.healthAndArmingCheckReport", "supported,canArm,canTakeoff,canStartMission"));
    let mission = object(&backend.get_fields("plan.missionController", "containsItems"));
    let flying = object(&backend.get_fields("planFly.missionController", "missionItemCount,currentMissionIndex"));
    let app = object(&backend.get_fields("settings.appSettings", "useChecklist,enforceChecklist"));
    let mode = text(&vehicle, "flightMode");
    let same_mode = |key: &str| !mode.is_empty() && text(&vehicle, key) == mode;
    let use_checklist = fact_flag(&app, "useChecklist");
    let enforce_checklist = fact_flag(&app, "enforceChecklist");
    let checklist_passed = !use_checklist || !enforce_checklist || integer(&vehicle, "checkListState") == Some(CHECKLIST_PASSED);
    let report_supported = flag(&report, "supported");
    let gate = |key: &str| checklist_passed && (!report_supported || flag(&report, key));
    let fixed_wing = flag(&vehicle, "fixedWing");
    let vtol_in_fwd_flight = flag(&vehicle, "vtolInFwdFlight");
    let forward_flight = vtol_in_fwd_flight || fixed_wing;
    GuidedState {
        connected: true,
        armed: flag(&vehicle, "armed"),
        flying: flag(&vehicle, "flying"),
        guided_supported: flag(&vehicle, "guidedModeSupported"),
        takeoff_supported: flag(&vehicle, "takeoffVehicleSupported"),
        pause_supported: flag(&vehicle, "pauseVehicleSupported"),
        fixed_wing,
        vtol: flag(&vehicle, "vtol"),
        vtol_in_fwd_flight,
        forward_flight,
        goto_loiter_radius: match forward_flight {
            true => value_number(&backend.get("settings.flyViewSettings.forwardFlightGoToLocationLoiterRad.rawValue")).unwrap_or(0.0),
            false => 0.0,
        },
        speed_limits: if forward_flight { flag(&vehicle, "haveFWSpeedLimits") } else { flag(&vehicle, "haveMRSpeedLimits") }
            || speed_limits_live(backend, flag(&vehicle, "px4Firmware"), flag(&vehicle, "apmFirmware"), forward_flight),
        landing: flag(&vehicle, "landing"),
        has_gripper: flag(&vehicle, "hasGripper"),
        initial_connect_complete: flag(&vehicle, "initialConnectComplete"),
        in_rtl: same_mode("rtlFlightMode") || same_mode("smartRTLFlightMode"),
        in_land: same_mode("landFlightMode"),
        in_mission: same_mode("missionFlightMode"),
        paused: same_mode("pauseFlightMode"),
        checklist_passed,
        can_arm: gate("canArm"),
        can_takeoff: gate("canTakeoff"),
        can_start_mission: gate("canStartMission"),
        mission_available: flag(&mission, "containsItems"),
        mission_item_count: integer(&flying, "missionItemCount").unwrap_or(0),
        current_mission_index: integer(&flying, "currentMissionIndex").unwrap_or(-1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer_of(state: &GuidedState, action: Action) -> &'static str {
        action.offer(state).offer
    }

    fn ready_on_ground() -> GuidedState {
        GuidedState {
            connected: true,
            guided_supported: true,
            takeoff_supported: true,
            pause_supported: true,
            initial_connect_complete: true,
            checklist_passed: true,
            can_arm: true,
            can_takeoff: true,
            can_start_mission: true,
            ..GuidedState::default()
        }
    }

    #[test]
    fn nothing_is_offered_without_a_vehicle() {
        let none = GuidedState::default();
        assert!(ACTIONS.iter().all(|a| offer_of(&none, *a) == "hidden"));
    }

    #[test]
    fn on_the_ground_arm_and_takeoff_are_ready_and_flight_actions_hidden() {
        let s = ready_on_ground();
        assert_eq!(offer_of(&s, Action::Arm), "ready");
        assert_eq!(offer_of(&s, Action::Takeoff), "ready");
        assert_eq!(offer_of(&s, Action::Rtl), "hidden");
        assert_eq!(offer_of(&s, Action::Pause), "hidden");
        assert_eq!(offer_of(&s, Action::EmergencyStop), "hidden");
        assert_eq!(offer_of(&s, Action::Disarm), "hidden");
        assert_eq!(offer_of(&s, Action::Grab), "hidden");
    }

    #[test]
    fn a_failing_gate_blocks_with_the_right_reason() {
        let checks = GuidedState { can_arm: false, ..ready_on_ground() };
        let arm = Action::Arm.offer(&checks);
        assert_eq!(arm.offer, "blocked");
        assert_eq!(arm.reason, "The vehicle's arming checks are failing.");
        let checklist = GuidedState { can_arm: false, checklist_passed: false, ..ready_on_ground() };
        assert_eq!(Action::Arm.offer(&checklist).reason, "The pre-flight checklist has not been completed.");
        assert_eq!(offer_of(&checks, Action::Takeoff), "ready");
    }

    #[test]
    fn in_flight_the_return_button_hides_while_returning_by_the_vehicles_own_mode_name() {
        let flying = GuidedState { armed: true, flying: true, speed_limits: true, ..ready_on_ground() };
        assert_eq!(offer_of(&flying, Action::Rtl), "ready");
        assert_eq!(offer_of(&flying, Action::ChangeSpeed), "ready");
        assert_eq!(offer_of(&flying, Action::EmergencyStop), "ready");
        assert_eq!(offer_of(&flying, Action::Arm), "hidden");
        let returning = GuidedState { in_rtl: true, ..flying.clone() };
        assert_eq!(offer_of(&returning, Action::Rtl), "hidden");
        assert_eq!(offer_of(&returning, Action::ChangeAltitude), "hidden");
        assert_eq!(offer_of(&returning, Action::Pause), "ready");
    }

    #[test]
    fn mission_actions_follow_the_item_cursor() {
        let mid = GuidedState { armed: true, flying: true, mission_available: true, mission_item_count: 5, current_mission_index: 2, ..ready_on_ground() };
        assert_eq!(offer_of(&mid, Action::ContinueMission), "ready");
        assert_eq!(offer_of(&mid, Action::StartMission), "hidden");
        let done = GuidedState { current_mission_index: 4, ..mid.clone() };
        assert_eq!(offer_of(&done, Action::ContinueMission), "hidden");
        let ground = GuidedState { armed: false, flying: false, ..mid };
        assert_eq!(offer_of(&ground, Action::StartMission), "ready");
    }

    #[test]
    fn force_arm_appears_only_where_the_ordinary_arm_was_refused() {
        let ready = ready_on_ground();
        assert_eq!(offer_of(&ready, Action::Arm), "ready");
        assert_eq!(offer_of(&ready, Action::ForceArm), "hidden", "a vehicle that will arm normally is never offered the way past its checks");
        let refused = GuidedState { can_arm: false, ..ready_on_ground() };
        assert_eq!(offer_of(&refused, Action::Arm), "blocked", "the ordinary arm is blocked by the failing checks");
        assert_eq!(offer_of(&refused, Action::ForceArm), "ready", "force arm is the escape hatch, so the checks that refused the arm do not gate it");
        let force = Action::ForceArm.offer(&refused);
        assert!(force.destructive, "bypassing the safety checks is destructive and a head must say so");
        assert!(!force.carries_value);
        assert!(!Action::Arm.offer(&ready).destructive, "the ordinary arm is not the destructive one");
        let armed = GuidedState { armed: true, can_arm: false, ..ready_on_ground() };
        assert_eq!(offer_of(&armed, Action::ForceArm), "hidden");
    }

    #[test]
    fn only_a_vtol_is_offered_a_transition_and_only_the_one_it_is_not_in() {
        let aloft = GuidedState { armed: true, flying: true, ..ready_on_ground() };
        assert_eq!(offer_of(&aloft, Action::VtolTransitionToFixedWing), "hidden", "a vehicle that cannot transition is never asked to");
        assert_eq!(offer_of(&aloft, Action::VtolTransitionToMultiRotor), "hidden");
        let grounded = GuidedState { vtol: true, ..ready_on_ground() };
        assert_eq!(offer_of(&grounded, Action::VtolTransitionToFixedWing), "hidden", "a VTOL on the ground is never offered a transition, as QGC only opens that drawer in the air");
        let rotor = GuidedState { vtol: true, ..aloft.clone() };
        assert_eq!(offer_of(&rotor, Action::VtolTransitionToFixedWing), "ready");
        assert_eq!(offer_of(&rotor, Action::VtolTransitionToMultiRotor), "hidden", "a vehicle already in multi-rotor flight is not offered the way it came");
        let wing = GuidedState { vtol: true, vtol_in_fwd_flight: true, ..aloft.clone() };
        assert_eq!(offer_of(&wing, Action::VtolTransitionToMultiRotor), "ready");
        assert_eq!(offer_of(&wing, Action::VtolTransitionToFixedWing), "hidden");
        let gone = GuidedState { connected: false, vtol: true, flying: true, ..GuidedState::default() };
        assert_eq!(offer_of(&gone, Action::VtolTransitionToFixedWing), "hidden");
        assert!(!Action::VtolTransitionToFixedWing.offer(&rotor).destructive, "a transition is not a destructive action");
        assert!(!Action::VtolTransitionToFixedWing.offer(&rotor).carries_value, "a transition takes no value from the operator");
    }

    #[test]
    fn fixed_wing_on_approach_offers_abort_instead_of_pause_and_land() {
        let approach = GuidedState { armed: true, flying: true, fixed_wing: true, landing: true, ..ready_on_ground() };
        assert_eq!(offer_of(&approach, Action::LandAbort), "ready");
        assert_eq!(offer_of(&approach, Action::Pause), "hidden");
        assert_eq!(offer_of(&approach, Action::Land), "hidden");
    }

    #[test]
    fn the_view_reads_the_checklist_and_report_gates_from_the_bridge() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                    "vehicle" => json!({ "kind": "object", "armed": false, "flying": false, "takeoffVehicleSupported": true, "guidedModeSupported": true, "checkListState": 0, "flightMode": "Hold", "rtlFlightMode": "Return" }),
                    "vehicle.healthAndArmingCheckReport" => json!({ "kind": "object", "supported": true, "canArm": false, "canTakeoff": true, "canStartMission": true }),
                    "plan.missionController" => json!({ "kind": "object", "containsItems": false }),
                    "settings.appSettings" => json!({ "kind": "object", "facts": [ { "name": "useChecklist", "value": true }, { "name": "enforceChecklist", "value": false } ] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = guided_view(&Fake, &[]);
        assert_eq!(view["connected"], true);
        let arm = &view["actions"][0];
        assert_eq!(arm["id"], "arm");
        assert_eq!(arm["offer"], "blocked");
        assert_eq!(arm["reason"], "The vehicle's arming checks are failing.");
        assert_eq!(view["actions"][1]["id"], "takeoff");
        assert_eq!(view["actions"][1]["offer"], "ready");
        assert_eq!(view["actions"][13]["id"], "emergencyStop");
        assert_eq!(view["actions"][13]["destructive"], true);
        assert_eq!(view["actions"][1]["carriesValue"], true);
        assert!(view["actions"][1].get("carries_value").is_none());
    }

    #[test]
    fn the_speed_offer_follows_the_live_parameter_when_the_latched_flag_is_stale() {
        struct Fake(bool);
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }).to_string(),
                    "vehicle" => json!({ "kind": "object", "armed": true, "flying": true, "guidedModeSupported": true, "haveMRSpeedLimits": false, "apmFirmware": true, "px4Firmware": false, "flightMode": "Guided" }).to_string(),
                    _ => json!({ "kind": "object" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                assert_eq!(path, "vehicle.parameterManager.parameterExists");
                assert!(args.contains("WPNAV_SPEED"));
                json!({ "ok": true, "result": self.0 }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        assert!(read_state(&Fake(true)).speed_limits);
        assert!(!read_state(&Fake(false)).speed_limits);
    }
}

#[cfg(test)]
mod loiter {
    use super::*;

    struct Flying { forward: bool, setting: f64 }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.flyViewSettings.forwardFlightGoToLocationLoiterRad.rawValue" => json!({ "kind": "value", "value": self.setting }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }).to_string(),
                "vehicle" => json!({
                    "kind": "object", "armed": true, "flying": true, "guidedModeSupported": true,
                    "fixedWing": self.forward, "vtolInFwdFlight": false, "flightMode": "Guided",
                }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_goto_carries_the_operators_loiter_radius_only_when_the_vehicle_flies_forward() {
        let winged = guided_view(&Flying { forward: true, setting: 120.0 }, &[]);
        assert_eq!(winged["forwardFlight"], true);
        assert_eq!(winged["gotoLoiterRadius"], 120.0, "a fixed wing cannot hover, so a goto it is sent has to say how wide to circle");

        let hovering = guided_view(&Flying { forward: false, setting: 120.0 }, &[]);
        assert_eq!(hovering["forwardFlight"], false);
        assert_eq!(hovering["gotoLoiterRadius"], 0.0, "a multirotor holds position, and sending it a radius would be sending a number nobody chose");
    }

    #[test]
    fn a_setting_the_app_does_not_answer_is_no_radius_rather_than_a_guess() {
        struct Silent;
        impl Backend for Silent {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }).to_string(),
                    "vehicle" => json!({ "kind": "object", "fixedWing": true, "armed": true, "flying": true }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(guided_view(&Silent, &[])["gotoLoiterRadius"], 0.0);
    }
}
