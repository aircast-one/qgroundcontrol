use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::{flag, integer};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.instance",
    "vehicle.armed",
    "vehicle.flightMode",
    "vehicle.cameraTriggerPoints.count",
    "planFly.missionController.containsItems",
    "planFly.geoFenceController.containsItems",
    "planFly.rallyPointController.containsItems",
    "planFly.missionController.resumeMissionIndex",
    "planFly.missionController.currentMissionIndex",
    crate::coreplan::CHANGED,
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Latch {
    instance: Option<u64>,
    armed: Option<bool>,
    was_armed: bool,
    was_in_mission: bool,
    shown: u64,
    open: bool,
    reached: Progress,
    finished: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub waypoint: i64,
    pub of: i64,
}

pub struct Reading {
    pub instance: Option<u64>,
    pub connected: bool,
    pub armed: bool,
    pub in_mission: bool,
    pub has_plan: bool,
    pub progress: Progress,
}

impl Reading {
    fn finished(&self) -> bool {
        self.progress.of > 0 && self.progress.waypoint > self.progress.of
    }
}

pub fn step(held: Latch, now: &Reading) -> Latch {
    let latch = match (held.instance, now.instance) {
        (Some(was), Some(is)) if was != is => Latch {
            shown: held.shown,
            ..Latch::default()
        },
        _ => held,
    };
    let armed = !now.connected || now.armed;
    let in_mission = now.connected && now.in_mission;
    let flying_it = in_mission && (1..=now.progress.of).contains(&now.progress.waypoint);
    let base = Latch {
        instance: now.instance.or(latch.instance),
        armed: now.connected.then_some(now.armed),
        open: latch.open && now.connected,
        reached: if flying_it {
            now.progress
        } else {
            latch.reached
        },
        finished: !flying_it && (latch.finished || (now.connected && now.finished())),
        ..latch
    };
    match (latch.armed.unwrap_or(true) != armed, armed) {
        (true, true) => Latch {
            was_armed: true,
            was_in_mission: in_mission,
            reached: if flying_it {
                now.progress
            } else {
                Progress::default()
            },
            finished: !flying_it && now.finished(),
            ..base
        },
        (true, false) => {
            let show = latch.was_armed && latch.was_in_mission && now.has_plan;
            Latch {
                was_armed: false,
                was_in_mission: false,
                shown: latch.shown + u64::from(show),
                open: latch.open || show,
                ..base
            }
        }
        (false, true) => Latch {
            was_in_mission: latch.was_in_mission || in_mission,
            ..base
        },
        (false, false) => base,
    }
}

static LATCH: Mutex<Latch> = Mutex::new(Latch {
    instance: None,
    armed: None,
    was_armed: false,
    was_in_mission: false,
    shown: 0,
    open: false,
    reached: Progress { waypoint: 0, of: 0 },
    finished: false,
});

fn contains(backend: &dyn Backend, controller: &str) -> bool {
    flag(
        &backend.value_fields(&format!("planFly.{controller}"), "containsItems"),
        "containsItems",
    )
}

fn reading(backend: &dyn Backend) -> (Reading, i64) {
    let vehicle = backend.value_fields("vehicle", "instance,armed,flightMode,missionFlightMode");
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let in_mission = vehicle.get("flightMode").is_some()
        && vehicle.get("flightMode") == vehicle.get("missionFlightMode");
    let images = integer(
        &backend.value_fields("vehicle.cameraTriggerPoints", "count"),
        "count",
    )
    .unwrap_or(0);
    let mission = match crate::coreplan::plan_state() {
        Some(plan) => plan.has_mission_items,
        None => contains(backend, "missionController"),
    };
    let has_plan = mission
        || contains(backend, "geoFenceController")
        || contains(backend, "rallyPointController")
        || images != 0;
    let progress = Progress {
        waypoint: integer(
            &backend.value_fields("planFly.missionController", "currentMissionIndex"),
            "currentMissionIndex",
        )
        .unwrap_or(-1),
        of: crate::missionprogress::last_sequence(backend),
    };
    (
        Reading {
            instance: vehicle.get("instance").and_then(Value::as_u64),
            connected,
            armed: flag(&vehicle, "armed"),
            in_mission,
            has_plan,
            progress,
        },
        images,
    )
}

pub fn mission_complete_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let (now, images) = reading(backend);
    let latch = {
        let mut held = LATCH.lock().unwrap_or_else(PoisonError::into_inner);
        *held = step(*held, &now);
        *held
    };
    let resume = latch
        .open
        .then(|| crate::guided::resume_offer(backend))
        .flatten();
    let summary = resume.and_then(resume_summary).map(|(first, drops)| {
        (
            resume_place(first, crate::coreplan::flown_row(first as i64).as_ref()),
            drops,
        )
    });
    let interrupted = resume.is_some();
    json!({
        "kind": "object",
        "class": "MissionComplete",
        "open": latch.open,
        "id": latch.shown,
        "imagesTaken": images,
        "resumeFromWaypoint": resume,
        "batteryWarning": resume.is_some(),
        "interrupted": interrupted,
        "title": title(
            latch.finished,
            latch.reached,
            crate::coreplan::flown_row(latch.reached.waypoint).as_ref()
        ),
        "detail": summary.as_ref().map(|(place, drops)| interrupted_detail(place, *drops)).unwrap_or_else(|| stopped_detail(latch.finished).to_string()),
        "resumeLabel": summary.as_ref().map(|(place, _)| resume_label(place)).or_else(|| resume.map(|at| format!("Resume mission from waypoint {at}"))),
    })
}

