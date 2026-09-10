use serde_json::{Value, json};

pub const CMD_PREFLIGHT_CALIBRATION: u16 = 241;
pub const CMD_DO_START_MAG_CAL: u16 = 42424;
pub const CMD_DO_CANCEL_MAG_CAL: u16 = 42426;
pub const CMD_ACCELCAL_VEHICLE_POS: u16 = 42429;
pub const RESULT_ACCEPTED: u8 = 0;
pub const RESULT_IN_PROGRESS: u8 = 5;
pub const MAG_CAL_SUCCESS: u8 = 4;
pub const COMPASS_FITNESS_PARAM: &str = "COMPASS_CAL_FIT";
pub const COMPASS_LEARN_PARAM: &str = "COMPASS_LEARN";
const SUPPORTED_CAL_VERSION: u32 = 2;
const CAL_PREFIX: &str = "[cal] ";
const MAX_LOG_LINES: usize = 200;
const ACCEL_POS_SUCCESS: u32 = 16_777_215;
const ACCEL_POS_FAILED: u32 = 16_777_216;
const APM_HIDDEN_PREFIXES: &[&str] = &["prearm:", "ekf", "arm", "initialising"];

const HELP_PLACE: &str = "Place your vehicle into one of the Incomplete orientations shown below and hold it still";
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

