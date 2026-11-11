use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const VIEW: &str = "view.payloadRail";
pub const ARM_RAIL: &str = "payloadRail.arm";
pub const SELECT_BAY: &str = "payloadRail.select";
pub const DROP_BAY: &str = "payloadRail.drop";
pub const DROP_ALL: &str = "payloadRail.dropAll";
pub const RAIL_CHANGED: &str = "core.payloadRail@changed";
pub const DEPS: &[&str] = &[
    "settings.flyViewSettings.payloadBays",
    "vehicle.payloadSetup",
    "vehicles.activeVehicleAvailable",
    "vehicle.id",
    "vehicle.gripper",
    RAIL_CHANGED,
];
pub const SAFE: &str = "The payload is safe. Arm it first.";
pub const RADIO_ARMS: &str = "The radio's BOMARM switch arms the payload while it streams.";
pub const RADIO_SELECTS: &str = "The radio's 6POS switch picks the bay while it streams.";
pub const RADIO_STALE_MS: u64 = crate::usbradio::STALE_AFTER_MS;
pub const RADIO: &str = "radio";
pub const APP: &str = "app";
pub const SWITCH_ARMED: &str = "armed";
pub const SWITCH_SAFE: &str = "safe";
pub const SWITCH_REFUSED: &str = "refused";
pub const SWITCH_OFF: &str = "off";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioSwitches {
    pub armed: bool,
    pub bay: Option<i64>,
    pub at_ms: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rail {
    armed: bool,
    selected: i64,
    radio: Option<RadioSwitches>,
    seen: Option<RadioSwitches>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owned<T> {
    pub value: T,
    pub by: &'static str,
}

impl Rail {
    fn radio(&self, now_ms: u64) -> Option<RadioSwitches> {
        self.radio
            .filter(|radio| now_ms.saturating_sub(radio.at_ms) <= RADIO_STALE_MS)
    }

    pub fn armed(&self, now_ms: u64) -> Owned<bool> {
        match self.radio(now_ms) {
            Some(radio) => Owned {
                value: radio.armed,
                by: RADIO,
            },
            None => Owned {
                value: self.armed,
                by: APP,
            },
        }
    }

    pub fn selected(&self, now_ms: u64) -> Owned<i64> {
        match self.radio(now_ms).and_then(|radio| radio.bay) {
            Some(bay) => Owned {
                value: bay,
                by: RADIO,
            },
            None => Owned {
                value: self.selected.max(1),
                by: APP,
            },
        }
    }

    pub fn heard(&mut self, switches: RadioSwitches) -> bool {
        let changed = self
            .radio
            .is_none_or(|was| (was.armed, was.bay) != (switches.armed, switches.bay));
        self.radio = Some(switches);
        changed
    }

    pub fn seen(&mut self, switches: Option<RadioSwitches>) {
        self.seen = switches;
    }

    pub fn verdict(&self, has_payload: bool, now_ms: u64) -> &'static str {
        match self.seen {
            None => SWITCH_OFF,
            Some(RadioSwitches { armed: false, .. }) => SWITCH_SAFE,
            Some(_) => match self.armed(now_ms) {
                Owned {
                    value: true,
                    by: RADIO,
                } if has_payload => SWITCH_ARMED,
                _ => SWITCH_REFUSED,
            },
        }
    }

    pub fn arm(&mut self, armed: bool, now_ms: u64) -> Result<(), &'static str> {
        match self.radio(now_ms) {
            Some(_) => Err(RADIO_ARMS),
            None => {
                self.armed = armed;
                Ok(())
            }
        }
    }

    pub fn select(&mut self, bay: i64, now_ms: u64) -> Result<(), &'static str> {
        match self.radio(now_ms).and_then(|radio| radio.bay) {
            Some(_) => Err(RADIO_SELECTS),
            None => {
                self.selected = bay.max(1);
                Ok(())
            }
        }
    }
}

static RAIL: Mutex<Rail> = Mutex::new(Rail {
    armed: false,
    selected: 1,
    radio: None,
    seen: None,
});
static CHANGED_FLAG: AtomicBool = AtomicBool::new(false);

fn rail() -> MutexGuard<'static, Rail> {
    RAIL.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn radio_heard(switches: RadioSwitches) {
    if rail().heard(switches) {
        CHANGED_FLAG.store(true, Ordering::Relaxed);
    }
}