const COMPLETE_TITLE: &str = "Flight plan complete";
const STOPPED_TITLE: &str = "Mission stopped";
const STOPPED_DETAIL: &str = "The aircraft left the mission before finishing it.";

pub fn title(finished: bool, reached: Progress, row: Option<&crate::coreplan::FlownRow>) -> String {
    match (finished, reached.waypoint > 0, row) {
        (true, ..) => COMPLETE_TITLE.to_string(),
        (false, true, Some(row)) => format!("{STOPPED_TITLE} at {}", row.place),
        (false, true, None) => format!(
            "{STOPPED_TITLE} at mission item {} of {}",
            reached.waypoint, reached.of
        ),
        (false, false, _) => STOPPED_TITLE.to_string(),
    }
}

fn stopped_detail(finished: bool) -> &'static str {
    match finished {
        true => "",
        false => STOPPED_DETAIL,
    }
}

pub fn resume_place(first: usize, row: Option<&crate::coreplan::FlownRow>) -> String {
    match row {
        Some(row) => {
            let inside = row.within.map_or(String::new(), |(at, of)| {
                format!(", at its waypoint {at} of {of}")
            });
            format!("{}{inside}", row.place)
        }
        None => format!("mission item {first}"),
    }
}

pub fn resume_label(place: &str) -> String {
    format!("Resume from {place}")
}

pub fn interrupted_detail(place: &str, drops: usize) -> String {
    let rest = match drops {
        0 => String::new(),
        1 => " One drop is still to come.".to_string(),
        n => format!(" {n} drops are still to come."),
    };
    format!("{STOPPED_DETAIL} Resuming flies it again from {place}.{rest}")
}

fn resume_summary(offer: i64) -> Option<(usize, usize)> {
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    let shift = i64::from(!vehicle.sends_home());
    let asked = usize::try_from(offer - shift).ok()?;
    let commands = crate::cmdinfo::tree(
        crate::plandoc::firmware(i64::from(vehicle.autopilot)),
        crate::plandoc::vehicle_class(i64::from(vehicle.vehicle_type)),
    );
    let (first, drops) = crate::resumemission::resume_summary(
        vehicle.mission_items(),
        asked,
        |command| crate::resumemission::shape(&commands, command),
        &vehicle.payload_servos(),
    );
    Some((first + usize::try_from(shift).unwrap_or(0), drops))
}

