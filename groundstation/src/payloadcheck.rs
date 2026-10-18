use serde_json::{json, Value};

use crate::plantransfer::Item;

pub const CMD_DO_GRIPPER: u16 = 211;
const GRIPPER_RELEASE: f64 = 0.0;
const GRIPPER_FUNCTION: f64 = 28.0;
const SERVO_OUTPUTS: usize = 32;
const GRIPPER_TYPE_SERVO: f64 = 1.0;
pub const AUTO_CLOSE_SECONDS: f64 = 2.0;
pub const DROP_HOLD_SECONDS: f64 = 2.0;
const CONTINUE_AUTO_ON_GCS_FAILSAFE: i64 = 2;
const RELEASE_GRIPPER_ON_FAILSAFE: i64 = 32;
const DEFAULT_GCS_TIMEOUT_SECONDS: f64 = 5.0;

pub fn is_release(item: &Item) -> bool {
    item.command == CMD_DO_GRIPPER && item.params[1] == GRIPPER_RELEASE
}

pub fn drops_in(items: &[Item]) -> usize {
    items.iter().filter(|item| is_release(item)).count()
}

const PLACES: [u16; 6] = [16, 17, 18, 19, 31, 82];

pub fn early_drops_in(items: &[Item], sends_home: bool) -> usize {
    items.iter().skip(usize::from(sends_home)).take_while(|item| !PLACES.contains(&item.command)).filter(|item| is_release(item)).count()
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Setup {
    pub ardupilot: bool,
    pub enabled: bool,
    pub loaded: bool,
    pub output: bool,
    pub auto_close_s: f64,
    pub fs_options: i64,
    pub gcs_failsafe: bool,
    pub gcs_timeout_s: f64,
    pub drops: usize,
    pub early_drops: usize,
}

impl Setup {
    pub fn read(ardupilot: bool, enabled: bool, parameter: impl Fn(&str) -> Option<f64>, drops: usize, early_drops: usize) -> Setup {
        let servo = parameter("GRIP_TYPE") == Some(GRIPPER_TYPE_SERVO);
        Setup {
            ardupilot,
            enabled,
            loaded: parameter("GRIP_TYPE").is_some(),
            output: !servo || (1..=SERVO_OUTPUTS).any(|n| parameter(&format!("SERVO{n}_FUNCTION")) == Some(GRIPPER_FUNCTION)),
            auto_close_s: parameter("GRIP_AUTOCLOSE").unwrap_or(0.0),
            fs_options: parameter("FS_OPTIONS").map_or(0, |v| v as i64),
            gcs_failsafe: parameter("FS_GCS_ENABLE").is_some_and(|v| v != 0.0),
            gcs_timeout_s: parameter("FS_GCS_TIMEOUT").unwrap_or(DEFAULT_GCS_TIMEOUT_SECONDS),
            drops,
            early_drops,
        }
    }

    pub fn json(&self) -> Value {
        json!({
            "ardupilot": self.ardupilot,
            "enabled": self.enabled,
            "loaded": self.loaded,
            "output": self.output,
            "autoCloseSeconds": self.auto_close_s,
            "fsOptions": self.fs_options,
            "gcsFailsafe": self.gcs_failsafe,
            "gcsTimeoutSeconds": self.gcs_timeout_s,
            "drops": self.drops,
            "earlyDrops": self.early_drops,
        })
    }

    pub fn from_json(value: &Value) -> Setup {
        let flag = |key: &str| value.get(key).and_then(Value::as_bool).unwrap_or(false);
        let number = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        Setup {
            ardupilot: flag("ardupilot"),
            enabled: flag("enabled"),
            loaded: flag("loaded"),
            output: flag("output"),
            auto_close_s: number("autoCloseSeconds"),
            fs_options: number("fsOptions") as i64,
            gcs_failsafe: flag("gcsFailsafe"),
            gcs_timeout_s: number("gcsTimeoutSeconds"),
            drops: number("drops") as usize,
            early_drops: number("earlyDrops") as usize,
        }
    }

    pub fn stays_open(&self) -> bool {
        self.ardupilot && self.auto_close_s <= 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    DropAtLaunch,
    NoGripper,
    NeedsReboot,
    NoOutput,
    StaysOpen,
    FailsafeDrops,
    LinkLossSkipsDrops,
}

const PROBLEMS: [Problem; 7] = [Problem::DropAtLaunch, Problem::NoGripper, Problem::NeedsReboot, Problem::NoOutput, Problem::StaysOpen, Problem::FailsafeDrops, Problem::LinkLossSkipsDrops];

impl Problem {
    pub fn code(self) -> &'static str {
        match self {
            Problem::DropAtLaunch => "dropAtLaunch",
            Problem::NoGripper => "noGripper",
            Problem::NeedsReboot => "needsReboot",
            Problem::NoOutput => "noOutput",
            Problem::StaysOpen => "staysOpen",
            Problem::FailsafeDrops => "failsafeDrops",
            Problem::LinkLossSkipsDrops => "linkLossSkipsDrops",
        }
    }

    pub fn of_code(code: &str) -> Option<Problem> {
        PROBLEMS.into_iter().find(|p| p.code() == code)
    }

    pub fn blocks(self) -> bool {
        matches!(self, Problem::DropAtLaunch | Problem::NoGripper | Problem::NeedsReboot | Problem::NoOutput | Problem::StaysOpen)
    }

    fn applies(self, s: &Setup, bays: i64) -> bool {
        let releases = s.drops.max(usize::try_from(bays).unwrap_or(0));
        match self {
            Problem::DropAtLaunch => s.early_drops > 0,
            Problem::NoGripper => s.drops > 0 && !s.enabled,
            Problem::NeedsReboot => s.ardupilot && s.enabled && !s.loaded,
            Problem::NoOutput => s.ardupilot && s.enabled && s.loaded && !s.output,
            Problem::StaysOpen => s.enabled && s.loaded && s.stays_open() && releases > 1,
            Problem::FailsafeDrops => s.ardupilot && s.enabled && s.fs_options & RELEASE_GRIPPER_ON_FAILSAFE != 0,
            Problem::LinkLossSkipsDrops => s.ardupilot && s.drops > 0 && s.gcs_failsafe && s.fs_options & CONTINUE_AUTO_ON_GCS_FAILSAFE == 0,
        }
    }

    pub fn text(self, s: &Setup, bays: i64) -> String {
        let releases = s.drops.max(usize::try_from(bays).unwrap_or(0));
        match self {
            Problem::DropAtLaunch => "The mission drops before its first waypoint, so the payload would fall over the launch point. Put a waypoint before the drop.".to_string(),
            Problem::NoGripper => "The mission drops a payload, but the flight controller has no gripper set up (GRIP_ENABLE is off).".to_string(),
            Problem::NeedsReboot => "The gripper was just turned on. Reboot the flight controller so its gripper settings appear.".to_string(),
            Problem::NoOutput => "The gripper has no output to move. Give one servo output the Gripper function (SERVOn_FUNCTION 28).".to_string(),
            Problem::StaysOpen => match releases - 1 {
                1 => "The gripper stays open after the first drop, so the second drop releases nothing. Turn on auto-close so it closes after each drop.".to_string(),
                more => format!("The gripper stays open after the first drop, so the other {more} drops release nothing. Turn on auto-close so it closes after each drop."),
            },
            Problem::FailsafeDrops => "Any failsafe opens the gripper and drops the payload wherever the drone is (FS_OPTIONS Release Gripper).".to_string(),
            Problem::LinkLossSkipsDrops => format!("If the link drops for {} s, the drone flies home and skips the drops still to come.", s.gcs_timeout_s),
        }
    }

    pub fn fix_label(self) -> &'static str {
        match self {
            Problem::DropAtLaunch => "",
            Problem::NeedsReboot => "Reboot the flight controller",
            Problem::StaysOpen => "Turn on auto-close",
            Problem::FailsafeDrops => "Keep the payload on failsafe",
            Problem::LinkLossSkipsDrops => "Keep flying the mission",
            Problem::NoGripper | Problem::NoOutput => "",
        }
    }

    pub fn fix(self, s: &Setup) -> Option<(&'static str, f64)> {
        match self {
            Problem::StaysOpen => Some(("GRIP_AUTOCLOSE", AUTO_CLOSE_SECONDS)),
            Problem::FailsafeDrops => Some(("FS_OPTIONS", (s.fs_options & !RELEASE_GRIPPER_ON_FAILSAFE) as f64)),
            Problem::LinkLossSkipsDrops => Some(("FS_OPTIONS", (s.fs_options | CONTINUE_AUTO_ON_GCS_FAILSAFE) as f64)),
            Problem::DropAtLaunch | Problem::NoGripper | Problem::NeedsReboot | Problem::NoOutput => None,
        }
    }
}

