use serde_json::{Value, json};

pub const CMD_PREFLIGHT_CALIBRATION: u16 = 241;
pub const CMD_DO_START_MAG_CAL: u16 = 42424;
pub const CMD_DO_CANCEL_MAG_CAL: u16 = 42426;
pub const CMD_ACCELCAL_VEHICLE_POS: u16 = 42429;
pub use crate::mavcmd::{RESULT_ACCEPTED, RESULT_IN_PROGRESS};

pub const MAG_CAL_SUCCESS: u8 = 4;
pub const COMPASS_COUNT: usize = 3;
pub const STALL_TIMEOUT_MS: u64 = 10 * 60 * 1000;
pub const COMPASS_FITNESS_PARAM: &str = "COMPASS_CAL_FIT";
pub const COMPASS_LEARN_PARAM: &str = "COMPASS_LEARN";
const SUPPORTED_CAL_VERSION: u32 = 2;
const CAL_PREFIX: &str = "[cal] ";
const MAX_LOG_LINES: usize = 200;
const ACCEL_POS_SUCCESS: u32 = 16_777_215;
const ACCEL_POS_FAILED: u32 = 16_777_216;
const APM_HIDDEN_PREFIXES: &[&str] = &["prearm:", "ekf", "arm", "initialising"];

const HELP_PLACE: &str = "Place your vehicle into one of the Incomplete orientations shown below and hold it still";
const HELP_PLACE_AGAIN: &str = "Place your vehicle into one of the orientations shown below and hold it still";
const HELP_ROTATE: &str = "Rotate the vehicle continuously as shown in the diagram until marked as Completed";
const HELP_HOLD: &str = "Hold still in the current orientation";
const HELP_ALREADY: &str = "Orientation already completed, place your vehicle into one of the incomplete orientations shown below and hold it still";
const HELP_APM_ACCEL: &str = "Hold still in the current orientation and press Next when ready";
const HELP_APM_COMPASS: &str = "Rotate the vehicle randomly around all axes until the progress bar fills all the way to the right.";
const HELP_COMPLETE: &str = "Calibration complete";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Accelerometer,
    Compass,
    Gyro,
    LevelHorizon,
    Pressure,
    Airspeed,
}

pub const KINDS: &[(Kind, &str, &str)] = &[
    (Kind::Accelerometer, "accelerometer", "Accelerometer"),
    (Kind::Compass, "compass", "Compass"),
    (Kind::LevelHorizon, "levelHorizon", "Level Horizon"),
    (Kind::Gyro, "gyro", "Gyro"),
    (Kind::Pressure, "pressure", "Pressure"),
    (Kind::Airspeed, "airspeed", "Airspeed"),
];

