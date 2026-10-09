use mavlink::dialects::ardupilotmega::*;

use crate::mocklink::{AUTOPILOT, Out};

pub type Stored = &'static [(&'static str, f64)];

const SIDE_NAMES: [&str; 6] = ["back", "front", "left", "right", "up", "down"];
const SIDE_RESULTS: [&str; 6] = ["[9.810 0.000 0.000]", "[-9.810 0.000 0.000]", "[0.000 9.810 0.000]", "[0.000 -9.810 0.000]", "[0.000 0.000 9.810]", "[0.000 0.000 -9.810]"];
const TICKS_PER_SIDE: usize = 5;
const TICKS_TO_FINISH: usize = 10;
const ACCEL_PROGRESS_PER_SIDE: usize = 17;
const MAG_STORED: Stored = &[("CAL_MAG0_ID", 197_388.0)];
const ACCEL_STORED: Stored = &[("CAL_ACC0_ID", 1_310_988.0)];
const APM_ACCEL_POSITIONS: [AccelcalVehiclePos; 6] = [
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_LEVEL,
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_LEFT,
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_RIGHT,
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_NOSEDOWN,
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_NOSEUP,
    AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_BACK,
];
const APM_ACCEL_STORED: Stored = &[("INS_ACCOFFS_X", 0.1), ("INS_ACCOFFS_Y", 0.1), ("INS_ACCOFFS_Z", 0.1)];
const APM_COMPASSES: u8 = 3;
const APM_COMPASS_MASK: u8 = 0x07;
const APM_COMPASS_STEP: u8 = 5;
const APM_COMPASS_FITNESS: f32 = 0.5;
const APM_COMPASS_STORED: Stored = &[
    ("COMPASS_OFS_X", 10.0),
    ("COMPASS_OFS_Y", 10.0),
    ("COMPASS_OFS_Z", 10.0),
    ("COMPASS_OFS2_X", 10.0),
    ("COMPASS_OFS2_Y", 10.0),
    ("COMPASS_OFS2_Z", 10.0),
    ("COMPASS_OFS3_X", 10.0),
    ("COMPASS_OFS3_Y", 10.0),
    ("COMPASS_OFS3_Z", 10.0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    TailDown,
    NoseDown,
    Left,
    Right,
    UpsideDown,
    RightSideUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sensor {
    Mag,
    Accel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Waiting,
    Sampling { side: usize, ticks: usize },
    Finishing(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sides {
    sensor: Sensor,
    done: [bool; 6],
    stage: Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handshake {
    at: usize,
    acked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Calibration {
    sides: Option<Sides>,
    pose: Option<Pose>,
    handshake: Option<Handshake>,
    compass: Option<u8>,
}

impl Sensor {
    fn name(self) -> &'static str {
        match self {
            Sensor::Mag => "mag",
            Sensor::Accel => "accel",
        }
    }

    fn finished(self) -> (Vec<Out>, Stored) {
        match self {
            Sensor::Mag => (vec![info("[cal] progress <100>"), info("[cal] calibration done: mag")], MAG_STORED),
            Sensor::Accel => (vec![info("[cal] calibration done: accel")], ACCEL_STORED),
        }
    }
}

impl Sides {
    fn count(&self) -> usize {
        self.done.iter().filter(|done| **done).count()
    }

    fn sampled(self, side: usize, ticks: usize) -> (Sides, Vec<Out>) {
        match ticks < TICKS_PER_SIDE {
            true => {
                let progress = self.count() * 100 / SIDE_NAMES.len() + (100 / SIDE_NAMES.len()) * ticks / TICKS_PER_SIDE;
                let rotating = (self.sensor == Sensor::Mag).then(|| info(&format!("[cal] {} side calibration: progress <{progress}>", SIDE_NAMES[side])));
                (Sides { stage: Stage::Sampling { side, ticks }, ..self }, rotating.into_iter().collect())
            }
            false => {
                let done = Sides { done: std::array::from_fn(|at| self.done[at] || at == side), ..self };
                let stage = if done.count() == SIDE_NAMES.len() { Stage::Finishing(0) } else { Stage::Waiting };
                let measured = (self.sensor == Sensor::Accel).then(|| [format!("[cal] {} side result: {}", SIDE_NAMES[side], SIDE_RESULTS[side]), format!("[cal] progress <{}>", ACCEL_PROGRESS_PER_SIDE * done.count())]);
                let sent = measured.into_iter().flatten().chain([format!("[cal] {} side done, rotate to a different side", SIDE_NAMES[side])]).map(|text| info(&text)).collect();
                (Sides { stage, ..done }, sent)
            }
        }
    }
}

impl Calibration {
    pub fn set_pose(&mut self, pose: Pose) {
        self.pose = Some(pose);
    }

    pub fn command(&mut self, apm: bool, command: MavCmd, params: [f32; 7]) -> (MavResult, Vec<Out>) {
        match command {
            MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION => (MavResult::MAV_RESULT_ACCEPTED, self.preflight(apm, params)),
            MavCmd::MAV_CMD_DO_START_MAG_CAL if apm => {
                self.compass = Some(0);
                (MavResult::MAV_RESULT_ACCEPTED, Vec::new())
            }
            MavCmd::MAV_CMD_DO_CANCEL_MAG_CAL if apm => {
                self.compass = None;
                (MavResult::MAV_RESULT_ACCEPTED, Vec::new())
            }
            _ => (MavResult::MAV_RESULT_UNSUPPORTED, Vec::new()),
        }
    }

    pub fn acked(&mut self, command: MavCmd) {
        self.handshake = self.handshake.map(|handshake| Handshake { acked: handshake.acked || command == MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS, ..handshake });
    }

    pub fn tick(&mut self) -> (Vec<Out>, Vec<(&'static str, f64)>) {
        let (sent, stored): (Vec<Vec<Out>>, Vec<Stored>) = [self.sides_tick(), self.handshake_tick(), self.compass_tick()].into_iter().unzip();
        (sent.concat(), stored.concat())
    }

    fn preflight(&mut self, apm: bool, params: [f32; 7]) -> Vec<Out> {
        let [gyro, mag, _, _, accel, _, _] = params;
        if params.iter().all(|param| *param == 0.0) {
            self.cancel(apm)
        } else if mag == 1.0 {
            self.start(Sensor::Mag)
        } else if gyro == 1.0 {
            vec![info("[cal] calibration started: 2 gyro")]
        } else if accel == 1.0 && apm {
            self.handshake = Some(Handshake { at: 0, acked: false });
            Vec::new()
        } else if accel == 1.0 {
            self.start(Sensor::Accel)
        } else {
            Vec::new()
        }
    }

    fn start(&mut self, sensor: Sensor) -> Vec<Out> {
        self.pose = None;
        self.sides = Some(Sides { sensor, done: [false; 6], stage: Stage::Waiting });
        vec![info(&format!("[cal] calibration started: 2 {}", sensor.name()))]
    }

    fn cancel(&mut self, apm: bool) -> Vec<Out> {
        match apm {
            true => self.handshake.take().map(|_| position(AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_FAILED)).into_iter().collect(),
            false => {
                self.pose = None;
                self.sides.take().map(|_| statustext(MavSeverity::MAV_SEVERITY_CRITICAL, "[cal] calibration cancelled")).into_iter().collect()
            }
        }
    }

    fn sides_tick(&mut self) -> (Vec<Out>, Stored) {
        let Some(sides) = self.sides else { return (Vec::new(), &[]) };
        match sides.stage {
            Stage::Finishing(ticks) if ticks + 1 < TICKS_TO_FINISH => {
                self.sides = Some(Sides { stage: Stage::Finishing(ticks + 1), ..sides });
                (Vec::new(), &[])
            }
            Stage::Finishing(_) => {
                self.sides = None;
                sides.sensor.finished()
            }
            Stage::Sampling { side, ticks } => {
                let (next, sent) = sides.sampled(side, ticks + 1);
                self.sides = Some(next);
                (sent, &[])
            }
            Stage::Waiting => (self.detect(sides), &[]),
        }
    }

    fn detect(&mut self, sides: Sides) -> Vec<Out> {
        match self.pose.take().map(|pose| pose as usize) {
            None => Vec::new(),
            Some(side) if sides.done[side] => vec![info(&format!("[cal] {} side already completed", SIDE_NAMES[side]))],
            Some(side) => {
                self.sides = Some(Sides { stage: Stage::Sampling { side, ticks: 0 }, ..sides });
                let hold = (sides.sensor == Sensor::Accel).then(|| format!("[cal] Hold still, measuring {} side", SIDE_NAMES[side]));
                std::iter::once(format!("[cal] {} orientation detected", SIDE_NAMES[side])).chain(hold).map(|text| info(&text)).collect()
            }
        }
    }

    fn handshake_tick(&mut self) -> (Vec<Out>, Stored) {
        match self.handshake {
            None => (Vec::new(), &[]),
            Some(Handshake { at, acked: false }) => (vec![position(APM_ACCEL_POSITIONS[at])], &[]),
            Some(Handshake { at, acked: true }) => match APM_ACCEL_POSITIONS.get(at + 1) {
                Some(next) => {
                    self.handshake = Some(Handshake { at: at + 1, acked: false });
                    (vec![position(*next)], &[])
                }
                None => {
                    self.handshake = None;
                    (vec![position(AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_SUCCESS)], APM_ACCEL_STORED)
                }
            },
        }
    }

    fn compass_tick(&mut self) -> (Vec<Out>, Stored) {
        let Some(percent) = self.compass else { return (Vec::new(), &[]) };
        let finished = percent >= 100;
        self.compass = (!finished).then(|| (percent + APM_COMPASS_STEP).min(100));
        let progress = (0..APM_COMPASSES).map(|id| mag_progress(id, percent));
        let reports = (0..APM_COMPASSES).filter(|_| finished).map(mag_report);
        (progress.chain(reports).collect(), if finished { APM_COMPASS_STORED } else { &[] })
    }
}

fn info(text: &str) -> Out {
    statustext(MavSeverity::MAV_SEVERITY_INFO, text)
}

fn statustext(severity: MavSeverity, text: &str) -> Out {
    (AUTOPILOT, MavMessage::STATUSTEXT(STATUSTEXT_DATA { severity, text: crate::mavout::chars(text), ..Default::default() }))
}

fn position(position: AccelcalVehiclePos) -> Out {
    (AUTOPILOT, MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
        param1: position as u32 as f32,
        command: MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS,
        target_system: crate::mavout::DEFAULT_GCS_SYSTEM,
        target_component: crate::mavout::GCS_COMPONENT,
        ..Default::default()
    }))
}

fn mag_progress(compass_id: u8, completion_pct: u8) -> Out {
    (AUTOPILOT, MavMessage::MAG_CAL_PROGRESS(MAG_CAL_PROGRESS_DATA { compass_id, cal_mask: APM_COMPASS_MASK, cal_status: MagCalStatus::MAG_CAL_RUNNING_STEP_ONE, completion_pct, ..Default::default() }))
}

fn mag_report(compass_id: u8) -> Out {
    (AUTOPILOT, MavMessage::MAG_CAL_REPORT(MAG_CAL_REPORT_DATA { compass_id, cal_mask: APM_COMPASS_MASK, cal_status: MagCalStatus::MAG_CAL_SUCCESS, fitness: APM_COMPASS_FITNESS, ..Default::default() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PX4: bool = false;
    const APM: bool = true;
    const CANCEL: [f32; 7] = [0.0; 7];
    const GYRO: [f32; 7] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    const MAG: [f32; 7] = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    const ACCEL: [f32; 7] = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    fn texts(sent: &[Out]) -> Vec<String> {
        sent.iter()
            .filter_map(|(_, message)| match message {
                MavMessage::STATUSTEXT(status) => status.text.to_str().ok().map(|text| text.trim_end_matches('\0').to_string()),
                _ => None,
            })
            .collect()
    }

    fn preflight(cal: &mut Calibration, apm: bool, params: [f32; 7]) -> Vec<String> {
        let (result, sent) = cal.command(apm, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, params);
        assert_eq!(result, MavResult::MAV_RESULT_ACCEPTED);
        texts(&sent)
    }

    fn ticks(cal: &mut Calibration, count: usize) -> Vec<Vec<String>> {
        (0..count).map(|_| texts(&cal.tick().0)).collect()
    }

    fn place(cal: &mut Calibration, pose: Pose, count: usize) -> Vec<Vec<String>> {
        cal.set_pose(pose);
        ticks(cal, count)
    }

    fn silent(count: usize) -> Vec<Vec<String>> {
        vec![Vec::new(); count]
    }

    fn positions(sent: &[Out]) -> Vec<u32> {
        sent.iter()
            .filter_map(|(_, message)| match message {
                MavMessage::COMMAND_LONG(c) if c.command == MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS && (c.target_system, c.target_component) == (255, 190) => Some(c.param1 as u32),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn px4_gyro_only_announces_its_start_like_qgc_mocklink() {
        let mut cal = Calibration::default();
        assert_eq!(preflight(&mut cal, PX4, GYRO), ["[cal] calibration started: 2 gyro"]);
        assert_eq!(ticks(&mut cal, 30), silent(30));
        assert!(preflight(&mut cal, PX4, CANCEL).is_empty(), "nothing pose-driven is running, so there is nothing to cancel");
    }

    #[test]
    fn px4_mag_rotates_each_placed_side_then_stores_the_id_and_finishes() {
        let mut cal = Calibration::default();
        assert_eq!(preflight(&mut cal, PX4, MAG), ["[cal] calibration started: 2 mag"]);
        assert_eq!(ticks(&mut cal, 3), silent(3), "waits for the vehicle to be placed");
        assert_eq!(
            place(&mut cal, Pose::RightSideUp, 6),
            [
                vec!["[cal] down orientation detected"],
                vec!["[cal] down side calibration: progress <3>"],
                vec!["[cal] down side calibration: progress <6>"],
                vec!["[cal] down side calibration: progress <9>"],
                vec!["[cal] down side calibration: progress <12>"],
                vec!["[cal] down side done, rotate to a different side"],
            ]
        );
        assert_eq!(place(&mut cal, Pose::RightSideUp, 1), [vec!["[cal] down side already completed"]]);
        [Pose::TailDown, Pose::NoseDown, Pose::Left, Pose::Right].into_iter().for_each(|pose| assert_eq!(place(&mut cal, pose, 6).len(), 6));
        assert_eq!(
            place(&mut cal, Pose::UpsideDown, 6),
            [
                vec!["[cal] up orientation detected"],
                vec!["[cal] up side calibration: progress <86>"],
                vec!["[cal] up side calibration: progress <89>"],
                vec!["[cal] up side calibration: progress <92>"],
                vec!["[cal] up side calibration: progress <95>"],
                vec!["[cal] up side done, rotate to a different side"],
            ]
        );
        assert_eq!(ticks(&mut cal, 9), silent(9), "the fit is still being calculated");
        let (sent, stored) = cal.tick();
        assert_eq!(texts(&sent), ["[cal] progress <100>", "[cal] calibration done: mag"]);
        assert_eq!(stored, [("CAL_MAG0_ID", 197_388.0)]);
        assert_eq!(cal, Calibration::default());
    }

    #[test]
    fn px4_accel_measures_each_held_side_then_stores_the_id_and_finishes() {
        let mut cal = Calibration::default();
        assert_eq!(preflight(&mut cal, PX4, ACCEL), ["[cal] calibration started: 2 accel"]);
        assert_eq!(
            place(&mut cal, Pose::TailDown, 6),
            [
                vec!["[cal] back orientation detected", "[cal] Hold still, measuring back side"],
                vec![],
                vec![],
                vec![],
                vec![],
                vec!["[cal] back side result: [9.810 0.000 0.000]", "[cal] progress <17>", "[cal] back side done, rotate to a different side"],
            ]
        );
        assert_eq!(place(&mut cal, Pose::TailDown, 1), [vec!["[cal] back side already completed"]]);
        [Pose::NoseDown, Pose::Left, Pose::Right, Pose::UpsideDown].into_iter().for_each(|pose| assert_eq!(place(&mut cal, pose, 6).len(), 6));
        assert_eq!(place(&mut cal, Pose::RightSideUp, 6).last().unwrap(), &["[cal] down side result: [0.000 0.000 -9.810]", "[cal] progress <102>", "[cal] down side done, rotate to a different side"]);
        assert_eq!(ticks(&mut cal, 9), silent(9));
        let (sent, stored) = cal.tick();
        assert_eq!(texts(&sent), ["[cal] calibration done: accel"]);
        assert_eq!(stored, [("CAL_ACC0_ID", 1_310_988.0)]);
        assert_eq!(ticks(&mut cal, 5), silent(5));
    }

    #[test]
    fn an_all_zero_calibration_cancels_a_px4_run_once_and_critically() {
        let mut cal = Calibration::default();
        preflight(&mut cal, PX4, ACCEL);
        place(&mut cal, Pose::Left, 3);
        let (result, sent) = cal.command(PX4, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, CANCEL);
        assert_eq!(result, MavResult::MAV_RESULT_ACCEPTED);
        assert!(matches!(&sent[..], [(AUTOPILOT, MavMessage::STATUSTEXT(s))] if s.severity == MavSeverity::MAV_SEVERITY_CRITICAL));
        assert_eq!(texts(&sent), ["[cal] calibration cancelled"]);
        assert_eq!(place(&mut cal, Pose::Right, 10), silent(10));
        assert!(preflight(&mut cal, PX4, CANCEL).is_empty());
    }

    #[test]
    fn apm_accel_repeats_each_position_until_next_is_acked_then_succeeds() {
        let mut cal = Calibration::default();
        assert!(preflight(&mut cal, APM, ACCEL).is_empty());
        assert_eq!(positions(&cal.tick().0), [1]);
        cal.acked(MavCmd::MAV_CMD_COMPONENT_ARM_DISARM);
        assert_eq!(positions(&cal.tick().0), [1], "only the Next ack advances");
        let walked: Vec<u32> = (0..5)
            .flat_map(|_| {
                cal.acked(MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS);
                positions(&cal.tick().0)
            })
            .collect();
        assert_eq!(walked, [2, 3, 4, 5, 6]);
        cal.acked(MavCmd::MAV_CMD_ACCELCAL_VEHICLE_POS);
        let (sent, stored) = cal.tick();
        assert_eq!(positions(&sent), [AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_SUCCESS as u32]);
        assert_eq!(stored, [("INS_ACCOFFS_X", 0.1), ("INS_ACCOFFS_Y", 0.1), ("INS_ACCOFFS_Z", 0.1)]);
        assert!(cal.tick().0.is_empty());
    }

    #[test]
    fn an_all_zero_calibration_fails_the_apm_accel_handshake() {
        let mut cal = Calibration::default();
        preflight(&mut cal, APM, ACCEL);
        cal.tick();
        let (_, sent) = cal.command(APM, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, CANCEL);
        assert_eq!(positions(&sent), [AccelcalVehiclePos::ACCELCAL_VEHICLE_POS_FAILED as u32]);
        assert!(cal.tick().0.is_empty());
        assert!(cal.command(APM, MavCmd::MAV_CMD_PREFLIGHT_CALIBRATION, CANCEL).1.is_empty());
    }

    #[test]
    fn apm_compass_streams_progress_for_three_compasses_then_reports_success() {
        let mut cal = Calibration::default();
        assert_eq!(cal.command(PX4, MavCmd::MAV_CMD_DO_START_MAG_CAL, [0.0; 7]).0, MavResult::MAV_RESULT_UNSUPPORTED);
        assert_eq!(cal.command(APM, MavCmd::MAV_CMD_DO_START_MAG_CAL, [0.0; 7]), (MavResult::MAV_RESULT_ACCEPTED, Vec::new()));
        let run: Vec<(Vec<Out>, Vec<(&str, f64)>)> = (0..21).map(|_| cal.tick()).collect();
        let percents: Vec<Vec<(u8, u8)>> = run
            .iter()
            .map(|(sent, _)| sent.iter().filter_map(|(_, m)| match m { MavMessage::MAG_CAL_PROGRESS(p) if p.cal_mask == 0x07 => Some((p.compass_id, p.completion_pct)), _ => None }).collect())
            .collect();
        assert_eq!(percents, (0..=100).step_by(5).map(|pct| vec![(0, pct), (1, pct), (2, pct)]).collect::<Vec<_>>());
        let (last, stored) = run.last().unwrap();
        let reports: Vec<(u8, MagCalStatus, f32)> = last.iter().filter_map(|(_, m)| match m { MavMessage::MAG_CAL_REPORT(r) => Some((r.compass_id, r.cal_status, r.fitness)), _ => None }).collect();
        assert_eq!(reports, [(0, MagCalStatus::MAG_CAL_SUCCESS, 0.5), (1, MagCalStatus::MAG_CAL_SUCCESS, 0.5), (2, MagCalStatus::MAG_CAL_SUCCESS, 0.5)]);
        assert_eq!(stored.len(), 9);
        assert!(run[..20].iter().all(|(sent, stored)| stored.is_empty() && sent.len() == 3));
        assert!(cal.tick().0.is_empty());
        cal.command(APM, MavCmd::MAV_CMD_DO_START_MAG_CAL, [0.0; 7]);
        cal.tick();
        assert_eq!(cal.command(APM, MavCmd::MAV_CMD_DO_CANCEL_MAG_CAL, [0.0; 7]).0, MavResult::MAV_RESULT_ACCEPTED);
        assert!(cal.tick().0.is_empty());
    }
}