const KINDS: &[(Kind, &str, &str)] = &[
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

    fn orientations(self) -> bool {
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
    compasses: [Compass; 3],
    compass_mask: u8,
    compass_fitness: Option<f64>,
    compass_learn: bool,
    mag_sides: u32,
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
            compasses: [Compass::default(); 3],
            compass_mask: 0,
            compass_fitness: None,
            compass_learn: false,
            mag_sides: 0b11_1111,
        }
    }

    pub fn running(&self) -> Option<Kind> {
        self.running
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

    fn begin(&mut self, kind: Kind) {
        self.running = Some(kind);
        self.last = Some(kind);
        self.outcome = None;
        self.waiting_for_cancel = false;
        self.unknown_firmware = false;
        self.progress = 0.0;
        self.next_enabled = false;
        self.help = "";
        self.log.clear();
        self.compasses = [Compass::default(); 3];
        self.reset_sides(0);
    }

    pub fn start(&mut self, kind: Kind, inputs: Inputs) -> Result<Vec<Action>, String> {
        if let Some(running) = self.running {
            return Err(format!("{} calibration is already running.", running.title()));
        }
        if !kind.supported(self.px4) {
            return Err(format!("{} calibration is not offered for this firmware.", kind.title()));
        }
        self.begin(kind);
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
                self.help = HELP_APM_ACCEL;
                self.next_enabled = true;
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
        let kind = self.running.take();
        self.outcome = Some(outcome);
        self.waiting_for_cancel = false;
        self.next_enabled = false;
        if !self.px4 {
            self.progress = if outcome == Outcome::Success { 1.0 } else { 0.0 };
        }
        if outcome == Outcome::Success {
            self.help = HELP_COMPLETE;
            self.sides.iter_mut().for_each(|side| *side = Side { stage: Stage::Done, rotate: false, visible: side.visible });
        }
        let restore = (kind == Some(Kind::Compass)).then(|| self.compass_fitness.take()).flatten().map(|value| Action::SetParam { name: COMPASS_FITNESS_PARAM, value });
        let learn = (!self.px4 && outcome == Outcome::Success && self.compass_learn).then_some(Action::SetParam { name: COMPASS_LEARN_PARAM, value: 0.0 });
        restore.into_iter().chain(learn).collect()
    }

    pub fn on_text(&mut self, raw: &str) -> Vec<Action> {
        if self.running.is_none() {
            return Vec::new();
        }
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
            self.help = HELP_PLACE;
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
        if kind.orientations() {
            let visible = if kind == Kind::Compass { self.mag_sides } else { u32::MAX };
            self.reset_sides(visible);
            self.help = HELP_PLACE;
        }
    }

    pub fn on_ack(&mut self, command: u16, result: u8) -> Vec<Action> {
        if self.px4 {
            return Vec::new();
        }
        match (self.running, command, result) {
            (Some(Kind::Compass), CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED) if self.compasses.iter().all(|c| !c.complete) => {
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
            (Some(Kind::Accelerometer), CMD_PREFLIGHT_CALIBRATION, _) => Vec::new(),
            (Some(_), CMD_PREFLIGHT_CALIBRATION, RESULT_IN_PROGRESS) => {
                self.note("In progress");
                Vec::new()
            }
            (Some(_), CMD_PREFLIGHT_CALIBRATION, RESULT_ACCEPTED) => {
                self.note("Successfully completed");
                self.stop(Outcome::Success)
            }
            (Some(_), CMD_PREFLIGHT_CALIBRATION, _) => {
                self.note("Failed");
                self.stop(Outcome::Failed)
            }
            _ => Vec::new(),
        }
    }

    pub fn on_mag_progress(&mut self, compass_id: u8, cal_mask: u8, completion_pct: u8) {
        if self.running != Some(Kind::Compass) {
            return;
        }
        let calibrating = cal_mask.count_ones() as u8;
        if compass_id < 3 && calibrating != 0 {
            self.compasses[compass_id as usize].progress = completion_pct / calibrating;
        }
        self.progress = self.compasses.iter().map(|c| c.progress as f64).sum::<f64>() / 100.0;
    }

    pub fn on_mag_report(&mut self, compass_id: u8, cal_status: u8, fitness: f64) -> Vec<Action> {
        if self.running != Some(Kind::Compass) || compass_id >= 3 {
            return Vec::new();
        }
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

    pub fn on_accel_position(&mut self, position: u32) -> Vec<Action> {
        if self.px4 || self.running != Some(Kind::Accelerometer) {
            return Vec::new();
        }
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
            (true, Some(_)) => !self.waiting_for_cancel,
            (false, Some(Kind::Compass)) => true,
            _ => false,
        };
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
            "showOrientations": running.is_some_and(|k| k.orientations() && (self.px4 || k == Kind::Accelerometer)),
            "usingLog": self.unknown_firmware,
            "sides": SIDES.iter().zip(self.sides.iter()).map(|((key, title, _, _), side)| json!({ "key": key, "title": title, "visible": side.visible, "stage": side.stage.name(), "rotate": side.rotate })).collect::<Vec<_>>(),
            "compasses": self.compasses.iter().enumerate().map(|(i, c)| json!({ "id": i, "progress": c.progress as f64 / 100.0, "complete": c.complete, "succeeded": c.succeeded, "fitness": c.fitness })).collect::<Vec<_>>(),
            "log": self.log,
            "routines": KINDS.iter().filter(|(k, _, _)| k.supported(self.px4)).map(|(k, id, title)| json!({ "id": id, "title": title, "enabled": running.is_none() && !self.waiting_for_cancel, "kind": k.id() })).collect::<Vec<_>>(),
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
        let started = cal.start(Kind::Accelerometer, Inputs::default()).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], show_error: false }]);
        assert!(cal.start(Kind::Gyro, Inputs::default()).is_err(), "one calibration at a time");
        cal.on_text("[cal] calibration started: 2 accel");
        assert!(cal.sides.iter().all(|s| s.visible && s.stage == Stage::Waiting));
        assert_eq!(cal.snapshot()["help"], HELP_PLACE);
        cal.on_text("[cal] down orientation detected");
        assert_eq!(stages(&cal), ["inProgress", "waiting", "waiting", "waiting", "waiting", "waiting"]);
        assert!(!cal.sides[0].rotate, "only compass asks to rotate");
        cal.on_text("[cal] progress &lt;40&gt;");
        assert_eq!(cal.progress, 0.4);
        cal.on_text("[cal] down side done, rotate to a different side");
        assert_eq!(stages(&cal)[0], "done");
        cal.on_text("[cal] down side already completed");
        assert_eq!(cal.snapshot()["help"], HELP_ALREADY);
        cal.on_text("[cal] front orientation detected");
        assert_eq!(stages(&cal)[4], "inProgress");
        assert!(cal.on_text("[cal] calibration done: accel").is_empty());
        assert_eq!(cal.running, None);
        assert_eq!(cal.outcome, Some(Outcome::Success));
        assert!(cal.sides.iter().all(|s| s.stage == Stage::Done));
        assert_eq!(cal.snapshot()["routines"].as_array().unwrap().len(), 5);
        assert!(cal.snapshot()["routines"][0]["enabled"].as_bool().unwrap());
    }

    #[test]
    fn px4_compass_shows_only_the_configured_sides_and_rotates() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Compass, Inputs { mag_sides: Some((1 << 5) | (1 << 2)), ..Inputs::default() }).unwrap();
        cal.on_text("[cal] calibration started: 2 mag");
        let visible: Vec<bool> = cal.sides.iter().map(|s| s.visible).collect();
        assert_eq!(visible, [true, false, true, false, false, false]);
        cal.on_text("[cal] left orientation detected");
        assert!(cal.sides[2].rotate);
        assert_eq!(cal.snapshot()["help"], HELP_ROTATE);
        assert!(cal.snapshot()["cancelEnabled"].as_bool().unwrap());
        let cancel = cal.cancel().unwrap();
        assert_eq!(cancel, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0; 7], show_error: true }]);
        assert!(cal.waiting_for_cancel);
        assert!(!cal.snapshot()["cancelEnabled"].as_bool().unwrap());
        cal.on_text("[cal] calibration cancelled");
        assert_eq!(cal.outcome, Some(Outcome::Cancelled));
        assert!(!cal.snapshot()["busy"].as_bool().unwrap());
    }

    #[test]
    fn px4_unsolicited_cancel_and_failure_are_failures() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Gyro, Inputs::default()).unwrap();
        cal.on_text("[cal] calibration started: 2 gyro");
        cal.on_text("[cal] calibration cancelled");
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        cal.start(Kind::LevelHorizon, Inputs::default()).unwrap();
        cal.on_text("[cal] calibration started: 2 level");
        assert!(!cal.snapshot()["showOrientations"].as_bool().unwrap());
        cal.on_text("[cal] calibration failed");
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        assert!(cal.on_text("[cal] calibration done: level").is_empty(), "texts outside a run are ignored");
        assert_eq!(cal.outcome, Some(Outcome::Failed));
    }

    #[test]
    fn px4_unknown_cal_version_falls_back_to_the_log() {
        let mut cal = Calibration::new(true);
        cal.start(Kind::Accelerometer, Inputs::default()).unwrap();
        cal.on_text("[cal] calibration started: 3 accel");
        assert!(cal.unknown_firmware);
        cal.on_text("[cal] down orientation detected");
        assert_eq!(stages(&cal)[0], "waiting", "an unknown protocol version only logs");
        assert_eq!(cal.log.len(), 3);
        assert!(cal.snapshot()["usingLog"].as_bool().unwrap());
        assert!(cal.start(Kind::Pressure, Inputs::default()).is_err(), "pressure is an ArduPilot routine");
    }

    #[test]
    fn apm_compass_cancels_then_starts_and_reports_per_compass() {
        let mut cal = Calibration::new(false);
        let inputs = Inputs { mag_sides: None, compass_mask: 0b011, compass_fitness: Some(30.0), compass_learn: true };
        let started = cal.start(Kind::Compass, inputs).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: false }]);
        let begun = cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED);
        assert_eq!(begun, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 100.0 }, Action::Command { command: CMD_DO_START_MAG_CAL, params: [3.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: true }]);
        assert!(cal.compasses[2].complete && cal.compasses[2].succeeded, "an absent compass counts as done");
        assert!(cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED).is_empty(), "a late duplicate ack does not restart");
        cal.on_mag_progress(0, 0b011, 50);
        cal.on_mag_progress(1, 0b011, 30);
        assert_eq!(cal.progress, 0.40);
        assert!(cal.on_mag_report(0, MAG_CAL_SUCCESS, 4.5).is_empty());
        assert_eq!(cal.log.last().unwrap(), "Continue rotating...");
        let done = cal.on_mag_report(1, MAG_CAL_SUCCESS, 6.0);
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
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, compass_fitness: Some(25.0), ..Inputs::default() }).unwrap();
        cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED);
        let failed = cal.on_mag_report(0, 5, 99.0);
        assert_eq!(failed, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 25.0 }]);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, compass_fitness: Some(25.0), ..Inputs::default() }).unwrap();
        let refused = cal.on_ack(CMD_DO_CANCEL_MAG_CAL, RESULT_ACCEPTED);
        assert_eq!(refused.len(), 2);
        let aborted = cal.on_ack(CMD_DO_START_MAG_CAL, 2);
        assert_eq!(aborted, vec![Action::SetParam { name: COMPASS_FITNESS_PARAM, value: 25.0 }], "a refused start restores the threshold");
        cal.start(Kind::Compass, Inputs { compass_mask: 0b001, ..Inputs::default() }).unwrap();
        let cancelled = cal.cancel().unwrap();
        assert_eq!(cancelled, vec![Action::Command { command: CMD_DO_CANCEL_MAG_CAL, params: [0.0; 7], show_error: true }]);
        assert_eq!(cal.outcome, Some(Outcome::Cancelled));
    }

    #[test]
    fn apm_accel_follows_the_vehicle_position_prompts_and_next_acks() {
        let mut cal = Calibration::new(false);
        assert!(cal.next().is_err());
        cal.start(Kind::Accelerometer, Inputs::default()).unwrap();
        assert_eq!(cal.snapshot()["help"], HELP_APM_ACCEL);
        assert!(cal.cancel().is_err(), "ArduPilot accel calibration has no cancel");
        cal.on_accel_position(1);
        assert_eq!(stages(&cal), ["inProgress", "waiting", "waiting", "waiting", "waiting", "waiting"]);
        assert_eq!(cal.next().unwrap(), vec![Action::Ack]);
        cal.on_accel_position(2);
        assert_eq!(stages(&cal), ["done", "waiting", "inProgress", "waiting", "waiting", "waiting"]);
        assert_eq!(cal.progress, 0.17);
        cal.on_accel_position(2);
        assert_eq!(cal.progress, 0.17, "a repeated prompt changes nothing");
        cal.on_accel_position(3);
        cal.on_accel_position(4);
        cal.on_accel_position(5);
        cal.on_accel_position(6);
        assert_eq!(stages(&cal), ["done", "inProgress", "done", "done", "done", "done"]);
        assert_eq!(cal.progress, 0.85);
        cal.on_text("PreArm: needs calibration");
        cal.on_text("Calibration successful");
        assert_eq!(cal.log, vec!["Calibration successful"], "prearm chatter stays out of the log");
        assert!(cal.on_accel_position(ACCEL_POS_SUCCESS).is_empty());
        assert_eq!(cal.outcome, Some(Outcome::Success));
        assert!(cal.sides.iter().all(|s| s.stage == Stage::Done));
        assert!(cal.next().is_err());
    }

    #[test]
    fn apm_simple_routines_finish_on_the_command_ack() {
        let mut cal = Calibration::new(false);
        let started = cal.start(Kind::Pressure, Inputs::default()).unwrap();
        assert_eq!(started, vec![Action::Command { command: CMD_PREFLIGHT_CALIBRATION, params: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], show_error: false }]);
        assert_eq!(cal.log, vec!["Requesting pressure calibration..."]);
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, RESULT_IN_PROGRESS);
        assert_eq!(cal.running, Some(Kind::Pressure));
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, RESULT_ACCEPTED);
        assert_eq!(cal.outcome, Some(Outcome::Success));
        cal.start(Kind::LevelHorizon, Inputs::default()).unwrap();
        cal.on_ack(CMD_PREFLIGHT_CALIBRATION, 4);
        assert_eq!(cal.outcome, Some(Outcome::Failed));
        assert!(cal.start(Kind::Airspeed, Inputs::default()).is_err(), "airspeed is a PX4 routine");
        assert_eq!(cal.snapshot()["routines"].as_array().unwrap().len(), 5);
    }
}
