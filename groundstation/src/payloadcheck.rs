use std::ops::RangeInclusive;

use serde_json::{Value, json};

use crate::plantransfer::Item;

pub const CMD_DO_GRIPPER: u16 = 211;
pub const CMD_DO_SET_SERVO: u16 = 183;
const GRIPPER_RELEASE: f64 = 0.0;
const GRIPPER_GRAB: f64 = 1.0;
const GRIPPER_FUNCTION: f64 = 28.0;
const SERVO_OUTPUTS: usize = 32;
const GRIPPER_TYPE_NONE: f64 = 0.0;
const GRIPPER_TYPE_SERVO: f64 = 1.0;
pub const AUTO_CLOSE_SECONDS: f64 = 2.0;
pub const DROP_HOLD_SECONDS: f64 = 2.0;
const CONTINUE_AUTO_ON_GCS_FAILSAFE: i64 = 2;
const RELEASE_GRIPPER_ON_FAILSAFE: i64 = 32;
const DEFAULT_GCS_TIMEOUT_SECONDS: f64 = 5.0;
pub const SERVO_PER_BAY: f64 = 1.0;
pub const MAX_BAYS: i64 = 6;
const DEFAULT_RELEASE_PWM: f64 = 1900.0;
const DEFAULT_HOLD_PWM: f64 = 1100.0;
const FREE_FUNCTION: f64 = 0.0;
const SETTINGS: &str = "settings.flyViewSettings";
const SET_IN_SETTINGS: &str = "Set one in Settings, Fly View, Payload.";
const SERVO_SET_FUNCTIONS: [RangeInclusive<i64>; 5] = [0..=1, 22..=23, 28..=28, 51..=66, 140..=155];
const ACCESSORY_FUNCTIONS: [RangeInclusive<i64>; 6] =
    [5..=15, 88..=88, 90..=109, 120..=123, 129..=136, 138..=139];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Servo {
    pub output: i64,
    pub release: f64,
    pub hold: f64,
}

#[cfg(test)]
thread_local! {
    pub static CONFIGURED: std::cell::RefCell<Option<Vec<Servo>>> = const { std::cell::RefCell::new(None) };
}

fn setting(name: &str) -> Option<f64> {
    crate::settingsstore::raw_setting(&format!("{SETTINGS}.{name}")).and_then(|v| v.as_f64())
}

pub fn servo_bays(ardupilot: bool) -> Vec<Servo> {
    #[cfg(test)]
    if let Some(servos) = CONFIGURED.with_borrow(Clone::clone) {
        return servos.into_iter().filter(|_| ardupilot).collect();
    }
    let bays = setting("payloadBays")
        .map_or(1, |bays| bays as i64)
        .clamp(1, MAX_BAYS);
    match ardupilot && setting("payloadRelease") == Some(SERVO_PER_BAY) {
        false => Vec::new(),
        true => (1..=bays)
            .map(|bay| Servo {
                output: setting(&format!("payloadBay{bay}Output")).map_or(0, |o| o as i64),
                release: setting(&format!("payloadBay{bay}ReleasePwm"))
                    .unwrap_or(DEFAULT_RELEASE_PWM),
                hold: setting(&format!("payloadBay{bay}HoldPwm")).unwrap_or(DEFAULT_HOLD_PWM),
            })
            .collect(),
    }
}

pub fn seed_pwm_defaults(parameter: impl Fn(&str) -> Option<f64>) {
    let seeds: Vec<(String, Value)> = [("GRIP_RELEASE", "ReleasePwm"), ("GRIP_GRAB", "HoldPwm")]
        .into_iter()
        .filter_map(|(name, suffix)| parameter(name).map(|pwm| (suffix, pwm)))
        .flat_map(|(suffix, pwm)| {
            (1..=MAX_BAYS)
                .map(move |bay| (format!("{SETTINGS}.payloadBay{bay}{suffix}"), json!(pwm)))
        })
        .filter(|(path, _)| !crate::settingsstore::is_stored(path))
        .collect();
    crate::settingsstore::set_raw_together(
        &seeds
            .iter()
            .map(|(path, pwm)| (path.as_str(), pwm.clone()))
            .collect::<Vec<_>>(),
    );
}

pub fn servo_drop_bay(command: i64, output: f64, pwm: f64, servos: &[Servo]) -> Option<i64> {
    (command == i64::from(CMD_DO_SET_SERVO)).then_some(())?;
    servos
        .iter()
        .position(|servo| servo.output as f64 == output && servo.release == pwm)
        .map(|at| at as i64 + 1)
}

pub fn drops(command: i64, first: f64, second: f64, servos: &[Servo]) -> bool {
    (command == i64::from(CMD_DO_GRIPPER) && second == GRIPPER_RELEASE)
        || servo_drop_bay(command, first, second, servos).is_some()
}

pub fn is_release(item: &Item, servos: &[Servo]) -> bool {
    drops(
        i64::from(item.command),
        item.params[0],
        item.params[1],
        servos,
    )
}