pub fn dismiss(id: u64) -> Value {
    let mut held = LATCH.lock().unwrap_or_else(PoisonError::into_inner);
    match held.open && held.shown == id {
        true => {
            held.open = false;
            json!({ "ok": true })
        }
        false => {
            json!({ "ok": false, "reason": "That mission complete notice is no longer open." })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interrupted_mission_says_where_it_resumes_and_what_is_left() {
        assert_eq!(
            interrupted_detail(&resume_place(4, None), 2),
            "The aircraft left the mission before finishing it. Resuming flies it again from mission item 4. 2 drops are still to come."
        );
        assert_eq!(
            interrupted_detail(&resume_place(4, None), 1),
            "The aircraft left the mission before finishing it. Resuming flies it again from mission item 4. One drop is still to come."
        );
        assert_eq!(
            interrupted_detail(&resume_place(6, None), 0),
            "The aircraft left the mission before finishing it. Resuming flies it again from mission item 6."
        );
    }

    #[test]
    fn a_resume_names_the_plan_row_and_where_inside_a_pattern_it_starts() {
        let survey = crate::coreplan::FlownRow {
            place: "survey 2 of 3".to_string(),
            arrived: false,
            within: Some((14, 40)),
        };
        assert_eq!(
            resume_label(&resume_place(49, Some(&survey))),
            "Resume from survey 2 of 3, at its waypoint 14 of 40",
            "the title names the survey as stop 2 of 3, so the button must not fall back to the vehicle's command 49"
        );
        let waypoint = crate::coreplan::FlownRow {
            place: "waypoint 3 of 5".to_string(),
            arrived: false,
            within: None,
        };
        assert_eq!(
            resume_label(&resume_place(4, Some(&waypoint))),
            "Resume from waypoint 3 of 5"
        );
        assert_eq!(
            interrupted_detail(&resume_place(49, Some(&survey)), 0),
            "The aircraft left the mission before finishing it. Resuming flies it again from survey 2 of 3, at its waypoint 14 of 40."
        );
        assert_eq!(
            resume_label(&resume_place(4, None)),
            "Resume from mission item 4",
            "with no plan rows to read, the vehicle's own number is all there is"
        );
    }

    fn at(armed: bool, in_mission: bool) -> Reading {
        Reading {
            instance: Some(1),
            connected: true,
            armed,
            in_mission,
            has_plan: true,
            progress: Progress { waypoint: 3, of: 6 },
        }
    }

    fn heading(waypoint: i64, in_mission: bool) -> Reading {
        Reading {
            progress: Progress { waypoint, of: 140 },
            ..at(true, in_mission)
        }
    }

    #[test]
    fn a_mission_left_by_a_return_says_where_it_stopped_instead_of_complete() {
        let returned = run(&[
            at(false, false),
            heading(48, true),
            heading(49, true),
            heading(49, false),
            Reading {
                progress: Progress {
                    waypoint: -1,
                    of: 140,
                },
                ..at(false, false)
            },
        ]);
        assert!(returned.open);
        assert_eq!(
            title(returned.finished, returned.reached, None),
            "Mission stopped at mission item 49 of 140",
            "a plan sent again while returning clears the vehicle's index, so the latch keeps the last waypoint flown"
        );
        let finished = run(&[
            at(false, false),
            heading(139, true),
            heading(140, true),
            heading(141, false),
            Reading {
                progress: Progress {
                    waypoint: 141,
                    of: 140,
                },
                ..at(false, false)
            },
        ]);
        assert_eq!(
            title(finished.finished, finished.reached, None),
            "Flight plan complete",
            "the core reads an index past the last sequence once MISSION_CURRENT says MISSION_STATE_COMPLETE"
        );
        assert_eq!(
            title(false, Progress::default(), None),
            "Mission stopped",
            "with no waypoint seen there is nothing to name"
        );
        let next = run(&[
            at(false, false),
            heading(140, true),
            heading(141, false),
            at(false, false),
            Reading {
                progress: Progress {
                    waypoint: 141,
                    of: 140,
                },
                ..at(true, false)
            },
            heading(5, true),
            heading(5, false),
            at(false, false),
        ]);
        assert_eq!(
            title(next.finished, next.reached, None),
            "Mission stopped at mission item 5 of 140",
            "ArduPilot still says complete when it arms again, and the mission flown after that is a new one"
        );
        let survey = crate::coreplan::FlownRow {
            place: "survey 2 of 3".to_string(),
            arrived: false,
            within: None,
        };
        assert_eq!(
            title(returned.finished, returned.reached, Some(&survey)),
            "Mission stopped at survey 2 of 3",
            "a plan row is what the pilot sees, so command 49 of 140 inside a survey reads as the survey, row 2 of 3"
        );
    }

    fn run(readings: &[Reading]) -> Latch {
        readings.iter().fold(Latch::default(), step)
    }

    #[test]
    fn opens_on_disarm_after_an_armed_mission() {
        assert!(
            run(&[
                at(false, false),
                at(true, false),
                at(true, true),
                at(false, false)
            ])
            .open,
            "armed, then Auto, then disarmed is a flown plan"
        );
        assert!(run(&[at(false, false), at(true, true), at(false, true)]).open);
        assert!(
            !run(&[at(false, false), at(true, false), at(false, false)]).open,
            "a flight that never entered the mission mode finished nothing"
        );
        assert!(
            !run(&[
                at(false, false),
                at(true, true),
                at(true, true),
                at(true, true)
            ])
            .open,
            "still armed is still flying"
        );
        let no_plan = [
            at(false, false),
            at(true, true),
            Reading {
                has_plan: false,
                ..at(false, false)
            },
        ];
        assert!(
            !run(&no_plan).open,
            "with no plan, fence, rally or photos there is nothing to report"
        );
        assert!(
            !run(&[
                at(false, false),
                at(true, true),
                Reading {
                    connected: false,
                    ..at(false, false)
                }
            ])
            .open,
            "losing the vehicle closes it, as the dialog does when activeVehicle goes"
        );
        assert!(
            !run(&[at(true, true), at(false, true)]).open,
            "a vehicle first seen already armed never armed under QGC's eyes, since no vehicle reads as armed"
        );
        let gone = Reading {
            connected: false,
            ..at(false, false)
        };
        assert!(
            run(&[at(false, false), at(true, true), gone, at(false, false)]).open,
            "a link lost in flight that returns disarmed is the finished mission QGC reports"
        );
        let another = Reading {
            instance: Some(2),
            ..at(false, false)
        };
        assert!(
            !run(&[at(false, false), at(true, true), another]).open,
            "another aircraft taking the lost one's id did not finish the lost one's mission"
        );
    }

    #[test]
    fn repeated_readings_do_not_retrigger() {
        let opened = run(&[
            at(false, false),
            at(true, true),
            at(false, false),
            at(false, false),
            at(false, false),
        ]);
        assert_eq!(opened.shown, 1);
    }

    #[test]
    fn dismissing_needs_the_open_id() {
        *LATCH.lock().unwrap() = Latch {
            open: true,
            shown: 4,
            ..Latch::default()
        };
        assert_eq!(dismiss(3)["ok"], false);
        assert_eq!(dismiss(4)["ok"], true);
        assert!(!LATCH.lock().unwrap().open);
    }
}