pub fn radio_seen(switches: Option<RadioSwitches>) {
    rail().seen(switches);
}

pub fn verdict() -> &'static str {
    let has_payload = shown();
    rail().verdict(has_payload, crate::hub::now_ms())
}

fn shown() -> bool {
    crate::hub::lock().active().is_some_and(|vehicle| {
        vehicle.payload_setup().enabled || !vehicle.payload_servos().is_empty()
    })
}

pub fn take_changed() -> bool {
    CHANGED_FLAG.swap(false, Ordering::Relaxed)
}

pub fn armed() -> bool {
    rail().armed(crate::hub::now_ms()).value
}

pub fn selected() -> i64 {
    rail().selected(crate::hub::now_ms()).value
}

pub fn owns(path: &str) -> bool {
    [ARM_RAIL, SELECT_BAY, DROP_BAY, DROP_ALL].contains(&path)
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

fn settled(outcome: Result<(), &'static str>) -> Value {
    CHANGED_FLAG.store(true, Ordering::Relaxed);
    match outcome {
        Ok(()) => json!({ "ok": true }),
        Err(reason) => refused(reason),
    }
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let now_ms = crate::hub::now_ms();
    match path {
        ARM_RAIL => settled(rail().arm(
            given.first().and_then(Value::as_bool).unwrap_or(false),
            now_ms,
        )),
        SELECT_BAY => match given
            .first()
            .and_then(Value::as_i64)
            .filter(|bay| *bay >= 1)
        {
            Some(bay) => {
                let reloaded = empty(bay)
                    && crate::guided::dispatch(
                        backend,
                        Some(json!({ "action": "bayLoaded", "bay": bay })),
                        crate::guided::active_id(backend),
                        SELECT_BAY,
                        "[]",
                    )["ok"]
                        == json!(true);
                match (rail().select(bay, now_ms), reloaded) {
                    (Err(_), true) => settled(Ok(())),
                    (picked, _) => settled(picked),
                }
            }
            None => refused("Name the bay to pick, from 1."),
        },
        DROP_BAY => drop(backend),
        DROP_ALL => drop_all(backend),
        _ => refused("Unknown payload rail action."),
    }
}

fn empty(bay: i64) -> bool {
    crate::hub::lock()
        .active()
        .is_some_and(|vehicle| vehicle.empty_bays.contains(&bay))
}

fn drop(backend: &dyn Backend) -> Value {
    let (armed, bay) = {
        let rail = rail();
        let now_ms = crate::hub::now_ms();
        (rail.armed(now_ms).value, rail.selected(now_ms).value)
    };
    if !armed {
        return refused(SAFE);
    }
    let bays = crate::hub::lock()
        .active()
        .map_or(0, crate::hub::Vehicle::drop_bays);
    let named = crate::radiocamera::named(bay, bays);
    crate::guided::gripper(
        backend,
        DROP_BAY,
        &json!([crate::guided::GRIPPER_RELEASE, named]).to_string(),
    )
}

fn drop_all(backend: &dyn Backend) -> Value {
    match armed() {
        false => refused(SAFE),
        true => crate::guided::dispatch(
            backend,
            Some(json!({ "action": "dropAll" })),
            crate::guided::active_id(backend),
            DROP_ALL,
            "[]",
        ),
    }
}

fn has_drop_board() -> bool {
    let now_ms = crate::hub::now_ms();
    crate::hub::lock()
        .active()
        .is_some_and(|vehicle| vehicle.drop_board_at(now_ms).is_some())
}

pub fn rail_json(
    bays: i64,
    empty: &[i64],
    armed: Owned<bool>,
    selected: Owned<i64>,
    drop_offer: Result<(), String>,
    shown: bool,
    drop_all: bool,
) -> Value {
    let drop_offered = drop_offer.is_ok();
    let loaded = |bay: i64| !empty.contains(&bay);
    let has_selected = (1..=bays).contains(&selected.value);
    let drop_ready = shown && armed.value && drop_offered && has_selected;
    let drop_label = match has_selected {
        true => format!("Drop {}", selected.value),
        false => format!("No bay {}", selected.value),
    };
    json!({
        "kind": "object",
        "class": "PayloadRail",
        "shown": shown,
        "armed": armed.value,
        "armedBy": armed.by,
        "selected": selected.value,
        "selectedBy": selected.by,
        "bays": (1..=bays).map(|bay| json!({ "bay": bay, "loaded": loaded(bay) })).collect::<Vec<_>>(),
        "dropReady": drop_ready,
        "dropAll": shown && drop_all,
        "dropAllReady": shown && drop_all && armed.value && drop_offered,
        "dropLabel": drop_label,
        "reason": match (shown, armed.value, has_selected, drop_offered) {
            (false, ..) => Value::Null,
            (_, false, ..) => json!(SAFE),
            (_, _, false, _) => json!(format!("This drone has {bays} bays.")),
            (_, _, _, false) => json!(drop_offer
                .as_ref()
                .err()
                .filter(|reason| !reason.is_empty())
                .map_or("Drop is not offered now.", String::as_str)),
            _ => Value::Null,
        },
    })
}

pub fn view(backend: &dyn Backend, _args: &[String]) -> Value {
    let guided = crate::guided::guided_view(backend, &[]);
    let payload = &guided["payload"];
    let bays = payload["bays"].as_i64().unwrap_or(1);
    let empty: Vec<i64> = payload["empty"]
        .as_array()
        .map(|bays| bays.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let drop_offer = guided["actions"]
        .as_array()
        .and_then(|offers| offers.iter().find(|offer| offer["id"] == "release"))
        .map_or(Err(String::new()), |offer| {
            match offer["offer"] == "ready" {
                true => Ok(()),
                false => Err(offer["reason"].as_str().unwrap_or_default().to_string()),
            }
        });
    let shown = shown();
    let drop_all = has_drop_board();
    let now_ms = crate::hub::now_ms();
    let rail = rail();
    rail_json(
        bays,
        &empty,
        rail.armed(now_ms),
        rail.selected(now_ms),
        drop_offer,
        shown,
        drop_all,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVE: RadioSwitches = RadioSwitches {
        armed: true,
        bay: Some(3),
        at_ms: 1_000,
    };

    #[test]
    fn drop_all_is_offered_only_for_a_drop_board_and_only_while_armed() {
        let armed = Owned {
            value: true,
            by: APP,
        };
        let safe = Owned {
            value: false,
            by: APP,
        };
        let on = |bay| Owned {
            value: bay,
            by: APP,
        };
        let board = rail_json(4, &[], armed, on(1), Ok(()), true, true);
        assert_eq!(
            (board["dropAll"].as_bool(), board["dropAllReady"].as_bool()),
            (Some(true), Some(true))
        );
        let locked = rail_json(4, &[], safe, on(1), Ok(()), true, true);
        assert_eq!(
            (
                locked["dropAll"].as_bool(),
                locked["dropAllReady"].as_bool()
            ),
            (Some(true), Some(false))
        );
        let gripper = rail_json(4, &[], armed, on(1), Ok(()), true, false);
        assert_eq!(
            gripper["dropAll"].as_bool(),
            Some(false),
            "a flight controller gripper has no drop all"
        );
    }

    #[test]
    fn a_radio_with_no_bay_switch_arms_and_leaves_the_bay_to_the_app() {
        let mut rail = Rail::default();
        assert_eq!(rail.select(2, 0), Ok(()));
        rail.heard(RadioSwitches { bay: None, ..LIVE });
        assert_eq!(
            rail.armed(1_100),
            Owned {
                value: true,
                by: RADIO
            }
        );
        assert_eq!(rail.selected(1_100), Owned { value: 2, by: APP });
        assert_eq!(rail.select(3, 1_100), Ok(()));
        assert_eq!(rail.arm(false, 1_100), Err(RADIO_ARMS));
    }

    #[test]
    fn the_radio_owns_the_switches_while_it_streams_and_the_app_takes_over_when_it_stops() {
        let mut rail = Rail::default();
        assert_eq!(
            rail.armed(0),
            Owned {
                value: false,
                by: APP
            }
        );
        assert_eq!(
            rail.selected(0),
            Owned { value: 1, by: APP },
            "the app starts on bay 1"
        );
        assert!(
            rail.heard(LIVE),
            "the first line from the radio is a change"
        );
        assert!(!rail.heard(LIVE), "the same switches again are not");
        assert_eq!(
            rail.armed(1_100),
            Owned {
                value: true,
                by: RADIO
            }
        );
        assert_eq!(
            rail.selected(1_100),
            Owned {
                value: 3,
                by: RADIO
            }
        );
        assert_eq!(rail.arm(false, 1_100), Err(RADIO_ARMS));
        assert_eq!(rail.select(2, 1_100), Err(RADIO_SELECTS));
        assert_eq!(
            rail.armed(1_000 + RADIO_STALE_MS + 1),
            Owned {
                value: false,
                by: APP
            },
            "half a second of silence hands the switches to the app"
        );
        assert_eq!(rail.arm(true, 2_000), Ok(()));
        assert_eq!(rail.select(2, 2_000), Ok(()));
        assert_eq!(
            rail.armed(2_000),
            Owned {
                value: true,
                by: APP
            }
        );
        assert_eq!(rail.selected(2_000), Owned { value: 2, by: APP });
    }

    #[test]
    fn the_rail_tells_the_radio_what_it_made_of_the_arm_switch() {
        let mut rail = Rail::default();
        assert_eq!(
            rail.verdict(true, 1_000),
            SWITCH_OFF,
            "a radio with no arm channel has nothing to arm with"
        );
        rail.seen(Some(RadioSwitches {
            armed: false,
            ..LIVE
        }));
        assert_eq!(rail.verdict(true, 1_000), SWITCH_SAFE);
        rail.seen(Some(LIVE));
        assert_eq!(
            rail.verdict(true, 1_000),
            SWITCH_REFUSED,
            "the switch is up but the rail never heard it: the radio is not flying the vehicle"
        );
        rail.heard(LIVE);
        assert_eq!(rail.verdict(true, 1_000), SWITCH_ARMED);
        assert_eq!(
            rail.verdict(false, 1_000),
            SWITCH_REFUSED,
            "with no payload the arm is not taken"
        );
        assert_eq!(
            rail.verdict(true, 1_000 + RADIO_STALE_MS + 1),
            SWITCH_REFUSED,
            "a stale arm is the app's, not the radio's"
        );
        rail.seen(None);
        assert_eq!(rail.verdict(true, 1_000), SWITCH_OFF);
    }

    #[test]
    fn the_rail_offers_drop_armed_on_any_bay_the_drone_has_and_shows_which_are_empty() {
        let armed = Owned {
            value: true,
            by: APP,
        };
        let safe = Owned {
            value: false,
            by: APP,
        };
        let on = |bay: i64| Owned {
            value: bay,
            by: RADIO,
        };
        let shown = rail_json(4, &[2], armed, on(1), Ok(()), true, false);
        assert_eq!(shown["dropReady"], true);
        assert_eq!(shown["dropLabel"], "Drop 1");
        assert_eq!(shown["bays"][1], json!({ "bay": 2, "loaded": false }));
        assert_eq!(shown["reason"], Value::Null);
        let reloaded_unseen = rail_json(4, &[2], armed, on(2), Ok(()), true, false);
        assert_eq!(
            (
                reloaded_unseen["dropReady"].clone(),
                reloaded_unseen["reason"].clone()
            ),
            (json!(true), Value::Null),
            "a bay counted empty still drops: the basket cannot say it was reloaded"
        );
        assert_eq!(
            rail_json(4, &[], safe, on(1), Ok(()), true, false)["reason"],
            SAFE
        );
        let past = rail_json(4, &[], armed, on(6), Ok(()), true, false);
        assert_eq!(
            (
                past["dropReady"].clone(),
                past["dropLabel"].clone(),
                past["reason"].clone()
            ),
            (
                json!(false),
                json!("No bay 6"),
                json!("This drone has 4 bays.")
            )
        );
        assert_eq!(
            rail_json(4, &[], armed, on(1), Err(String::new()), true, false)["reason"],
            "Drop is not offered now."
        );
        assert_eq!(
            rail_json(4, &[], armed, on(1), Err("No signal.".into()), true, false)["reason"],
            "No signal.",
            "the deck's own reason is shown when it has one"
        );
        let hidden = rail_json(1, &[], armed, on(1), Ok(()), false, false);
        assert_eq!(
            (
                hidden["shown"].clone(),
                hidden["dropReady"].clone(),
                hidden["reason"].clone()
            ),
            (json!(false), json!(false), Value::Null)
        );
    }

    #[test]
    fn the_rail_owns_its_three_actions() {
        assert!(owns(ARM_RAIL) && owns(SELECT_BAY) && owns(DROP_BAY) && !owns("payloadRail.other"));
    }
}
