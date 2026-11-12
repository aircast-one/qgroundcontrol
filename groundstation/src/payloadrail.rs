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
pub const RADIO_SELECTS: &str = "The radio's 6POS switch picks the bay while it streams.";
pub const NO_PAYLOAD: &str = "This drone has nothing to drop.";
pub const RADIO_STALE_MS: u64 = crate::usbradio::STALE_AFTER_MS;
pub const RADIO: &str = "radio";
pub const APP: &str = "app";
pub const SWITCH_ARMED: &str = "armed";
pub const SWITCH_SAFE: &str = "safe";
pub const SWITCH_REFUSED: &str = "refused";
pub const SWITCH_OFF: &str = "off";
pub const ARMED_PULSE: u16 = 2012;
pub const SAFE_PULSE: u16 = 988;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioSwitches {
    pub armed: bool,
    pub bay: Option<i64>,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Radio(RadioSwitches),
    Seen(Option<RadioSwitches>),
    Arm(bool),
    Pick { bay: i64, reloaded: bool },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

    pub fn armed(&self) -> bool {
        self.armed
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

    pub fn apply(self, event: Event, now_ms: u64) -> (Rail, Result<bool, &'static str>) {
        match event {
            Event::Radio(switches) => (
                Rail {
                    armed: match self.radio {
                        Some(was) if was.armed != switches.armed => switches.armed,
                        _ => self.armed,
                    },
                    radio: Some(switches),
                    ..self
                },
                Ok(self
                    .radio
                    .is_none_or(|was| (was.armed, was.bay) != (switches.armed, switches.bay))),
            ),
            Event::Seen(switches) => (
                Rail {
                    seen: switches,
                    ..self
                },
                Ok(false),
            ),
            Event::Arm(armed) => (Rail { armed, ..self }, Ok(true)),
            Event::Pick { bay, reloaded } => {
                match (self.radio(now_ms).and_then(|radio| radio.bay), reloaded) {
                    (Some(_), true) => (self, Ok(true)),
                    (Some(_), false) => (self, Err(RADIO_SELECTS)),
                    (None, _) => (
                        Rail {
                            selected: bay.max(1),
                            ..self
                        },
                        Ok(true),
                    ),
                }
            }
        }
    }

    pub fn drop_bay(&self, bay: i64, bays: Option<i64>) -> Result<i64, String> {
        match (self.armed, bays) {
            (false, _) => Err(SAFE.to_string()),
            (true, None) => Err(NO_PAYLOAD.to_string()),
            (true, Some(bays)) => named(bay, bays).ok_or_else(|| bay_count(bays)),
        }
    }

    pub fn verdict(&self, has_payload: bool) -> &'static str {
        match (self.seen, self.armed && has_payload) {
            (None, _) => SWITCH_OFF,
            (Some(_), true) => SWITCH_ARMED,
            (Some(RadioSwitches { armed: true, .. }), false) => SWITCH_REFUSED,
            (Some(_), false) => SWITCH_SAFE,
        }
    }

    pub fn reloads(&self, empty: bool) -> bool {
        empty && !self.armed
    }
}

fn named(bay: i64, bays: i64) -> Option<i64> {
    match bays {
        0 => Some(0),
        _ => (1..=bays).contains(&bay).then_some(bay),
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

fn apply(event: Event) -> Result<bool, &'static str> {
    let mut rail = rail();
    let (next, outcome) = rail.apply(event, crate::hub::now_ms());
    *rail = next;
    outcome
}

pub fn radio_heard(switches: RadioSwitches) {
    if apply(Event::Radio(switches)) == Ok(true) {
        log::info!(target: "radio", "BOMARM {} and the 6POS on bay {:?}", if switches.armed { "armed" } else { "safe" }, switches.bay);
        CHANGED_FLAG.store(true, Ordering::Relaxed);
    }
}

pub fn radio_seen(switches: Option<RadioSwitches>) {
    let _ = apply(Event::Seen(switches));
}

pub fn verdict() -> &'static str {
    let has_payload = shown();
    rail().verdict(has_payload)
}

fn shown() -> bool {
    crate::hub::lock().active().is_some_and(|vehicle| {
        vehicle.payload(crate::hub::now_ms()) != crate::payload::Payload::None
    })
}

pub fn take_changed() -> bool {
    CHANGED_FLAG.swap(false, Ordering::Relaxed)
}

pub fn armed() -> bool {
    rail().armed()
}