pub fn problems(s: &Setup, bays: i64) -> Vec<Problem> {
    PROBLEMS.into_iter().filter(|p| p.applies(s, bays)).collect()
}

pub fn blocking(s: &Setup, bays: i64) -> Option<String> {
    problems(s, bays).into_iter().find(|p| p.blocks()).map(|p| p.text(s, bays))
}

pub fn problems_json(s: &Setup, bays: i64) -> Value {
    json!(problems(s, bays)
        .into_iter()
        .map(|p| json!({ "code": p.code(), "text": p.text(s, bays), "blocking": p.blocks(), "fix": p.fix_label() }))
        .collect::<Vec<_>>())
}

pub fn open_refusal(s: &Setup, released_at_ms: Option<i64>, now_ms: i64) -> Option<String> {
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
        Item { seq, frame: 2, command, current: false, auto_continue: true, params: [1.0, action, 0.0, 0.0, 0.0, 0.0, 0.0] }
    }

    fn copter(drops: usize) -> Setup {
        Setup { ardupilot: true, enabled: true, loaded: true, output: true, auto_close_s: 0.0, fs_options: 0, gcs_failsafe: false, gcs_timeout_s: 5.0, drops, early_drops: 0 }
    }

    #[test]
    fn only_gripper_releases_count_as_drops() {
        let items = [item(0, 16, 0.0), item(1, CMD_DO_GRIPPER, 0.0), item(2, CMD_DO_GRIPPER, 1.0), item(3, CMD_DO_GRIPPER, 0.0), item(4, 20, 0.0)];
        assert_eq!(drops_in(&items), 2, "a grab is not a drop, and a waypoint's param2 is not a gripper action");
    }

    #[test]
    fn a_drop_before_the_first_waypoint_falls_over_the_launch_point() {
        let home = item(0, 16, 0.0);
        let mission = [home.clone(), item(1, CMD_DO_GRIPPER, 0.0), item(2, 22, 0.0), item(3, CMD_DO_GRIPPER, 0.0), item(4, 16, 0.0), item(5, CMD_DO_GRIPPER, 0.0)];
        assert_eq!(early_drops_in(&mission, true), 2, "home is not a waypoint, and neither is the takeoff");
        assert_eq!(early_drops_in(&mission[1..], false), 2, "PX4 sends no home");
        assert_eq!(early_drops_in(&[home, item(1, 22, 0.0), item(2, 16, 0.0), item(3, CMD_DO_GRIPPER, 0.0)], true), 0);
        let early = Setup { early_drops: 1, ..copter(1) };
        assert_eq!(problems(&early, 1), vec![Problem::DropAtLaunch]);
        assert!(Problem::DropAtLaunch.blocks() && blocking(&early, 1).is_some_and(|text| text.contains("launch point")));
    }

    #[test]
    fn a_gripper_that_stays_open_blocks_a_second_drop() {
        assert_eq!(problems(&copter(1), 1), vec![], "one drop needs nothing to close again");
        assert_eq!(problems(&copter(4), 1), vec![Problem::StaysOpen]);
        assert_eq!(problems(&copter(0), 4), vec![Problem::StaysOpen], "four bays dropped by hand need it as much as a mission does");
        assert_eq!(problems(&Setup { auto_close_s: 2.0, ..copter(4) }, 4), vec![]);
        assert_eq!(blocking(&copter(4), 1).as_deref(), Some("The gripper stays open after the first drop, so the other 3 drops release nothing. Turn on auto-close so it closes after each drop."));
        assert_eq!(Problem::StaysOpen.fix(&copter(4)), Some(("GRIP_AUTOCLOSE", AUTO_CLOSE_SECONDS)));
    }

    #[test]
    fn a_mission_that_drops_needs_a_gripper_with_an_output() {
        assert_eq!(problems(&Setup { enabled: false, ..copter(1) }, 1), vec![Problem::NoGripper]);
        assert_eq!(problems(&Setup { enabled: false, ..copter(0) }, 1), vec![], "nothing planned, nothing missing");
        assert_eq!(problems(&Setup { output: false, ..copter(1) }, 1), vec![Problem::NoOutput]);
        assert!(Problem::NoGripper.fix(&copter(1)).is_none() && Problem::NoGripper.blocks());
    }

    #[test]
    fn a_gripper_turned_on_without_a_reboot_asks_for_one_before_anything_else() {
        let fresh = Setup { loaded: false, output: true, ..copter(4) };
        assert_eq!(problems(&fresh, 1), vec![Problem::NeedsReboot], "until the reboot there is no GRIP_AUTOCLOSE to turn on, so only the reboot is offered");
        assert_eq!((Problem::NeedsReboot.fix_label(), Problem::NeedsReboot.blocks()), ("Reboot the flight controller", true));
        assert!(!Setup::read(true, true, |_| None, 0, 0).loaded, "GRIP_TYPE appears only after the reboot");
    }

    #[test]
    fn failsafe_options_that_lose_drops_are_named_and_fixed_bit_by_bit() {
        let releasing = Setup { fs_options: 32 | 8, ..copter(1) };
        assert_eq!(problems(&releasing, 1), vec![Problem::FailsafeDrops]);
        assert_eq!(Problem::FailsafeDrops.fix(&releasing), Some(("FS_OPTIONS", 8.0)), "only the Release Gripper bit is cleared");
        let homing = Setup { gcs_failsafe: true, fs_options: 1, ..copter(2) };
        assert_eq!(problems(&Setup { auto_close_s: 2.0, ..homing.clone() }, 1), vec![Problem::LinkLossSkipsDrops]);
        assert_eq!(Problem::LinkLossSkipsDrops.fix(&homing), Some(("FS_OPTIONS", 3.0)));
        assert!(!Problem::LinkLossSkipsDrops.blocks(), "flying home on a lost link is a choice, so it warns without blocking");
        assert_eq!(problems(&Setup { fs_options: 3, auto_close_s: 2.0, ..homing }, 1), vec![]);
    }

    #[test]
    fn px4_is_only_asked_for_a_gripper() {
        let px4 = Setup { ardupilot: false, enabled: true, loaded: false, output: true, auto_close_s: 0.0, fs_options: 32, gcs_failsafe: true, gcs_timeout_s: 5.0, drops: 4, early_drops: 0 };
        assert_eq!(problems(&px4, 4), vec![]);
        assert_eq!(problems(&Setup { enabled: false, ..px4 }, 4), vec![Problem::NoGripper]);
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
        let read = Setup::read(true, true, params, 3, 0);
        assert_eq!(read, Setup { ardupilot: true, enabled: true, loaded: true, output: true, auto_close_s: 2.0, fs_options: 2, gcs_failsafe: true, gcs_timeout_s: 5.0, drops: 3, early_drops: 0 });
        assert_eq!(Setup::from_json(&read.json()), read);
        assert!(!Setup::read(true, true, |name| (name == "GRIP_TYPE").then_some(1.0), 0, 0).output, "a servo gripper with no output assigned cannot move");
        assert!(Setup::read(true, true, |name| (name == "GRIP_TYPE").then_some(2.0), 0, 0).output, "an EPM needs no servo output");
    }

    #[test]
    fn a_drop_while_the_gripper_is_still_open_is_refused() {
        assert_eq!(open_refusal(&copter(0), None, 5_000), None, "never released");
        assert!(open_refusal(&copter(0), Some(1_000), 60_000).is_some_and(|r| r.starts_with("The gripper is still open")), "without auto-close it stays open");
        let closing = Setup { auto_close_s: 2.0, ..copter(0) };
        assert_eq!(open_refusal(&closing, Some(1_000), 2_100).as_deref(), Some("The gripper is still open from the last drop. It closes in 1 s."));
        assert_eq!(open_refusal(&closing, Some(1_000), 3_000), None);
        assert_eq!(open_refusal(&Setup { ardupilot: false, ..copter(0) }, Some(1_000), 1_001), None, "PX4 grippers are not modelled");
    }
}