pub fn bay_of(item: &Item, servos: &[Servo]) -> Option<i64> {
    servo_drop_bay(
        i64::from(item.command),
        item.params[0],
        item.params[1],
        servos,
    )
}

pub fn drops_in(items: &[Item], servos: &[Servo]) -> usize {
    items.iter().filter(|item| is_release(item, servos)).count()
}

const PLACES: [u16; 6] = [16, 17, 18, 19, 31, 82];

pub fn early_drops_in(items: &[Item], sends_home: bool, servos: &[Servo]) -> usize {
    items
        .iter()
        .skip(usize::from(sends_home))
        .take_while(|item| !PLACES.contains(&item.command))
        .filter(|item| is_release(item, servos))
        .count()
}

pub fn servo_command(servos: &[Servo], action: f64, bay: i64) -> Result<(f64, f64), String> {
    let servo = usize::try_from(bay - 1)
        .ok()
        .and_then(|at| servos.get(at))
        .ok_or_else(|| {
            format!(
                "There is no bay {bay}. This drone has {} bays.",
                servos.len()
            )
        })?;
    let pwm = match action {
        GRIPPER_RELEASE => servo.release,
        GRIPPER_GRAB => servo.hold,
        _ => return Err("A bay with its own servo only drops and grabs.".to_string()),
    };
    match servo.output > 0 {
        true => Ok((servo.output as f64, pwm)),
        false => Err(format!("Bay {bay} has no servo output. {SET_IN_SETTINGS}")),
    }
}

pub fn takes_set_servo(function: i64) -> bool {
    SERVO_SET_FUNCTIONS
        .iter()
        .any(|range| range.contains(&function))
}