pub fn pulse(armed: bool) -> u16 {
    match armed {
        true => ARMED_PULSE,
        false => SAFE_PULSE,
    }
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

fn settled(outcome: Result<bool, &'static str>) -> Value {
    CHANGED_FLAG.store(true, Ordering::Relaxed);
    match outcome {
        Ok(_) => json!({ "ok": true }),
        Err(reason) => refused(reason),
    }
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    match path {
        ARM_RAIL => settled(apply(Event::Arm(
            given.first().and_then(Value::as_bool).unwrap_or(false),
        ))),
        SELECT_BAY => match given
            .first()
            .and_then(Value::as_i64)
            .filter(|bay| *bay >= 1)
        {
            Some(bay) => {
                let reloaded = rail().reloads(empty(bay))
                    && crate::guided::dispatch(
                        backend,
                        Some(json!({ "action": "bayLoaded", "bay": bay })),
                        crate::guided::active_id(backend),
                        SELECT_BAY,
                        "[]",
                    )["ok"]
                        == json!(true);
                settled(apply(Event::Pick { bay, reloaded }))
            }
            None => refused("Name the bay to pick, from 1."),
        },
        DROP_BAY => drop(backend),
        DROP_ALL => drop_all(backend),
        _ => refused("Unknown payload rail action."),
    }
}

fn bay_count(bays: i64) -> String {
    format!(
        "This drone has {bays} {}.",
        if bays == 1 { "bay" } else { "bays" }
    )
}

fn empty(bay: i64) -> bool {
    crate::hub::lock()
        .active()
        .is_some_and(|vehicle| vehicle.empty_bays.contains(&bay))
}

pub fn drop_bay(bay: i64) -> Result<i64, String> {
    let now_ms = crate::hub::now_ms();
    let bays = crate::hub::lock().active().and_then(|vehicle| {
        let payload = vehicle.payload(now_ms);
        (payload != crate::payload::Payload::None).then(|| payload.bays())
    });
    rail().drop_bay(bay, bays)
}

fn drop(backend: &dyn Backend) -> Value {
    match drop_bay(selected()) {
        Ok(bay) => crate::guided::gripper(
            backend,
            DROP_BAY,
            &json!([crate::guided::GRIPPER_RELEASE, bay]).to_string(),
        ),
        Err(reason) => refused(&reason),
    }
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
        .is_some_and(|vehicle| vehicle.payload(now_ms).drops_all())
}

pub fn rail_json(
    bays: i64,
    empty: &[i64],
    armed: bool,
    selected: Owned<i64>,
    drop_offer: Result<(), String>,
    shown: bool,
    drop_all: bool,
) -> Value {
    let drop_offered = drop_offer.is_ok();
    let loaded = |bay: i64| !empty.contains(&bay);
    let has_selected = (1..=bays).contains(&selected.value);
    let drop_ready = shown && armed && drop_offered && has_selected;
    let drop_label = match has_selected {
        true => format!("Drop {}", selected.value),
        false => format!("No bay {}", selected.value),
    };
    json!({
        "kind": "object",
        "class": "PayloadRail",
        "shown": shown,
        "armed": armed,
        "selected": selected.value,
        "selectedBy": selected.by,
        "bays": (1..=bays).map(|bay| json!({ "bay": bay, "loaded": loaded(bay) })).collect::<Vec<_>>(),
        "dropReady": drop_ready,
        "dropAll": shown && drop_all,
        "dropAllReady": shown && drop_all && armed && drop_offered,
        "dropLabel": drop_label,
        "reason": match (shown, armed, has_selected, drop_offered) {
            (false, ..) => Value::Null,
            (_, false, ..) => json!(SAFE),
            (_, _, false, _) => json!(bay_count(bays)),
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
        rail.armed(),
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

    fn switch(armed: bool, at_ms: u64) -> RadioSwitches {
        RadioSwitches {
            armed,
            bay: None,
            at_ms,
        }
    }

    #[test]
    fn drop_all_is_offered_only_for_a_drop_board_and_only_while_armed() {
        let on = |bay| Owned {
            value: bay,
            by: APP,
        };
        let board = rail_json(4, &[], true, on(1), Ok(()), true, true);
        assert_eq!(
            (board["dropAll"].as_bool(), board["dropAllReady"].as_bool()),
            (Some(true), Some(true))
        );
        let locked = rail_json(4, &[], false, on(1), Ok(()), true, true);
        assert_eq!(
            (
                locked["dropAll"].as_bool(),
                locked["dropAllReady"].as_bool()
            ),
            (Some(true), Some(false))
        );
        let gripper = rail_json(4, &[], true, on(1), Ok(()), true, false);
        assert_eq!(
            gripper["dropAll"].as_bool(),
            Some(false),
            "a flight controller gripper has no drop all"
        );
    }

    fn heard(rail: Rail, armed: bool, at_ms: u64) -> Rail {
        rail.apply(Event::Radio(switch(armed, at_ms)), at_ms).0
    }

    fn tapped(rail: Rail, armed: bool) -> Rail {
        rail.apply(Event::Arm(armed), 0).0
    }

    #[test]
    fn the_last_change_arms_or_safes_whether_it_came_from_the_radio_or_the_app() {
        let rail = heard(Rail::default(), true, 0);
        assert!(
            !rail.armed(),
            "a radio plugged in with its switch up arms nothing until the switch moves"
        );
        let rail = heard(heard(rail, false, 100), true, 200);
        assert!(rail.armed(), "flipping the switch up arms");
        let rail = heard(tapped(rail, false), true, 300);
        assert!(
            !rail.armed(),
            "a tap makes it safe, and the switch left up does not undo it"
        );
        let rail = heard(heard(tapped(rail, true), true, 400), false, 500);
        assert!(!rail.armed(), "flipping the switch down makes it safe");
        let rail = heard(tapped(rail, true), false, 600);
        assert!(rail.armed(), "a tap arms while the switch stays down");
        assert!(
            Rail {
                radio: None,
                ..rail
            }
            .armed(),
            "the arm outlives the radio going quiet"
        );
    }

    #[test]
    fn a_radio_with_no_bay_switch_leaves_the_bay_to_the_app() {
        let pick = |bay| Event::Pick {
            bay,
            reloaded: false,
        };
        let (rail, picked) = Rail::default().apply(pick(2), 0);
        assert_eq!(picked, Ok(true));
        let rail = rail
            .apply(Event::Radio(RadioSwitches { bay: None, ..LIVE }), 1_000)
            .0;
        assert_eq!(rail.selected(1_100), Owned { value: 2, by: APP });
        assert_eq!(rail.apply(pick(3), 1_100).1, Ok(true));
    }

    #[test]
    fn the_radio_picks_the_bay_while_it_streams_and_the_app_takes_over_when_it_stops() {
        let pick = Event::Pick {
            bay: 2,
            reloaded: false,
        };
        assert_eq!(
            Rail::default().selected(0),
            Owned { value: 1, by: APP },
            "the app starts on bay 1"
        );
        let (rail, first) = Rail::default().apply(Event::Radio(LIVE), 1_000);
        assert_eq!(first, Ok(true), "the first line from the radio is a change");
        let (rail, again) = rail.apply(Event::Radio(LIVE), 1_000);
        assert_eq!(again, Ok(false), "the same switches again are not");
        assert_eq!(
            rail.selected(1_100),
            Owned {
                value: 3,
                by: RADIO
            }
        );
        assert_eq!(rail.apply(pick, 1_100).1, Err(RADIO_SELECTS));
        let quiet = 1_000 + RADIO_STALE_MS + 1;
        let (rail, picked) = rail.apply(pick, quiet);
        assert_eq!(
            picked,
            Ok(true),
            "half a second of silence hands the bay to the app"
        );
        assert_eq!(rail.selected(quiet), Owned { value: 2, by: APP });
    }

    #[test]
    fn the_radio_is_told_the_payload_state_and_when_its_switch_disagrees() {
        let seen = |rail: Rail, switches| rail.apply(Event::Seen(switches), 0).0;
        let rail = Rail::default();
        assert_eq!(
            rail.verdict(true),
            SWITCH_OFF,
            "a radio with no arm channel has nothing to arm with"
        );
        let rail = seen(
            rail,
            Some(RadioSwitches {
                armed: false,
                ..LIVE
            }),
        );
        assert_eq!(rail.verdict(true), SWITCH_SAFE);
        let rail = tapped(rail, true);
        assert_eq!(
            rail.verdict(true),
            SWITCH_ARMED,
            "armed from the app, the radio shows armed with its switch down"
        );
        assert_eq!(
            rail.verdict(false),
            SWITCH_SAFE,
            "with no payload nothing is armed"
        );
        let rail = tapped(seen(rail, Some(LIVE)), false);
        assert_eq!(
            rail.verdict(true),
            SWITCH_REFUSED,
            "the switch is up but the payload is safe"
        );
        assert_eq!(seen(rail, None).verdict(true), SWITCH_OFF);
    }

    #[test]
    fn tapping_an_empty_bay_marks_it_loaded_even_while_the_radio_picks_the_bay() {
        let pick = |bay, reloaded| Event::Pick { bay, reloaded };
        let rail = Rail::default()
            .apply(
                Event::Radio(RadioSwitches {
                    armed: true,
                    bay: Some(3),
                    at_ms: 0,
                }),
                0,
            )
            .0;
        let (rail, reload) = rail.apply(pick(2, true), 10);
        assert_eq!(
            reload,
            Ok(true),
            "the reload is taken although the 6POS keeps the pick"
        );
        assert_eq!(rail.selected(10).value, 3);
        assert_eq!(
            rail.apply(pick(2, false), 10).1,
            Err(RADIO_SELECTS),
            "a loaded bay cannot be picked from the strip while the radio picks"
        );
        let (app, picked) = Rail::default().apply(pick(2, true), 10);
        assert_eq!(picked, Ok(true));
        assert_eq!(app.selected(10), Owned { value: 2, by: APP });
    }

    #[test]
    fn an_empty_bay_is_marked_reloaded_only_while_the_payload_is_safe() {
        let rail = Rail::default();
        assert!(
            rail.reloads(true),
            "on the ground, safe, a tap says the clip is back"
        );
        assert!(!rail.reloads(false), "a loaded bay has nothing to reload");
        assert!(
            !tapped(rail, true).reloads(true),
            "armed, a tap on a dropped bay only picks it: nobody reloads in flight"
        );
        assert_eq!(bay_count(1), "This drone has 1 bay.");
        assert_eq!(bay_count(4), "This drone has 4 bays.");
    }

    #[test]
    fn the_rail_offers_drop_armed_on_any_bay_the_drone_has_and_shows_which_are_empty() {
        let on = |bay: i64| Owned {
            value: bay,
            by: RADIO,
        };
        let shown = rail_json(4, &[2], true, on(1), Ok(()), true, false);
        assert_eq!(shown["dropReady"], true);
        assert_eq!(shown["dropLabel"], "Drop 1");
        assert_eq!(shown["bays"][1], json!({ "bay": 2, "loaded": false }));
        assert_eq!(shown["reason"], Value::Null);
        let reloaded_unseen = rail_json(4, &[2], true, on(2), Ok(()), true, false);
        assert_eq!(
            (
                reloaded_unseen["dropReady"].clone(),
                reloaded_unseen["reason"].clone()
            ),
            (json!(true), Value::Null),
            "a bay counted empty still drops: the basket cannot say it was reloaded"
        );
        assert_eq!(
            rail_json(4, &[], false, on(1), Ok(()), true, false)["reason"],
            SAFE
        );
        let past = rail_json(4, &[], true, on(6), Ok(()), true, false);
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
            rail_json(4, &[], true, on(1), Err(String::new()), true, false)["reason"],
            "Drop is not offered now."
        );
        assert_eq!(
            rail_json(4, &[], true, on(1), Err("No signal.".into()), true, false)["reason"],
            "No signal.",
            "the deck's own reason is shown when it has one"
        );
        let hidden = rail_json(1, &[], true, on(1), Ok(()), false, false);
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
    fn the_radio_and_the_app_drop_by_the_same_rule() {
        let armed = tapped(Rail::default(), true);
        assert_eq!(
            Rail::default().drop_bay(1, Some(4)),
            Err(SAFE.to_string()),
            "safe drops nothing, whoever asks"
        );
        assert_eq!(armed.drop_bay(3, Some(4)), Ok(3));
        assert_eq!(
            armed.drop_bay(5, Some(4)),
            Err(bay_count(4)),
            "6POS position 5 on a four-holder basket drops nothing, not the next holder"
        );
        assert_eq!(
            armed.drop_bay(5, Some(0)),
            Ok(0),
            "one gripper drops whatever the 6POS says"
        );
        assert_eq!(
            armed.drop_bay(1, None),
            Err(NO_PAYLOAD.to_string()),
            "a drone with no payload drops nothing"
        );
    }

    #[test]
    fn the_rail_owns_its_three_actions() {
        assert!(owns(ARM_RAIL) && owns(SELECT_BAY) && owns(DROP_BAY) && !owns("payloadRail.other"));
    }
}