impl Kind {
    pub fn id(self) -> &'static str {
        KINDS.iter().find(|(k, _, _)| *k == self).map(|(_, id, _)| *id).unwrap_or("")
    }

    fn title(self) -> &'static str {
        KINDS.iter().find(|(k, _, _)| *k == self).map(|(_, _, title)| *title).unwrap_or("")
    }

    pub fn parse(id: &str) -> Option<Kind> {
        KINDS.iter().find(|(_, key, _)| *key == id).map(|(k, _, _)| *k)
    }

    fn supported(self, px4: bool) -> bool {
        match self {
            Kind::Airspeed => px4,
            Kind::Pressure => !px4,
            _ => true,
        }
    }

    fn params(self) -> [f64; 7] {
        match self {
            Kind::Gyro => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            Kind::Compass => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            Kind::Pressure => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            Kind::Accelerometer => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            Kind::LevelHorizon => [0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0],
            Kind::Airspeed => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }
    }

    pub fn orientations(self) -> bool {
        matches!(self, Kind::Accelerometer | Kind::Compass | Kind::Gyro)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Waiting,
    InProgress,
    Done,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Waiting => "waiting",
            Stage::InProgress => "inProgress",
            Stage::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Cancelled,
    Failed,
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Cancelled => "cancelled",
            Outcome::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Command { command: u16, params: [f64; 7], show_error: bool },
    SetParam { name: &'static str, value: f64 },
    Ack,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Inputs {
    pub mag_sides: Option<u32>,
    pub compass_mask: u8,
    pub compass_fitness: Option<f64>,
    pub compass_learn: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Side {
    stage: Stage,
    rotate: bool,
    visible: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Compass {
    progress: u8,
    complete: bool,
    succeeded: bool,
    fitness: f64,
}

const SIDE_COUNT: usize = 6;
const SIDES: [(&str, &str, &str, u32); SIDE_COUNT] = [
    ("Down", "Level", "down", 1 << 5),
    ("UpsideDown", "Upside down", "up", 1 << 4),
    ("Left", "Left side", "left", 1 << 2),
    ("Right", "Right side", "right", 1 << 3),
    ("NoseDown", "Nose down", "front", 1 << 1),
    ("TailDown", "Tail down", "back", 1 << 0),
];
const ACCEL_POSITIONS: [(u32, usize, f64); SIDE_COUNT] = [(1, 0, 0.0), (2, 2, 0.17), (3, 3, 0.34), (4, 4, 0.51), (5, 5, 0.68), (6, 1, 0.85)];

#[derive(Debug)]
pub struct Calibration {
    px4: bool,
    running: Option<Kind>,
    last: Option<Kind>,
    waiting_for_cancel: bool,
    unknown_firmware: bool,
    sides: [Side; SIDE_COUNT],
    progress: f64,
    outcome: Option<Outcome>,
    help: &'static str,
    log: Vec<String>,
    next_enabled: bool,
    compasses: [Compass; COMPASS_COUNT],
    compass_mask: u8,
    compass_fitness: Option<f64>,
    compass_learn: bool,
    mag_sides: u32,
    mag_cal_started: bool,
    visual: bool,
    active_ms: Option<u64>,
}

fn side_index(name: &str) -> Option<usize> {
    SIDES.iter().position(|(_, _, key, _)| *key == name)
}

impl Calibration {
    pub fn new(px4: bool) -> Calibration {
        Calibration {
            px4,
            running: None,
            last: None,
            waiting_for_cancel: false,
            unknown_firmware: false,
            sides: [Side::default(); SIDE_COUNT],
            progress: 0.0,
            outcome: None,
            help: "",
            log: Vec::new(),
            next_enabled: false,
            compasses: [Compass::default(); COMPASS_COUNT],
            compass_mask: 0,
            compass_fitness: None,
            compass_learn: false,
            mag_sides: 0b11_1111,
            mag_cal_started: false,
            visual: false,
            active_ms: None,
        }
    }

    fn note(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > MAX_LOG_LINES {
            self.log.remove(0);
        }
    }

    fn reset_sides(&mut self, visible: u32) {
        self.sides = std::array::from_fn(|i| Side { visible: visible & SIDES[i].3 != 0, ..Side::default() });
    }

    fn begin(&mut self, kind: Kind, now_ms: u64) {
        self.running = Some(kind);
        self.last = Some(kind);
        self.outcome = None;
        self.waiting_for_cancel = false;
        self.unknown_firmware = false;
        self.progress = 0.0;
        self.next_enabled = false;
        self.mag_cal_started = false;
        self.visual = false;
        self.active_ms = Some(now_ms);
        self.help = "";
        self.log.clear();
        self.compasses = [Compass::default(); COMPASS_COUNT];
        self.reset_sides(0);
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<Action> {
        let stalled = self.active_ms.is_some_and(|since| now_ms.saturating_sub(since) >= STALL_TIMEOUT_MS);
        if !stalled || self.running.is_none() {
            return Vec::new();
        }
        self.note("The vehicle stopped answering during the calibration");
        self.stop(Outcome::Failed)
    }

    pub fn start(&mut self, kind: Kind, inputs: Inputs, now_ms: u64) -> Result<Vec<Action>, String> {
        if let Some(running) = self.running {
            return Err(format!("{} calibration is already running.", running.title()));
        }
        if !kind.supported(self.px4) {
            return Err(format!("{} calibration is not offered for this firmware.", kind.title()));
        }
        self.begin(kind, now_ms);
        self.mag_sides = inputs.mag_sides.unwrap_or(0b11_1111);
        self.compass_mask = inputs.compass_mask;
        self.compass_fitness = inputs.compass_fitness;
        self.compass_learn = inputs.compass_learn;
        if self.px4 {
            return Ok(vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: kind.params(), show_error: false }]);
        }
        match kind {
            Kind::Compass => Ok(vec![Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: false }]),
            Kind::Accelerometer => {
                self.reset_sides(u32::MAX);
                self.visual = true;
                self.help = HELP_APM_ACCEL;
                Ok(vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: kind.params(), show_error: false }])
            }
            Kind::LevelHorizon => {
                self.note("Hold the vehicle in its level flight position.");
                Ok(vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: kind.params(), show_error: false }])
            }
            other => {
                self.note(format!("Requesting {} calibration...", other.title().to_lowercase()));
                Ok(vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: other.params(), show_error: false }])
            }
        }
    }

    pub fn cancel(&mut self) -> Result<Vec<Action>, String> {
        let Some(running) = self.running else { return Err("No calibration is running.".to_string()) };
        if self.px4 {
            self.waiting_for_cancel = true;
            return Ok(vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0; 7], show_error: true }]);
        }
        if running != Kind::Compass {
            return Err(format!("{} calibration cannot be cancelled once started.", running.title()));
        }
        let stopped = self.stop(Outcome::Cancelled);
        Ok(std::iter::once(Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: true }).chain(stopped).collect())
    }

    pub fn next(&mut self) -> Result<Vec<Action>, String> {
        match (self.px4, self.running, self.next_enabled) {
            (false, Some(Kind::Accelerometer), true) => Ok(vec![Action::Ack]),
            _ => Err("Nothing is waiting for the next step.".to_string()),
        }
    }

    fn stop(&mut self, outcome: Outcome) -> Vec<Action> {
        self.running = None;
        self.outcome = Some(outcome);
        self.waiting_for_cancel = false;
        self.next_enabled = false;
        self.active_ms = None;
        self.progress = if outcome == Outcome::Success { 1.0 } else { 0.0 };
        if outcome == Outcome::Success {
            self.help = HELP_COMPLETE;
            let sides = self.sides;
            self.sides = std::array::from_fn(|i| Side { stage: Stage::Done, rotate: false, visible: sides[i].visible });
        }
        let restore = std::mem::take(&mut self.mag_cal_started).then(|| self.compass_fitness.take()).flatten().map(|value| Action::SetParam { name: COMPASS_FITNESS_PARAM, value });
        let learn = (!self.px4 && outcome == Outcome::Success && self.compass_learn).then_some(Action::SetParam { name: COMPASS_LEARN_PARAM, value: 0.0 });
        restore.into_iter().chain(learn).collect()
    }

    pub fn on_text(&mut self, raw: &str, now_ms: u64) -> Vec<Action> {
        if self.running.is_none() {
            return Vec::new();
        }
        self.active_ms = Some(now_ms);
        if self.px4 { self.on_px4_text(&raw.replace("&lt;", "<").replace("&gt;", ">")) } else { self.on_apm_text(raw) }
    }

    fn on_apm_text(&mut self, text: &str) -> Vec<Action> {
        let lower = text.to_lowercase();
        if !APM_HIDDEN_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
            self.note(text);
        }
        Vec::new()
    }

    fn on_px4_text(&mut self, text: &str) -> Vec<Action> {
        if let Some(percent) = text.split_once("progress <").and_then(|(_, rest)| rest.split('>').next()).and_then(|p| p.trim().parse::<u32>().ok()) {
            self.progress = percent.min(100) as f64 / 100.0;
            return Vec::new();
        }
        self.note(text);
        if self.unknown_firmware {
            return Vec::new();
        }
        let Some(cal) = text.strip_prefix(CAL_PREFIX) else { return Vec::new() };
        if let Some(started) = cal.strip_prefix("calibration started: ") {
            self.on_started(started);
            return Vec::new();
        }
        if let Some(side) = cal.strip_suffix(" orientation detected").and_then(side_index) {
            let rotate = self.running == Some(Kind::Compass);
            self.sides[side] = Side { stage: Stage::InProgress, rotate, visible: self.sides[side].visible };
            self.help = if rotate { HELP_ROTATE } else { HELP_HOLD };
            return Vec::new();
        }
        if let Some(side) = cal.strip_suffix(" side done, rotate to a different side").and_then(side_index) {
            self.sides[side] = Side { stage: Stage::Done, rotate: false, visible: self.sides[side].visible };
            self.help = HELP_PLACE_AGAIN;
            return Vec::new();
        }
        if cal.ends_with("side already completed") {
            self.help = HELP_ALREADY;
            return Vec::new();
        }
        if cal.starts_with("calibration done:") {
            return self.stop(Outcome::Success);
        }
        if cal.starts_with("calibration cancelled") {
            let outcome = if self.waiting_for_cancel { Outcome::Cancelled } else { Outcome::Failed };
            return self.stop(outcome);
        }
        if cal.starts_with("calibration failed") {
            return self.stop(Outcome::Failed);
        }
        Vec::new()
    }

    fn on_started(&mut self, rest: &str) {
        let mut parts = rest.split_whitespace();
        let version = parts.next().and_then(|v| v.parse::<u32>().ok());
        let what = parts.next().unwrap_or("");
        if version != Some(SUPPORTED_CAL_VERSION) {
            self.unknown_firmware = true;
            self.note("Unsupported calibration firmware version, using log");
            return;
        }
        let kind = match what {
            "accel" => Some(Kind::Accelerometer),
            "mag" => Some(Kind::Compass),
            "gyro" => Some(Kind::Gyro),
            "airspeed" => Some(Kind::Airspeed),
            "level" => Some(Kind::LevelHorizon),
            _ => None,
        };
        let Some(kind) = kind else { return };
        self.running = Some(kind);
        self.last = Some(kind);
        self.progress = 0.0;
        self.visual = true;
        if kind.orientations() {
            self.reset_sides(match kind {
                Kind::Compass => self.mag_sides,
                Kind::Gyro => SIDES[0].3,
                _ => u32::MAX,
            });
            self.help = HELP_PLACE;
        }
    }

    pub fn on_ack(&mut self, command: u16, result: u8, now_ms: u64) -> Vec<Action> {
        if self.px4 || self.running.is_none() {
            return Vec::new();
        }
        self.active_ms = Some(now_ms);
        match (self.running, command, result) {
            (Some(Kind::Compass), CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED) if !self.mag_cal_started => {
                self.mag_cal_started = true;
                self.visual = true;
                self.compasses = std::array::from_fn(|i| if self.compass_mask & (1 << i) != 0 { Compass::default() } else { Compass { complete: true, succeeded: true, ..Compass::default() } });
                self.help = HELP_APM_COMPASS;
                self.note(HELP_APM_COMPASS);
                let fitness = self.compass_fitness.map(|_| Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 100.0 });
                let start = Action::Command { command: CMD_DO_START_MAG_CAL, params: [self.compass_mask as f64, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: true };
                fitness.into_iter().chain(std::iter::once(start)).collect()
            }
            (Some(Kind::Compass), CMD_DO_START_MAG_CAL, result) if result != RESULT_ACCEPTED => {
                self.note("Compass calibration could not start");
                self.stop(Outcome::Failed)
            }
            (Some(Kind::Gyro | Kind::LevelHorizon | Kind::Pressure), CMD_PREFLIGHT_CALIBRATION, RESULT_IN_PROGRESS) => {
                self.note("In progress");
                Vec::new()
            }
            (Some(Kind::Gyro | Kind::LevelHorizon | Kind::Pressure), CMD_PREFLIGHT_CALIBRATION, RESULT_ACCEPTED) => {
                self.note("Successfully completed");
                self.stop(Outcome::Success)
            }
            (Some(Kind::Gyro | Kind::LevelHorizon | Kind::Pressure), CMD_PREFLIGHT_CALIBRATION, _) => {
                self.note("Failed");
                self.stop(Outcome::Failed)
            }
            _ => Vec::new(),
        }
    }

    pub fn on_mag_progress(&mut self, compass_id: u8, cal_mask: u8, completion_pct: u8, now_ms: u64) {
        if self.running != Some(Kind::Compass) {
            return;
        }
        self.active_ms = Some(now_ms);
        let calibrating = (cal_mask & 0b111).count_ones() as u8;
        if (compass_id as usize) < COMPASS_COUNT && calibrating != 0 {
            self.compasses[compass_id as usize].progress = completion_pct / calibrating;
        }
        self.progress = self.compasses.iter().map(|c| c.progress as f64).sum::<f64>() / 100.0;
    }

    pub fn on_mag_report(&mut self, compass_id: u8, cal_status: u8, fitness: f64, now_ms: u64) -> Vec<Action> {
        if self.running != Some(Kind::Compass) || compass_id as usize >= COMPASS_COUNT {
            return Vec::new();
        }
        self.active_ms = Some(now_ms);
        let index = compass_id as usize;
        let succeeded = cal_status == MAG_CAL_SUCCESS;
        if !self.compasses[index].complete {
            self.note(if succeeded { format!("Compass {compass_id} calibration complete") } else { format!("Compass {compass_id} calibration below quality threshold") });
            self.compasses[index] = Compass { progress: self.compasses[index].progress, complete: true, succeeded, fitness };
        }
        if !self.compasses.iter().all(|c| c.complete) {
            self.note("Continue rotating...");
            return Vec::new();
        }
        if self.compasses.iter().all(|c| c.succeeded) {
            self.note("All compasses calibrated successfully");
            self.note("YOU MUST REBOOT YOUR VEHICLE NOW FOR NEW SETTINGS TO TAKE AFFECT");
            self.stop(Outcome::Success)
        } else {
            self.note("Compass calibration failed");
            self.note("YOU MUST REBOOT YOUR VEHICLE NOW AND RETRY COMPASS CALIBRATION PRIOR TO FLIGHT");
            self.stop(Outcome::Failed)
        }
    }

    pub fn on_accel_position(&mut self, position: u32, now_ms: u64) -> Vec<Action> {
        if self.px4 || self.running != Some(Kind::Accelerometer) {
            return Vec::new();
        }
        self.active_ms = Some(now_ms);
        match position {
            ACCEL_POS_SUCCESS => self.stop(Outcome::Success),
            ACCEL_POS_FAILED => self.stop(Outcome::Failed),
            other => {
                let Some(step) = ACCEL_POSITIONS.iter().position(|(code, _, _)| *code == other) else { return Vec::new() };
                let (_, side, progress) = ACCEL_POSITIONS[step];
                if self.sides[side].stage == Stage::InProgress {
                    return Vec::new();
                }
                if let Some(&(_, previous, _)) = step.checked_sub(1).map(|i| &ACCEL_POSITIONS[i]) {
                    self.sides[previous] = Side { stage: Stage::Done, rotate: false, visible: true };
                }
                self.sides[side] = Side { stage: Stage::InProgress, rotate: false, visible: true };
                self.progress = progress;
                self.next_enabled = true;
                Vec::new()
            }
        }
    }

    pub fn snapshot(&self) -> Value {
        let running = self.running;
        let cancel_enabled = match (self.px4, running) {
            (true, Some(_)) => self.visual && !self.waiting_for_cancel,
            (false, Some(Kind::Compass)) => self.mag_cal_started,
            _ => false,
        };
        let shown = running.or(if self.outcome == Some(Outcome::Success) { self.last } else { None });
        json!({
            "running": running.map(Kind::id),
            "last": self.last.map(Kind::id),
            "busy": running.is_some() || self.waiting_for_cancel,
            "waitingForCancel": self.waiting_for_cancel,
            "progress": self.progress,
            "outcome": self.outcome.map(Outcome::name),
            "help": self.help,
            "nextEnabled": self.next_enabled,
            "cancelEnabled": cancel_enabled,
            "showOrientations": self.visual && shown.is_some_and(Kind::orientations),
            "usingLog": self.unknown_firmware,
            "sides": SIDES.iter().zip(self.sides.iter()).map(|((key, title, _, _), side)| json!({ "key": key, "title": title, "visible": side.visible, "stage": side.stage.name(), "rotate": side.rotate })).collect::<Vec<_>>(),
            "compasses": self.compasses.iter().enumerate().map(|(i, c)| json!({ "id": i, "progress": c.progress as f64 / 100.0, "complete": c.complete, "succeeded": c.succeeded, "fitness": c.fitness })).collect::<Vec<_>>(),
            "log": self.log,
            "routines": KINDS.iter().filter(|(k, _, _)| k.supported(self.px4)).map(|(_, id, title)| json!({ "id": id, "title": title, "enabled": running.is_none() && !self.waiting_for_cancel })).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stages(cal: &Calibration) -> Vec<&'static str> {
        cal.sides.iter().map(|s| s.stage.name()).collect()
    }

    #[test]
    fn px4_accel_walks_the_sides_from_the_cal_texts() {
        let mut cal = Calibration::new(true);
        let started = cal.start(Kind::Accelerometer, Inputs::default(), 0).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], show_error: false }]);
        assert!(cal.start(Kind::Gyro, Inputs::default(), 0).is_err(), "one calibration at a time");
        cal.on_text("[cal] calibration started: 2 accel", 0);
        assert!(cal.sides.iter().all(|s| s.visible && s.stage == Stage::Waiting));
        assert_eq!(cal.snapshot()["help"], HELP_PLACE);
        // The wire format carries the prefix - MockLink emits "[cal] calibration started: 2 gyro"
        // - and SensorsComponentController strips "[cal] " before reading the side off the first
        // word. This parser does the same, so the fixture is the text a vehicle actually sends.
        cal.on_text("[cal] down orientation detected", 0);
        assert_eq!(stages(&cal), ["inProgress", "waiting", "waiting", "waiting", "waiting", "waiting"]);
        assert!(!cal.sides[0].rotate, "only compass asks to rotate");
        cal.on_text("[cal] progress &lt;40&gt;", 0);
        assert_eq!(cal.progress, 0.4);
        cal.on_text("[cal] down side done, rotate to a different side", 0);
        assert_eq!(stages(&cal)[0], "done");
        cal.on_text("[cal] down side already completed", 0);
        assert_eq!(cal.snapshot()["help"], HELP_ALREADY);
        cal.on_text("[cal] front orientation detected", 0);
        assert_eq!(stages(&cal)[4], "inProgress");
        assert!(cal.on_text("[cal] calibration done: accel", 0).is_empty());
        assert_eq!(cal.running, None);
        assert_eq!(cal.outcome, Some(Outcome::Success));
        assert!(cal.sides.iter().all(|s| s.stage == Stage::Done));
        assert_eq!(cal.snapshot()["routines"].as_array().unwrap().len(), 5);
        assert!(cal.snapshot()["routines"][0]["enabled"].as_bool().unwrap());
    }

    #[test]
    fn px4_compass_shows_only_the_configured_sides_and_rotates() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Compass, Inputs { mag_sides: Some((1 << 5) | (1 << 2)), ..Inputs::default() }, 0).unwrap();
        cal.on_text("[cal] calibration started: 2 mag", 0);
        let visible: Vec<bool> = cal.sides.iter().map(|s| s.visible).collect();
        assert_eq!(visible, [true, false, true, false, false, false]);
        cal.on_text("[cal] left orientation detected", 0);
        assert!(cal.sides[2].rotate);
        assert_eq!(cal.snapshot()["help"], HELP_ROTATE);
        assert!(cal.snapshot()["cancelEnabled"].as_bool().unwrap());
        // busy was only ever asserted false, so pinning it to false passed: a panel that never
        // showed a calibration in progress, on the one screen whose whole job is to show that.
        assert!(cal.snapshot()["busy"].as_bool().unwrap(), "a calibration mid-routine is the definition of busy");
        let cancel = cal.cancel().unwrap();
        assert_eq!(cancel, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0; 7], show_error: true }]);
        assert!(cal.waiting_for_cancel);
        assert!(!cal.snapshot()["cancelEnabled"].as_bool().unwrap());
        assert!(cal.snapshot()["busy"].as_bool().unwrap(), "a cancel the vehicle has not acknowledged is still a routine in progress, which is the second half of the rule");
        cal.on_text("[cal] calibration cancelled", 0);
        assert_eq!(cal.outcome, Some(Outcome::Cancelled));
        assert!(!cal.snapshot()["busy"].as_bool().unwrap());
    }

    #[test]
    fn px4_unsolicited_cancel_and_failure_are_failures() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Gyro, Inputs::default(), 0).unwrap();
        cal.on_text("[cal] calibration started: 2 gyro", 0);
        cal.on_text("[cal] calibration cancelled", 0);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        cal.start(Kind::LevelHorizon, Inputs::default(), 0).unwrap();
        cal.on_text("[cal] calibration started: 2 level", 0);
        assert!(!cal.snapshot()["showOrientations"].as_bool().unwrap());
        cal.on_text("[cal] calibration failed", 0);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        assert!(cal.on_text("[cal] calibration done: level", 0).is_empty(), "texts outside a run are ignored");
        assert_eq!(cal.outcome, Some(Outcome::Failed));
    }

    #[test]
    fn px4_unknown_cal_version_falls_back_to_the_log() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Accelerometer, Inputs::default(), 0).unwrap();
        cal.on_text("[cal] calibration started: 3 accel", 0);
        assert!(cal.unknown_firmware);
        cal.on_text("[cal] down orientation detected", 0);
        assert_eq!(stages(&cal)[0], "waiting", "an unknown protocol version only logs");
        assert_eq!(cal.log.len(), 3);
        assert!(cal.snapshot()["usingLog"].as_bool().unwrap());
        assert!(cal.start(Kind::Pressure, Inputs::default(), 0).is_err(), "pressure is an ArduPilot routine");
    }

    #[test]
    fn apm_compass_cancels_then_starts_and_reports_per_compass() {
        let mut cal = Calibration::new(false);
        let inputs = Inputs { mag_sides: None, compass_mask: 0b111, compass_fitness: Some(30.0), compass_learn: true };
        let started = cal.start(Kind::Compass, inputs, 0).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: false }]);
        let begun = cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0);
        assert_eq!(begun, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 100.0 }, Action::Command { command: CMD_DO_START_MAG_CAL, params: [7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: true }]);
        assert!(cal.compasses.iter().all(|c| !c.complete), "every configured compass starts incomplete, which is why the guard cannot be the completion flags");
        assert!(cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0).is_empty(), "a late duplicate ack does not restart");
        assert!(cal.on_ack(CMD_PREFLIGHT_CALIBRATION, RESULT_ACCEPTED, 0).is_empty(), "an ack for a command the compass flow never sent does not end it");
        assert_eq!(cal.running, Some(Kind::Compass));
        cal.on_mag_progress(0, 0b111, 60, 0);
        cal.on_mag_progress(1, 0b111, 30, 0);
        assert_eq!(cal.progress, 0.30);
        cal.on_mag_progress(2, 0b1111, 30, 0);
        assert_eq!(cal.progress, 0.40, "only the three compass bits divide the progress, whatever else the firmware sets");
        assert!(cal.on_mag_report(0, MAG_CAL_SUCCESS, 4.5, 0).is_empty());
        assert_eq!(cal.log.last().unwrap(), "Continue rotating...");
        assert!(cal.on_mag_report(2, MAG_CAL_SUCCESS, 5.0, 0).is_empty());
        let done = cal.on_mag_report(1, MAG_CAL_SUCCESS, 6.0, 0);
        assert_eq!(done, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 30.0 }, Action::SetParam { name: COMPASS_LEARN_PARAM, value: 0.0 }]);
        assert_eq!(cal.outcome, Some(Outcome::Success));
        assert_eq!(cal.progress, 1.0);
        let snapshot = cal.snapshot();
        assert_eq!(snapshot["compasses"][1]["fitness"], 6.0);
        assert!(snapshot["log"].as_array().unwrap().iter().any(|l| l.as_str().unwrap().contains("REBOOT")));
    }

    #[test]
    fn apm_compass_below_threshold_fails_and_restores_fitness() {
        let mut cal = Calibration::new(false);
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, compass_fitness: Some(25.0), ..Inputs::default() }, 0).unwrap();
        cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0);
        let failed = cal.on_mag_report(0, 5, 99.0, 0);
        assert_eq!(failed, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 25.0 }]);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, compass_fitness: Some(25.0), ..Inputs::default() }, 0).unwrap();
        assert!(!cal.snapshot()["cancelEnabled"].as_bool().unwrap(), "cancel waits for the vehicle to accept the mag cal");
        let refused = cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0);
        assert_eq!(refused.len(), 2);
        assert!(cal.snapshot()["cancelEnabled"].as_bool().unwrap());
        let aborted = cal.on_ack(CMD_DO_START_MAG_CAL, 2, 0);
        assert_eq!(aborted, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 25.0 }], "a refused start restores the threshold");
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, ..Inputs::default() }, 0).unwrap();
        cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0);
        let cancelled = cal.cancel().unwrap();
        assert_eq!(cancelled, vec![Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: true }]);
        assert_eq!(cal.outcome, Some(Outcome::Cancelled));
    }

    #[test]
    fn apm_accel_follows_the_vehicle_position_prompts_and_next_acks() {
        let mut cal = Calibration::new(false);
        assert!(cal.next().is_err());
        cal.start(Kind::Accelerometer, Inputs::default(), 0).unwrap();
        assert_eq!(cal.snapshot()["help"], HELP_APM_ACCEL);
        assert!(cal.cancel().is_err(), "ArduPilot accel calibration has no cancel");
        assert!(cal.next().is_err(), "next waits for the vehicle to ask for the first position");
        cal.on_accel_position(1, 0);
        assert_eq!(stages(&cal), ["inProgress", "waiting", "waiting", "waiting", "waiting", "waiting"]);
        assert_eq!(cal.next().unwrap(), vec![Action::Ack]);
        cal.on_accel_position(2, 0);
        assert_eq!(stages(&cal), ["done", "waiting", "inProgress", "waiting", "waiting", "waiting"]);
        assert_eq!(cal.progress, 0.17);
        cal.on_accel_position(2, 0);
        assert_eq!(cal.progress, 0.17, "a repeated prompt changes nothing");
        cal.on_accel_position(3, 0);
        cal.on_accel_position(4, 0);
        cal.on_accel_position(5, 0);
        cal.on_accel_position(6, 0);
        assert_eq!(stages(&cal), ["done", "inProgress", "done", "done", "done", "done"]);
        assert_eq!(cal.progress, 0.85);
        cal.on_text("PreArm: needs calibration", 0);
        cal.on_text("Calibration successful", 0);
        assert_eq!(cal.log, vec!["Calibration successful"], "prearm chatter stays out of the log");
        assert!(cal.on_accel_position(ACCEL_POS_SUCCESS, 0).is_empty());
        assert_eq!(cal.outcome, Some(Outcome::Success));
        assert!(cal.sides.iter().all(|s| s.stage == Stage::Done));
        assert!(cal.snapshot()["showOrientations"].as_bool().unwrap(), "the finished grid stays on screen, as the Qt view leaves it");
        assert!(cal.next().is_err());
        assert!(cal.on_accel_position(3, 0).is_empty(), "a prompt after the run ended changes nothing");
        assert_eq!(cal.outcome, Some(Outcome::Success));
    }

    #[test]
    fn a_px4_gyro_asks_for_one_orientation_only() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Gyro, Inputs::default(), 0).unwrap();
        assert!(!cal.snapshot()["showOrientations"].as_bool().unwrap(), "nothing is shown before the vehicle says the calibration started");
        cal.on_text("[cal] calibration started: 2 gyro", 0);
        let visible: Vec<bool> = cal.sides.iter().map(|s| s.visible).collect();
        assert_eq!(visible, [true, false, false, false, false, false], "a gyro calibration is held level, never rotated through six sides");
        assert!(cal.snapshot()["showOrientations"].as_bool().unwrap());
        cal.on_text("[cal] progress <40>", 0);
        assert_eq!(cal.progress, 0.4);
        cal.on_text("[cal] calibration failed", 0);
        assert_eq!(cal.progress, 0.0, "a failed run does not leave the bar where it stopped");
    }

    #[test]
    fn an_apm_accel_prompt_closes_the_side_before_it() {
        let mut cal = Calibration::new(false);
        cal.start(Kind::Accelerometer, Inputs::default(), 0).unwrap();
        cal.on_accel_position(4, 0);
        assert_eq!(stages(&cal), ["waiting", "waiting", "waiting", "done", "inProgress", "waiting"], "each prompt closes the side before it in ArduPilot's order, which is what the Qt view does");
        cal.on_accel_position(1, 0);
        assert_eq!(stages(&cal)[0], "inProgress", "the level prompt has no side before it to close");
        assert_eq!(stages(&cal)[4], "inProgress");
    }

    #[test]
    fn a_run_the_vehicle_abandons_stops_itself() {
        let mut cal = Calibration::new(false);
        cal.start(Kind::Accelerometer, Inputs::default(), 1_000).unwrap();
        assert!(cal.tick(1_000 + STALL_TIMEOUT_MS - 1).is_empty());
        cal.on_accel_position(1, 5_000);
        assert!(cal.tick(5_000 + STALL_TIMEOUT_MS - 1).is_empty(), "every answer from the vehicle restarts the clock");
        assert!(cal.tick(5_000 + STALL_TIMEOUT_MS).is_empty(), "an accelerometer run writes no parameters when it gives up");
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        assert_eq!(cal.log.last().unwrap(), "The vehicle stopped answering during the calibration");
        assert!(cal.start(Kind::Accelerometer, Inputs::default(), 9_000).is_ok(), "giving up frees the vehicle for another try");
        assert!(cal.tick(9_000).is_empty());
    }

    #[test]
    fn an_apm_compass_with_nothing_configured_finishes_on_the_first_report() {
        let mut cal = Calibration::new(false);
        cal.start(Kind::Compass, Inputs { compass_mask: 0, ..Inputs::default() }, 0).unwrap();
        let begun = cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED, 0);
        assert_eq!(begun, vec![Action::Command { command: CMD_DO_START_MAG_CAL, params: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: true }], "with no fitness parameter there is nothing to bump");
        assert!(cal.compasses.iter().all(|c| c.complete && c.succeeded));
        let done = cal.on_mag_report(0, MAG_CAL_SUCCESS, 3.0, 0);
        assert!(done.is_empty(), "no fitness was bumped, so nothing is restored");
        assert_eq!(cal.outcome, Some(Outcome::Success));
    }

    #[test]
    fn apm_simple_routines_finish_on_the_command_ack() {
        let mut cal = Calibration::new(false);
        let started = cal.start(Kind::Pressure, Inputs::default(), 0).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: false }]);
        assert_eq!(cal.log, vec!["Requesting pressure calibration..."]);
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, RESULT_IN_PROGRESS, 0);
        assert_eq!(cal.running, Some(Kind::Pressure));
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, RESULT_ACCEPTED, 0);
        assert_eq!(cal.outcome, Some(Outcome::Success));
        cal.start(Kind::LevelHorizon, Inputs::default(), 0).unwrap();
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, 4, 0);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        assert!(cal.start(Kind::Airspeed, Inputs::default(), 0).is_err(), "airspeed is a PX4 routine");
        assert_eq!(cal.snapshot()["routines"].as_array().unwrap().len(), 5);
    }
}