fn freeable(function: i64) -> bool {
    ACCESSORY_FUNCTIONS
        .iter()
        .any(|range| range.contains(&function))
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ServoBay {
    pub output: i64,
    pub function: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Setup {
    pub ardupilot: bool,
    pub enabled: bool,
    pub loaded: bool,
    pub typed: bool,
    pub stale: bool,
    pub output: bool,
    pub auto_close_s: f64,
    pub fs_options: i64,
    pub gcs_failsafe: bool,
    pub gcs_timeout_s: f64,
    pub drops: usize,
    pub early_drops: usize,
    pub servos: Vec<ServoBay>,
}

impl Setup {
    pub fn read(
        ardupilot: bool,
        enabled: bool,
        parameter: impl Fn(&str) -> Option<f64>,
        servos: &[Servo],
        drops: usize,
        early_drops: usize,
    ) -> Setup {
        let servo = parameter("GRIP_TYPE") == Some(GRIPPER_TYPE_SERVO);
        Setup {
            ardupilot,
            enabled,
            loaded: parameter("GRIP_TYPE").is_some(),
            typed: parameter("GRIP_TYPE").is_some_and(|kind| kind != GRIPPER_TYPE_NONE),
            stale: false,
            output: !servo
                || (1..=SERVO_OUTPUTS)
                    .any(|n| parameter(&format!("SERVO{n}_FUNCTION")) == Some(GRIPPER_FUNCTION)),
            auto_close_s: parameter("GRIP_AUTOCLOSE").unwrap_or(0.0),
            fs_options: parameter("FS_OPTIONS").map_or(0, |v| v as i64),
            gcs_failsafe: parameter("FS_GCS_ENABLE").is_some_and(|v| v != 0.0),
            gcs_timeout_s: parameter("FS_GCS_TIMEOUT").unwrap_or(DEFAULT_GCS_TIMEOUT_SECONDS),
            drops,
            early_drops,
            servos: servos
                .iter()
                .map(|servo| ServoBay {
                    output: servo.output,
                    function: parameter(&format!("SERVO{}_FUNCTION", servo.output))
                        .map(|f| f as i64),
                })
                .collect(),
        }
    }

    pub fn json(&self) -> Value {
        json!({
            "ardupilot": self.ardupilot,
            "enabled": self.enabled,
            "loaded": self.loaded,
            "typed": self.typed,
            "stale": self.stale,
            "output": self.output,
            "autoCloseSeconds": self.auto_close_s,
            "fsOptions": self.fs_options,
            "gcsFailsafe": self.gcs_failsafe,
            "gcsTimeoutSeconds": self.gcs_timeout_s,
            "drops": self.drops,
            "earlyDrops": self.early_drops,
            "servos": self.servos.iter().map(|bay| json!({ "output": bay.output, "function": bay.function })).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(value: &Value) -> Setup {
        let flag = |key: &str| value.get(key).and_then(Value::as_bool).unwrap_or(false);
        let number = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        Setup {
            ardupilot: flag("ardupilot"),
            enabled: flag("enabled"),
            loaded: flag("loaded"),
            typed: flag("typed"),
            stale: flag("stale"),
            output: flag("output"),
            auto_close_s: number("autoCloseSeconds"),
            fs_options: number("fsOptions") as i64,
            gcs_failsafe: flag("gcsFailsafe"),
            gcs_timeout_s: number("gcsTimeoutSeconds"),
            drops: number("drops") as usize,
            early_drops: number("earlyDrops") as usize,
            servos: value
                .get("servos")
                .and_then(Value::as_array)
                .map(|bays| {
                    bays.iter()
                        .map(|bay| ServoBay {
                            output: bay.get("output").and_then(Value::as_i64).unwrap_or(0),
                            function: bay.get("function").and_then(Value::as_i64),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub fn stays_open(&self) -> bool {
        self.ardupilot && self.auto_close_s <= 0.0
    }

    pub fn servo(&self) -> bool {
        !self.servos.is_empty()
    }

    fn numbered(&self) -> impl Iterator<Item = (i64, &ServoBay)> {
        (1..).zip(self.servos.iter())
    }

    fn unset_bay(&self) -> Option<i64> {
        self.numbered()
            .find(|(_, bay)| bay.output < 1)
            .map(|(number, _)| number)
    }

    fn shared_output(&self) -> Option<(i64, i64, i64)> {
        self.numbered()
            .filter(|(_, bay)| bay.output > 0)
            .find_map(|(first, bay)| {
                self.numbered()
                    .find(|(second, other)| *second > first && other.output == bay.output)
                    .map(|(second, _)| (first, second, bay.output))
            })
    }

    fn busy_output(&self) -> Option<(i64, i64, i64)> {
        self.numbered().find_map(|(number, bay)| {
            bay.function
                .filter(|function| !takes_set_servo(*function))
                .map(|function| (number, bay.output, function))
        })
    }

    fn gripper_on_a_bay(&self) -> bool {
        self.servos
            .iter()
            .any(|bay| bay.function == Some(GRIPPER_FUNCTION as i64))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    DropAtLaunch,
    NoGripper,
    NeedsReboot,
    NoType,
    NoOutput,
    StaysOpen,
    NoBayOutput,
    SharedOutput,
    OutputInUse,
    TooManyDrops,
    FailsafeDrops,
    LinkLossSkipsDrops,
}

const PROBLEMS: [Problem; 12] = [
    Problem::DropAtLaunch,
    Problem::NoGripper,
    Problem::NeedsReboot,
    Problem::NoType,
    Problem::NoOutput,
    Problem::StaysOpen,
    Problem::NoBayOutput,
    Problem::SharedOutput,
    Problem::OutputInUse,
    Problem::TooManyDrops,
    Problem::FailsafeDrops,
    Problem::LinkLossSkipsDrops,
];

impl Problem {
    pub fn code(self) -> &'static str {
        match self {
            Problem::DropAtLaunch => "dropAtLaunch",
            Problem::NoGripper => "noGripper",
            Problem::NeedsReboot => "needsReboot",
            Problem::NoType => "noType",
            Problem::NoOutput => "noOutput",
            Problem::StaysOpen => "staysOpen",
            Problem::NoBayOutput => "noBayOutput",
            Problem::SharedOutput => "sharedOutput",
            Problem::OutputInUse => "outputInUse",
            Problem::TooManyDrops => "tooManyDrops",
            Problem::FailsafeDrops => "failsafeDrops",
            Problem::LinkLossSkipsDrops => "linkLossSkipsDrops",
        }
    }

    pub fn of_code(code: &str) -> Option<Problem> {
        PROBLEMS.into_iter().find(|p| p.code() == code)
    }

    pub fn blocks(self) -> bool {
        !matches!(self, Problem::FailsafeDrops | Problem::LinkLossSkipsDrops)
    }

    fn applies(self, s: &Setup, bays: i64) -> bool {
        let releases = s.drops.max(usize::try_from(bays).unwrap_or(0));
        let gripper = !s.servo();
        match self {
            Problem::DropAtLaunch => s.early_drops > 0,
            Problem::NoGripper => gripper && s.drops > 0 && !s.enabled,
            Problem::NeedsReboot => gripper && s.ardupilot && s.enabled && (!s.loaded || s.stale),
            Problem::NoType => {
                gripper && s.ardupilot && s.enabled && s.loaded && !s.stale && !s.typed
            }
            Problem::NoOutput => gripper && s.ardupilot && s.enabled && s.loaded && !s.output,
            Problem::StaysOpen => {
                gripper && s.enabled && s.loaded && s.stays_open() && releases > 1
            }
            Problem::NoBayOutput => s.unset_bay().is_some(),
            Problem::SharedOutput => s.shared_output().is_some(),
            Problem::OutputInUse => s.busy_output().is_some(),
            Problem::TooManyDrops => s.servo() && s.drops > s.servos.len(),
            Problem::FailsafeDrops => {
                s.ardupilot
                    && s.enabled
                    && s.fs_options & RELEASE_GRIPPER_ON_FAILSAFE != 0
                    && (gripper || s.gripper_on_a_bay())
            }
            Problem::LinkLossSkipsDrops => {
                s.ardupilot
                    && s.drops > 0
                    && s.gcs_failsafe
                    && s.fs_options & CONTINUE_AUTO_ON_GCS_FAILSAFE == 0
            }
        }
    }

    pub fn text(self, s: &Setup, bays: i64) -> String {
        let releases = s.drops.max(usize::try_from(bays).unwrap_or(0));
        match self {
            Problem::DropAtLaunch => "The mission drops before its first waypoint, so the payload would fall over the launch point. Put a waypoint before the drop.".to_string(),
            Problem::NoGripper => "The mission drops a payload, but the flight controller has no gripper set up (GRIP_ENABLE is off).".to_string(),
            Problem::NeedsReboot if s.loaded => "The gripper settings changed since the flight controller started, and it only reads them at startup. Reboot the flight controller.".to_string(),
            Problem::NeedsReboot => "The gripper was just turned on. Reboot the flight controller so its gripper settings appear.".to_string(),
            Problem::NoType => "The gripper is on, but its type is None (GRIP_TYPE), so nothing moves at a drop. Make it a servo gripper, then reboot the flight controller.".to_string(),
            Problem::NoOutput => "The gripper has no output to move. Give one servo output the Gripper function (SERVOn_FUNCTION 28).".to_string(),
            Problem::StaysOpen => match releases - 1 {
                1 => "The gripper stays open after the first drop, so the second drop releases nothing. Turn on auto-close so it closes after each drop.".to_string(),
                more => format!("The gripper stays open after the first drop, so the other {more} drops release nothing. Turn on auto-close so it closes after each drop."),
            },
            Problem::NoBayOutput => format!("Bay {} has no servo output, so it cannot drop. {SET_IN_SETTINGS}", s.unset_bay().unwrap_or_default()),
            Problem::SharedOutput => {
                let (first, second, output) = s.shared_output().unwrap_or_default();
                format!("Bays {first} and {second} both use output {output}, so dropping one drops both. Give each bay its own output.")
            }
            Problem::OutputInUse => {
                let (bay, output, function) = s.busy_output().unwrap_or_default();
                let named = format!("ArduPilot will not move output {output} for bay {bay}: SERVO{output}_FUNCTION is {function}, so the output already has a job.");
                match freeable(function) {
                    true => named,
                    false => format!("{named} Choose another output for bay {bay}."),
                }
            }
            Problem::TooManyDrops => format!("The mission drops {} times but there are {} bays, so a bay would drop twice.", s.drops, s.servos.len()),
            Problem::FailsafeDrops => "Any failsafe opens the gripper and drops the payload wherever the drone is (FS_OPTIONS Release Gripper).".to_string(),
            Problem::LinkLossSkipsDrops => format!("If the link drops for {} s, the drone flies home and skips the drops still to come.", s.gcs_timeout_s),
        }
    }

    pub fn fix_label(self, s: &Setup) -> String {
        match self {
            Problem::NeedsReboot => "Reboot the flight controller".to_string(),
            Problem::NoType => "Make it a servo gripper".to_string(),
            Problem::StaysOpen => "Turn on auto-close".to_string(),
            Problem::FailsafeDrops => "Keep the payload on failsafe".to_string(),
            Problem::LinkLossSkipsDrops => "Keep flying the mission".to_string(),
            Problem::OutputInUse => self
                .fix(s)
                .and(s.busy_output())
                .map_or(String::new(), |(_, output, _)| {
                    format!("Free output {output}")
                }),
            Problem::DropAtLaunch
            | Problem::NoGripper
            | Problem::NoOutput
            | Problem::NoBayOutput
            | Problem::SharedOutput
            | Problem::TooManyDrops => String::new(),
        }
    }

    pub fn fix(self, s: &Setup) -> Option<(String, f64)> {
        match self {
            Problem::StaysOpen => Some(("GRIP_AUTOCLOSE".to_string(), AUTO_CLOSE_SECONDS)),
            Problem::NoType => Some(("GRIP_TYPE".to_string(), GRIPPER_TYPE_SERVO)),
            Problem::FailsafeDrops => Some((
                "FS_OPTIONS".to_string(),
                (s.fs_options & !RELEASE_GRIPPER_ON_FAILSAFE) as f64,
            )),
            Problem::LinkLossSkipsDrops => Some((
                "FS_OPTIONS".to_string(),
                (s.fs_options | CONTINUE_AUTO_ON_GCS_FAILSAFE) as f64,
            )),
            Problem::OutputInUse => s
                .busy_output()
                .filter(|(_, _, function)| freeable(*function))
                .map(|(_, output, _)| (format!("SERVO{output}_FUNCTION"), FREE_FUNCTION)),
            Problem::DropAtLaunch
            | Problem::NoGripper
            | Problem::NeedsReboot
            | Problem::NoOutput
            | Problem::NoBayOutput
            | Problem::SharedOutput
            | Problem::TooManyDrops => None,
        }
    }
}

pub fn problems(s: &Setup, bays: i64) -> Vec<Problem> {
    PROBLEMS
        .into_iter()
        .filter(|p| p.applies(s, bays))
        .collect()
}

pub fn blocking(s: &Setup, bays: i64) -> Option<String> {
    problems(s, bays)
        .into_iter()
        .find(|p| p.blocks())
        .map(|p| p.text(s, bays))
}

pub fn problem_json(p: Problem, s: &Setup, bays: i64) -> Value {
    json!({ "code": p.code(), "text": p.text(s, bays), "blocking": p.blocks(), "fix": p.fix_label(s) })
}

pub fn problems_json(s: &Setup, bays: i64) -> Value {
    json!(
        problems(s, bays)
            .into_iter()
            .map(|p| problem_json(p, s, bays))
            .collect::<Vec<_>>()
    )
}

pub fn release_refusal(s: &Setup, released_at_ms: Option<i64>, now_ms: i64) -> Option<String> {
    [Problem::NeedsReboot, Problem::NoType, Problem::NoOutput]
        .into_iter()
        .find(|problem| problem.applies(s, 1))
        .map(|problem| format!("Nothing would fall. {}", problem.text(s, 1)))
        .or_else(|| open_refusal(s, released_at_ms, now_ms))
}

fn open_refusal(s: &Setup, released_at_ms: Option<i64>, now_ms: i64) -> Option<String> {
    let since = released_at_ms?;
    match s.ardupilot && s.enabled {
        false => None,
        true if s.stays_open() => Some("The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.".to_string()),
        true => {
            let left_ms = since + (s.auto_close_s * 1000.0) as i64 - now_ms;
            (left_ms > 0).then(|| format!("The gripper is still open from the last drop. It closes in {} s.", (left_ms + 999) / 1000))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(seq: u16, command: u16, action: f64) -> Item {
        Item {
            seq,
            frame: 2,
            command,
            current: false,
            auto_continue: true,
            params: [1.0, action, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    fn copter(drops: usize) -> Setup {
        Setup {
            ardupilot: true,
            enabled: true,
            loaded: true,
            typed: true,
            stale: false,
            output: true,
            auto_close_s: 0.0,
            fs_options: 0,
            gcs_failsafe: false,
            gcs_timeout_s: 5.0,
            drops,
            early_drops: 0,
            servos: Vec::new(),
        }
    }

    #[test]
    fn only_gripper_releases_count_as_drops() {
        let items = [
            item(0, 16, 0.0),
            item(1, CMD_DO_GRIPPER, 0.0),
            item(2, CMD_DO_GRIPPER, 1.0),
            item(3, CMD_DO_GRIPPER, 0.0),
            item(4, 20, 0.0),
        ];
        assert_eq!(
            drops_in(&items, &[]),
            2,
            "a grab is not a drop, and a waypoint's param2 is not a gripper action"
        );
    }

    #[test]
    fn a_drop_before_the_first_waypoint_falls_over_the_launch_point() {
        let home = item(0, 16, 0.0);
        let mission = [
            home.clone(),
            item(1, CMD_DO_GRIPPER, 0.0),
            item(2, 22, 0.0),
            item(3, CMD_DO_GRIPPER, 0.0),
            item(4, 16, 0.0),
            item(5, CMD_DO_GRIPPER, 0.0),
        ];
        assert_eq!(
            early_drops_in(&mission, true, &[]),
            2,
            "home is not a waypoint, and neither is the takeoff"
        );
        assert_eq!(
            early_drops_in(&mission[1..], false, &[]),
            2,
            "PX4 sends no home"
        );
        assert_eq!(
            early_drops_in(
                &[
                    home,
                    item(1, 22, 0.0),
                    item(2, 16, 0.0),
                    item(3, CMD_DO_GRIPPER, 0.0)
                ],
                true,
                &[]
            ),
            0
        );
        let early = Setup {
            early_drops: 1,
            ..copter(1)
        };
        assert_eq!(problems(&early, 1), vec![Problem::DropAtLaunch]);
        assert!(
            Problem::DropAtLaunch.blocks()
                && blocking(&early, 1).is_some_and(|text| text.contains("launch point"))
        );
    }

    #[test]
    fn a_gripper_that_stays_open_blocks_a_second_drop() {
        assert_eq!(
            problems(&copter(1), 1),
            vec![],
            "one drop needs nothing to close again"
        );
        assert_eq!(problems(&copter(4), 1), vec![Problem::StaysOpen]);
        assert_eq!(
            problems(&copter(0), 4),
            vec![Problem::StaysOpen],
            "four bays dropped by hand need it as much as a mission does"
        );
        assert_eq!(
            problems(
                &Setup {
                    auto_close_s: 2.0,
                    ..copter(4)
                },
                4
            ),
            vec![]
        );
        assert_eq!(
            blocking(&copter(4), 1).as_deref(),
            Some(
                "The gripper stays open after the first drop, so the other 3 drops release nothing. Turn on auto-close so it closes after each drop."
            )
        );
        assert_eq!(
            Problem::StaysOpen.fix(&copter(4)),
            Some(("GRIP_AUTOCLOSE".to_string(), AUTO_CLOSE_SECONDS))
        );
    }

    #[test]
    fn a_mission_that_drops_needs_a_gripper_with_an_output() {
        assert_eq!(
            problems(
                &Setup {
                    enabled: false,
                    ..copter(1)
                },
                1
            ),
            vec![Problem::NoGripper]
        );
        assert_eq!(
            problems(
                &Setup {
                    enabled: false,
                    ..copter(0)
                },
                1
            ),
            vec![],
            "nothing planned, nothing missing"
        );
        assert_eq!(
            problems(
                &Setup {
                    output: false,
                    ..copter(1)
                },
                1
            ),
            vec![Problem::NoOutput]
        );
        assert!(Problem::NoGripper.fix(&copter(1)).is_none() && Problem::NoGripper.blocks());
    }

    #[test]
    fn a_gripper_turned_on_without_a_reboot_asks_for_one_before_anything_else() {
        let fresh = Setup {
            loaded: false,
            output: true,
            ..copter(4)
        };
        assert_eq!(
            problems(&fresh, 1),
            vec![Problem::NeedsReboot],
            "until the reboot there is no GRIP_AUTOCLOSE to turn on, so only the reboot is offered"
        );
        assert_eq!(
            (
                Problem::NeedsReboot.fix_label(&fresh),
                Problem::NeedsReboot.blocks()
            ),
            ("Reboot the flight controller".to_string(), true)
        );
        assert!(
            !Setup::read(true, true, |_| None, &[], 0, 0).loaded,
            "GRIP_TYPE appears only after the reboot"
        );
    }

    #[test]
    fn a_gripper_of_type_none_is_made_a_servo_and_then_rebooted() {
        let untyped = Setup {
            typed: false,
            ..copter(1)
        };
        assert_eq!(
            problems(&untyped, 1),
            vec![Problem::NoType],
            "ArduPilot makes no gripper of type None, so every drop would do nothing"
        );
        assert!(Problem::NoType.blocks());
        assert_eq!(
            Problem::NoType.fix(&untyped),
            Some(("GRIP_TYPE".to_string(), 1.0))
        );
        let changed = Setup {
            stale: true,
            ..copter(1)
        };
        assert_eq!(
            problems(&changed, 1),
            vec![Problem::NeedsReboot],
            "the gripper is made at startup, so a type set since then waits for a reboot"
        );
        assert!(
            Problem::NeedsReboot
                .text(&changed, 1)
                .contains("changed since the flight controller started")
        );
        assert!(
            Setup::read(
                true,
                true,
                |name| (name == "GRIP_TYPE").then_some(0.0),
                &[],
                0,
                0
            )
            .loaded
                && !Setup::read(
                    true,
                    true,
                    |name| (name == "GRIP_TYPE").then_some(0.0),
                    &[],
                    0,
                    0
                )
                .typed
        );
    }

    #[test]
    fn a_hand_drop_is_refused_only_when_nothing_would_fall() {
        assert_eq!(release_refusal(&copter(0), None, 0), None);
        assert!(
            release_refusal(
                &Setup {
                    typed: false,
                    ..copter(0)
                },
                None,
                0
            )
            .is_some_and(
                |reason| reason.starts_with("Nothing would fall.") && reason.contains("GRIP_TYPE")
            )
        );
        assert!(
            release_refusal(
                &Setup {
                    stale: true,
                    ..copter(0)
                },
                None,
                0
            )
            .is_some_and(|reason| reason.contains("Reboot"))
        );
        assert!(
            release_refusal(
                &Setup {
                    output: false,
                    ..copter(0)
                },
                None,
                0
            )
            .is_some_and(|reason| reason.contains("SERVOn_FUNCTION 28"))
        );
        assert!(
            release_refusal(&copter(0), Some(1_000), 60_000)
                .is_some_and(|reason| reason.starts_with("The gripper is still open")),
            "a servo gripper left open has nothing more to drop"
        );
        assert_eq!(
            release_refusal(
                &Setup {
                    enabled: false,
                    ..copter(0)
                },
                None,
                0
            ),
            None,
            "without GRIP_ENABLE the drop is not offered at all"
        );
    }

    #[test]
    fn failsafe_options_that_lose_drops_are_named_and_fixed_bit_by_bit() {
        let releasing = Setup {
            fs_options: 32 | 8,
            ..copter(1)
        };
        assert_eq!(problems(&releasing, 1), vec![Problem::FailsafeDrops]);
        assert_eq!(
            Problem::FailsafeDrops.fix(&releasing),
            Some(("FS_OPTIONS".to_string(), 8.0)),
            "only the Release Gripper bit is cleared"
        );
        let homing = Setup {
            gcs_failsafe: true,
            fs_options: 1,
            ..copter(2)
        };
        assert_eq!(
            problems(
                &Setup {
                    auto_close_s: 2.0,
                    ..homing.clone()
                },
                1
            ),
            vec![Problem::LinkLossSkipsDrops]
        );
        assert_eq!(
            Problem::LinkLossSkipsDrops.fix(&homing),
            Some(("FS_OPTIONS".to_string(), 3.0))
        );
        assert!(
            !Problem::LinkLossSkipsDrops.blocks(),
            "flying home on a lost link is a choice, so it warns without blocking"
        );
        assert_eq!(
            problems(
                &Setup {
                    fs_options: 3,
                    auto_close_s: 2.0,
                    ..homing
                },
                1
            ),
            vec![]
        );
    }

    #[test]
    fn px4_is_only_asked_for_a_gripper() {
        let px4 = Setup {
            ardupilot: false,
            enabled: true,
            loaded: false,
            typed: false,
            stale: false,
            output: true,
            auto_close_s: 0.0,
            fs_options: 32,
            gcs_failsafe: true,
            gcs_timeout_s: 5.0,
            drops: 4,
            early_drops: 0,
            servos: Vec::new(),
        };
        assert_eq!(problems(&px4, 4), vec![]);
        assert_eq!(
            problems(
                &Setup {
                    enabled: false,
                    ..px4
                },
                4
            ),
            vec![Problem::NoGripper]
        );
    }

    #[test]
    fn the_setup_reads_the_flight_controller_parameters() {
        let params = |name: &str| match name {
            "GRIP_TYPE" => Some(1.0),
            "SERVO9_FUNCTION" => Some(28.0),
            "GRIP_AUTOCLOSE" => Some(2.0),
            "FS_OPTIONS" => Some(2.0),
            "FS_GCS_ENABLE" => Some(1.0),
            _ => None,
        };
        let read = Setup::read(true, true, params, &[], 3, 0);
        assert_eq!(
            read,
            Setup {
                ardupilot: true,
                enabled: true,
                loaded: true,
                typed: true,
                stale: false,
                output: true,
                auto_close_s: 2.0,
                fs_options: 2,
                gcs_failsafe: true,
                gcs_timeout_s: 5.0,
                drops: 3,
                early_drops: 0,
                servos: Vec::new(),
            }
        );
        assert_eq!(Setup::from_json(&read.json()), read);
        assert!(
            !Setup::read(
                true,
                true,
                |name| (name == "GRIP_TYPE").then_some(1.0),
                &[],
                0,
                0
            )
            .output,
            "a servo gripper with no output assigned cannot move"
        );
        assert!(
            Setup::read(
                true,
                true,
                |name| (name == "GRIP_TYPE").then_some(2.0),
                &[],
                0,
                0
            )
            .output,
            "an EPM needs no servo output"
        );
    }

    #[test]
    fn a_drop_while_the_gripper_is_still_open_is_refused() {
        assert_eq!(
            open_refusal(&copter(0), None, 5_000),
            None,
            "never released"
        );
        assert!(
            open_refusal(&copter(0), Some(1_000), 60_000)
                .is_some_and(|r| r.starts_with("The gripper is still open")),
            "without auto-close it stays open"
        );
        let closing = Setup {
            auto_close_s: 2.0,
            ..copter(0)
        };
        assert_eq!(
            open_refusal(&closing, Some(1_000), 2_100).as_deref(),
            Some("The gripper is still open from the last drop. It closes in 1 s.")
        );
        assert_eq!(open_refusal(&closing, Some(1_000), 3_000), None);
        assert_eq!(
            open_refusal(
                &Setup {
                    ardupilot: false,
                    ..copter(0)
                },
                Some(1_000),
                1_001
            ),
            None,
            "PX4 grippers are not modelled"
        );
    }

    fn four_servos() -> Vec<Servo> {
        (9..=12)
            .map(|output| Servo {
                output,
                release: 1900.0,
                hold: 1100.0,
            })
            .collect()
    }

    fn servo_copter(functions: [Option<i64>; 4], drops: usize) -> Setup {
        Setup {
            servos: (9..=12)
                .zip(functions)
                .map(|(output, function)| ServoBay { output, function })
                .collect(),
            ..copter(drops)
        }
    }

    #[test]
    fn a_servo_drop_is_the_release_pwm_on_a_bays_own_output() {
        let servos = four_servos();
        let set = |output: f64, pwm: f64| Item {
            params: [output, pwm, 0.0, 0.0, 0.0, 0.0, 0.0],
            ..item(3, CMD_DO_SET_SERVO, 0.0)
        };
        assert_eq!(bay_of(&set(10.0, 1900.0), &servos), Some(2));
        assert!(
            !is_release(&set(10.0, 1100.0), &servos),
            "the hold PWM closes the bay, it drops nothing"
        );
        assert!(
            !is_release(&set(5.0, 1900.0), &servos),
            "an output no bay uses is someone else's servo, a light or a camera"
        );
        assert!(
            !is_release(&set(10.0, 1900.0), &[]),
            "with one gripper a DO_SET_SERVO is never a drop"
        );
        let mission = [
            item(0, 16, 0.0),
            item(1, 16, 0.0),
            set(9.0, 1900.0),
            item(3, 16, 0.0),
            set(10.0, 1900.0),
            item(5, CMD_DO_GRIPPER, 0.0),
        ];
        assert_eq!(
            drops_in(&mission, &servos),
            3,
            "a gripper release still drops"
        );
        assert_eq!(
            early_drops_in(&[item(0, 16, 0.0), set(9.0, 1900.0)], true, &servos),
            1
        );
    }

    #[test]
    fn a_hand_drop_on_a_servo_bay_sends_that_bays_output_and_pwm() {
        let servos = four_servos();
        assert_eq!(servo_command(&servos, 0.0, 2), Ok((10.0, 1900.0)));
        assert_eq!(
            servo_command(&servos, 1.0, 4),
            Ok((12.0, 1100.0)),
            "a grab sends the hold PWM"
        );
        assert!(
            servo_command(&servos, 2.0, 1).is_err(),
            "a servo has no hold-in-place"
        );
        assert_eq!(
            servo_command(&servos, 0.0, 5).unwrap_err(),
            "There is no bay 5. This drone has 4 bays."
        );
        let unset = [Servo {
            output: 0,
            ..servos[0]
        }];
        assert!(
            servo_command(&unset, 0.0, 1)
                .is_err_and(|e| e.starts_with("Bay 1 has no servo output"))
        );
    }

    #[test]
    fn servo_bays_skip_the_gripper_checks_and_check_their_own_outputs() {
        let free = servo_copter([Some(0), Some(0), Some(0), Some(0)], 4);
        assert_eq!(
            problems(
                &Setup {
                    auto_close_s: 0.0,
                    typed: false,
                    enabled: false,
                    ..free.clone()
                },
                4
            ),
            vec![],
            "no GRIP_ENABLE, GRIP_TYPE or auto-close matters when each bay has its own servo"
        );
        assert_eq!(
            problems(
                &Setup {
                    drops: 5,
                    ..free.clone()
                },
                4
            ),
            vec![Problem::TooManyDrops]
        );
        let unset = Setup {
            servos: vec![ServoBay {
                output: 0,
                function: None,
            }],
            ..free.clone()
        };
        assert_eq!(
            problems(&unset, 1),
            vec![Problem::NoBayOutput, Problem::TooManyDrops]
        );
        let shared = Setup {
            servos: [9, 10, 10, 12]
                .map(|output| ServoBay {
                    output,
                    function: Some(0),
                })
                .to_vec(),
            ..free.clone()
        };
        assert_eq!(
            Problem::SharedOutput.text(&shared, 4),
            "Bays 2 and 3 both use output 10, so dropping one drops both. Give each bay its own output."
        );
        assert!(
            problems(&shared, 4).contains(&Problem::SharedOutput) && Problem::SharedOutput.blocks()
        );
    }

    #[test]
    fn an_output_busy_with_an_accessory_is_freed_in_one_tap_but_a_motor_never_is() {
        let camera = servo_copter([Some(0), Some(10), Some(0), Some(1)], 2);
        assert_eq!(problems(&camera, 4), vec![Problem::OutputInUse]);
        assert!(Problem::OutputInUse.blocks());
        assert_eq!(
            Problem::OutputInUse.fix(&camera),
            Some(("SERVO10_FUNCTION".to_string(), 0.0))
        );
        assert_eq!(Problem::OutputInUse.fix_label(&camera), "Free output 10");
        let motor = servo_copter([Some(0), Some(0), Some(37), Some(0)], 2);
        assert_eq!(
            Problem::OutputInUse.fix(&motor),
            None,
            "freeing a motor output grounds the drone"
        );
        assert_eq!(Problem::OutputInUse.fix_label(&motor), "");
        assert!(
            Problem::OutputInUse
                .text(&motor, 4)
                .ends_with("Choose another output for bay 3.")
        );
        assert_eq!(
            problems(
                &servo_copter([Some(28), Some(51), Some(140), Some(1)], 2),
                4
            ),
            vec![],
            "ArduPilot's do_set_servo moves Disabled, RCPassThru, RCIN, RCIN mapped and Gripper outputs"
        );
    }

    #[test]
    fn a_failsafe_drops_a_servo_bay_only_when_that_bay_is_on_the_gripper_function() {
        let releasing = |functions| Setup {
            fs_options: 32,
            ..servo_copter(functions, 1)
        };
        assert_eq!(
            problems(&releasing([Some(0), Some(0), Some(0), Some(0)]), 4),
            vec![]
        );
        assert_eq!(
            problems(&releasing([Some(28), Some(0), Some(0), Some(0)]), 4),
            vec![Problem::FailsafeDrops],
            "a failsafe calls the gripper's release, which moves every output on function 28"
        );
    }

    #[test]
    fn the_setup_reads_each_bays_output_function() {
        let params = |name: &str| match name {
            "SERVO9_FUNCTION" => Some(0.0),
            "SERVO10_FUNCTION" => Some(33.0),
            _ => None,
        };
        let read = Setup::read(true, false, params, &four_servos()[..2], 2, 0);
        assert_eq!(
            read.servos,
            vec![
                ServoBay {
                    output: 9,
                    function: Some(0)
                },
                ServoBay {
                    output: 10,
                    function: Some(33)
                }
            ]
        );
        assert_eq!(Setup::from_json(&read.json()), read);
    }
}
