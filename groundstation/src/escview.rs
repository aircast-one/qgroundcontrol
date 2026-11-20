use serde_json::{Value, json};

use crate::read::flag;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.escs.count",
    "vehicle.apmFirmware",
    "vehicle.multiRotor",
    "vehicle.parameterManager.parametersReady",
];

const HEALTH_CHECKED_MOTORS: u32 = 4;
const TEMPERATURE_ELEVATED_C: f64 = 70.0;
const TEMPERATURE_CRITICAL_C: f64 = 90.0;
const IMBALANCE_ELEVATED: f64 = 0.25;
const IMBALANCE_CRITICAL: f64 = 0.5;
const IMBALANCE_ELEVATED_FLOOR_A: f64 = 2.0;
const IMBALANCE_CRITICAL_FLOOR_A: f64 = 4.0;
const IMBALANCE_MIN_MOTORS: usize = 3;
const RPM_IN_THOUSANDS: f64 = 10_000.0;
const FIRST_MOTOR_FUNCTION: i64 = 33;
const LAST_MOTOR_FUNCTION: i64 = 40;

const ARDUPILOT_MOTOR_ANGLES: &[(i64, i64, &[f64])] = &[
    (1, 0, &[90.0, -90.0, 0.0, 180.0]),
    (1, 1, &[45.0, -135.0, -45.0, 135.0]),
    (2, 0, &[0.0, 180.0, -120.0, 60.0, -60.0, 120.0]),
    (2, 1, &[90.0, -90.0, -30.0, 150.0, 30.0, -150.0]),
    (3, 0, &[0.0, 180.0, 45.0, 135.0, -45.0, -135.0, -90.0, 90.0]),
    (
        3,
        1,
        &[22.5, -157.5, 67.5, 157.5, -22.5, -112.5, -67.5, 112.5],
    ),
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Wiring {
    pub angles: &'static [f64],
    pub outputs: Vec<i64>,
    pub motors: Vec<Option<i64>>,
}

fn fact<'a>(group: &'a Value, name: &str) -> Option<&'a Value> {
    group
        .get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| {
            facts
                .iter()
                .find(|f| f.get("name").and_then(Value::as_str) == Some(name))
        })
}

fn raw(group: &Value, name: &str) -> i64 {
    fact(group, name)
        .and_then(|f| f.get("rawValue").or(f.get("value")))
        .and_then(Value::as_f64)
        .map_or(0, |v| v as i64)
}

fn shown(group: &Value, name: &str) -> String {
    fact(group, name)
        .map(|f| {
            let value = f
                .get("valueString")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let units = f.get("units").and_then(Value::as_str).unwrap_or_default();
            [value, units]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn reading(group: &Value, name: &str) -> f64 {
    fact(group, name)
        .and_then(|f| f.get("rawValue").or(f.get("value")))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn level(value: f64, elevated: f64, critical: f64) -> &'static str {
    match value {
        v if v >= critical => "critical",
        v if v >= elevated => "elevated",
        _ => "normal",
    }
}

fn title(index: usize, esc: &Value, wiring: Option<&Wiring>) -> String {
    match wiring {
        None => format!("Motor {}", raw(esc, "id") + 1),
        Some(w) => match (w.motors.get(index).copied().flatten(), w.outputs.get(index)) {
            (Some(motor), _) => format!("Motor {motor}"),
            (None, Some(output)) => format!("ESC {output}"),
            (None, None) => format!("ESC {}", raw(esc, "id") + 1),
        },
    }
}

fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    (sorted.len() >= IMBALANCE_MIN_MOTORS).then(|| {
        if sorted.len() % 2 == 0 {
            (sorted[middle - 1] + sorted[middle]) / 2.0
        } else {
            sorted[middle]
        }
    })
}

fn imbalance(steady: f64, median: Option<f64>) -> &'static str {
    median.map_or("normal", |median| {
        level(
            (steady - median).abs(),
            (median * IMBALANCE_ELEVATED).max(IMBALANCE_ELEVATED_FLOOR_A),
            (median * IMBALANCE_CRITICAL).max(IMBALANCE_CRITICAL_FLOOR_A),
        )
    })
}

fn rank(level: &str) -> u8 {
    match level {
        "critical" => 2,
        "elevated" => 1,
        _ => 0,
    }
}

struct Rotor {
    motor: i64,
    angle: f64,
    at: Option<usize>,
    state: &'static str,
    temperature: f64,
    current: f64,
    rpm: f64,
    temperature_level: &'static str,
    current_level: &'static str,
}

impl Rotor {
    fn level(&self) -> &'static str {
        match self.state {
            "lost" => "critical",
            _ if rank(self.temperature_level) >= rank(self.current_level) => self.temperature_level,
            _ => self.current_level,
        }
    }

    fn reasons(&self, middle: Option<f64>) -> Vec<(u8, String)> {
        let motor = self.motor;
        let lost = (self.state == "lost")
            .then(|| (3, format!("Motor {motor} stopped sending ESC telemetry.")));
        let heat = match self.temperature_level {
            "critical" => Some((
                2,
                format!(
                    "Motor {motor} is at {}, past the {} limit.",
                    crate::units::temperature_text(self.temperature),
                    crate::units::temperature_text(TEMPERATURE_CRITICAL_C)
                ),
            )),
            "elevated" => Some((
                1,
                format!(
                    "Motor {motor} is warm at {}; {} is critical.",
                    crate::units::temperature_text(self.temperature),
                    crate::units::temperature_text(TEMPERATURE_CRITICAL_C)
                ),
            )),
            _ => None,
        };
        let draw = middle
            .filter(|_| self.current_level != "normal")
            .map(|middle| {
                (
                    rank(self.current_level),
                    format!(
                        "Motor {motor} draws {:.0} A while the others draw about {middle:.0} A: check its propeller and motor.",
                        self.current
                    ),
                )
            });
        [lost, heat, draw].into_iter().flatten().collect()
    }

    fn json(&self) -> Value {
        json!({
            "motor": self.motor,
            "angle": self.angle,
            "state": self.state,
            "temperature": crate::units::temperature_text(self.temperature),
            "current": format!("{:.1}", self.current),
            "rpm": if self.rpm >= RPM_IN_THOUSANDS {
                format!("{:.0}k", self.rpm / 1000.0)
            } else {
                format!("{:.0}", self.rpm)
            },
            "temperatureLevel": self.temperature_level,
            "currentLevel": self.current_level,
        })
    }
}

fn rotors(
    escs: &[Value],
    wiring: &Wiring,
    online: &dyn Fn(usize) -> bool,
    steady: &[f64],
) -> (Vec<Rotor>, Option<f64>) {
    let placed: Vec<(f64, i64, Option<usize>)> = wiring
        .angles
        .iter()
        .zip(1..)
        .map(|(angle, motor)| {
            let at = wiring
                .motors
                .iter()
                .position(|m| *m == Some(motor))
                .filter(|at| *at < escs.len());
            (*angle, motor, at)
        })
        .collect();
    let live = |at: Option<usize>| at.filter(|at| online(*at));
    let steady_of = |at: Option<usize>| live(at).and_then(|at| steady.get(at)).copied();
    let middle = median(
        &placed
            .iter()
            .filter_map(|(_, _, at)| steady_of(*at))
            .collect::<Vec<_>>(),
    );
    let placed_rotors = placed
        .iter()
        .map(|(angle, motor, at)| {
            let esc = live(*at).and_then(|at| escs.get(at));
            let temperature = esc.map_or(0.0, |e| reading(e, "temperature") / 100.0);
            Rotor {
                motor: *motor,
                angle: *angle,
                at: *at,
                state: match (at, esc) {
                    (_, Some(_)) => "reporting",
                    (Some(_), None) => "lost",
                    (None, None) => "none",
                },
                temperature,
                current: esc.map_or(0.0, |e| reading(e, "current")),
                rpm: esc.map_or(0.0, |e| reading(e, "rpm")),
                temperature_level: level(
                    temperature,
                    TEMPERATURE_ELEVATED_C,
                    TEMPERATURE_CRITICAL_C,
                ),
                current_level: steady_of(*at).map_or("normal", |s| imbalance(s, middle)),
            }
        })
        .collect();
    (placed_rotors, middle)
}

fn unwired_hint(unwired: &[i64]) -> Option<String> {
    match unwired {
        [] => None,
        [motor] => Some(format!(
            "Motor {motor} sends no ESC telemetry: check its ESC's telemetry connection and numbering."
        )),
        [rest @ .., last] => Some(format!(
            "Motors {} and {last} send no ESC telemetry: check their ESCs' telemetry connection and numbering.",
            rest.iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn hints(wiring: &Wiring, rotors: &[Rotor], middle: Option<f64>) -> Vec<String> {
    let reasons: Vec<(u8, String)> = rotors
        .iter()
        .flat_map(|rotor| rotor.reasons(middle))
        .collect();
    let unwired: Vec<i64> = rotors
        .iter()
        .filter(|rotor| rotor.state == "none")
        .map(|rotor| rotor.motor)
        .collect();
    let unmatched = wiring
        .motors
        .iter()
        .zip(&wiring.outputs)
        .find(|(motor, _)| motor.is_none())
        .map(|(_, output)| {
            format!(
                "ESC {output} matches no motor: its output drives none and the frame has fewer motors. Number each ESC after its motor, or shift DroneCAN ESCs with CAN_D1_UC_ESC_OF onto their motors' outputs."
            )
        });
    [3, 2, 1]
        .iter()
        .flat_map(|urgency| {
            reasons
                .iter()
                .filter(move |(rank, _)| rank == urgency)
                .map(|(_, text)| text.clone())
        })
        .chain(unwired_hint(&unwired))
        .chain(unmatched)
        .collect()
}

pub fn summary(escs: &[Value], wiring: Option<&Wiring>, steady: &[f64]) -> Value {
    let Some(first) = escs.first() else {
        return json!({ "kind": "object", "class": "Escs", "shown": false });
    };
    let motor_count = raw(first, "count");
    let online_mask = raw(first, "info");
    let online_count = if motor_count == 0 {
        0
    } else {
        i64::from(online_mask.count_ones())
    };
    let online = |index: usize| online_mask & (1 << index) != 0;
    let motor_healthy = |index: usize| {
        online(index)
            && escs
                .get(index)
                .is_some_and(|esc| raw(esc, "failureFlags") == 0)
    };
    let failing_checked = (0..HEALTH_CHECKED_MOTORS as usize).any(|index| {
        online(index)
            && escs
                .get(index)
                .is_some_and(|esc| raw(esc, "failureFlags") > 0)
    });
    let (frame, middle) = wiring
        .map(|w| rotors(escs, w, &online, steady))
        .unwrap_or_default();
    let reporting = frame
        .iter()
        .filter(|rotor| rotor.state == "reporting")
        .count();
    let healthy = online_count == motor_count
        && !failing_checked
        && reporting == frame.len()
        && frame.iter().all(|rotor| rotor.level() != "critical");
    let healthy_motors = (0..escs.len()).filter(|i| motor_healthy(*i)).count();
    let mut motors: Vec<(&'static str, Option<i64>, usize, Value)> = escs
        .iter()
        .enumerate()
        .map(|(index, esc)| {
            let fine = motor_healthy(index);
            let motor = wiring.and_then(|w| w.motors.get(index).copied().flatten());
            let rotor = frame.iter().find(|rotor| rotor.at == Some(index));
            let level = if fine {
                rotor.map_or("normal", Rotor::level)
            } else {
                "critical"
            };
            let entry = json!({
                "title": format!("{} {}", title(index, esc, wiring), if fine { "" } else { "- OFFLINE" }).trim_end().to_string(),
                "motor": motor,
                "level": level,
                "reasons": rotor.map(|rotor| rotor.reasons(middle).into_iter().map(|(_, text)| text).collect::<Vec<_>>()).unwrap_or_default(),
                "rpm": shown(esc, "rpm"),
                "temperature": shown(esc, "temperature"),
                "voltage": shown(esc, "voltage"),
                "current": shown(esc, "current"),
                "errors": shown(esc, "errorCount"),
            });
            (level, motor, index, entry)
        })
        .collect();
    motors.sort_by_key(|(level, motor, index, _)| {
        (
            std::cmp::Reverse(rank(level)),
            motor.unwrap_or(i64::MAX),
            *index,
        )
    });
    json!({
        "kind": "object",
        "class": "Escs",
        "shown": true,
        "onlineCount": online_count,
        "healthy": healthy,
        "healthText": if healthy { "OK" } else { "ERR" },
        "healthyMotorsText": if frame.is_empty() {
            format!("{healthy_motors}/{}", escs.len())
        } else {
            format!("{reporting}/{}", frame.len())
        },
        "totalErrors": escs.iter().map(|esc| raw(esc, "errorCount")).sum::<i64>(),
        "hints": wiring.map(|w| hints(w, &frame, middle)).unwrap_or_default(),
        "frame": frame.iter().map(Rotor::json).collect::<Vec<_>>(),
        "motors": motors.into_iter().map(|(_, _, _, entry)| entry).collect::<Vec<_>>(),
    })
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<i64> {
    let fact = backend.value(&format!("vehicle.parameterManager.getParameter(-1,{name})"));
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact")
        && fact
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| !n.is_empty());
    present
        .then(|| {
            fact.get("rawValue")
                .or(fact.get("value"))
                .and_then(Value::as_f64)
        })
        .flatten()
        .map(|v| v as i64)
}

pub fn motor_angles(class: Option<i64>, frame_type: Option<i64>) -> &'static [f64] {
    ARDUPILOT_MOTOR_ANGLES
        .iter()
        .find(|(c, t, _)| Some(*c) == class && Some(*t) == frame_type)
        .map_or(&[], |(_, _, angles)| *angles)
}

fn ardupilot_wiring(backend: &dyn Backend, escs: &[Value]) -> Option<Wiring> {
    let vehicle = backend.value_fields("vehicle", "apmFirmware,multiRotor");
    flag(&vehicle, "apmFirmware").then(|| {
        let offset = parameter(backend, "ESC_TLM_MAV_OFS").unwrap_or(0);
        let outputs: Vec<i64> = escs.iter().map(|esc| raw(esc, "id") + offset + 1).collect();
        let angles = if flag(&vehicle, "multiRotor") {
            motor_angles(
                parameter(backend, "FRAME_CLASS"),
                parameter(backend, "FRAME_TYPE"),
            )
        } else {
            &[]
        };
        let motor_count = angles.len() as i64;
        Wiring {
            angles,
            motors: outputs
                .iter()
                .map(|output| {
                    parameter(backend, &format!("SERVO{output}_FUNCTION"))
                        .filter(|f| (FIRST_MOTOR_FUNCTION..=LAST_MOTOR_FUNCTION).contains(f))
                        .map(|f| f - FIRST_MOTOR_FUNCTION + 1)
                        .or_else(|| (*output <= motor_count).then_some(*output))
                })
                .collect(),
            outputs,
        }
    })
}

pub fn esc_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let escs = backend
        .value("vehicle.escs")
        .get("elements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let steady: Vec<f64> = backend
        .value("vehicle.escs.steadyCurrents")
        .get("value")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    summary(&escs, ardupilot_wiring(backend, &escs).as_ref(), &steady)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esc(id: i64, count: i64, info: i64, failures: i64, errors: i64) -> Value {
        let f = |name: &str, value: i64, units: &str| json!({ "name": name, "rawValue": value, "valueString": value.to_string(), "units": units });
        json!({ "facts": [f("id", id, ""), f("count", count, ""), f("info", info, ""), f("failureFlags", failures, ""), f("errorCount", errors, ""), f("rpm", 4200, "rpm"), f("voltage", 16, "V")] })
    }

    #[test]
    fn health_reads_as_the_toolbar() {
        let four = [
            esc(0, 4, 0b1111, 0, 1),
            esc(1, 4, 0b1111, 0, 0),
            esc(2, 4, 0b1111, 0, 2),
            esc(3, 4, 0b1111, 0, 0),
        ];
        let fine = summary(&four, None, &[]);
        assert_eq!(
            (fine["onlineCount"].as_i64(), fine["healthText"].as_str()),
            (Some(4), Some("OK"))
        );
        assert_eq!(fine["totalErrors"], 3);
        assert_eq!(fine["healthyMotorsText"], "4/4");
        assert_eq!(fine["motors"][0]["title"], "Motor 1");
        assert_eq!(fine["motors"][0]["rpm"], "4200 rpm");
        let one_down = [
            esc(0, 4, 0b0111, 0, 0),
            esc(1, 4, 0b0111, 0, 0),
            esc(2, 4, 0b0111, 0, 0),
            esc(3, 4, 0b0111, 0, 0),
        ];
        let down = summary(&one_down, None, &[]);
        assert_eq!(
            down["healthText"], "ERR",
            "a motor missing from the online mask is an error"
        );
        assert_eq!(
            (
                down["motors"][0]["title"].as_str(),
                down["motors"][0]["level"].as_str()
            ),
            (Some("Motor 4 - OFFLINE"), Some("critical")),
            "the motor in trouble leads the list"
        );
        let failing = [esc(0, 1, 0b1, 4, 0)];
        assert_eq!(
            summary(&failing, None, &[])["healthText"],
            "ERR",
            "a failure flag on an online motor is an error"
        );
        assert_eq!(summary(&[], None, &[])["shown"], false);
    }

    #[test]
    fn ardupilot_esc_telemetry_lists_only_the_escs_that_report() {
        use mavlink::dialects::ardupilotmega::{
            ESC_TELEMETRY_1_TO_4_DATA, ESC_TELEMETRY_5_TO_8_DATA, MavMessage,
        };
        let mut escs = crate::sensorfacts::Escs::default();
        escs.apply(
            &MavMessage::ESC_TELEMETRY_1_TO_4(ESC_TELEMETRY_1_TO_4_DATA {
                voltage: [5010, 5000, 0, 0],
                current: [250, 0, 0, 0],
                rpm: [4200, 4100, 0, 0],
                count: [12, 3, 0, 0],
                temperature: [41, 40, 0, 0],
                ..Default::default()
            }),
            0,
        );
        escs.apply(
            &MavMessage::ESC_TELEMETRY_5_TO_8(ESC_TELEMETRY_5_TO_8_DATA {
                rpm: [0, 3900, 0, 0],
                count: [0, 7, 0, 0],
                ..Default::default()
            }),
            0,
        );
        let listed = crate::vehiclefact::esc_list(&escs);
        let shown = summary(listed["elements"].as_array().unwrap(), None, &[]);
        assert_eq!(
            (shown["onlineCount"].as_i64(), shown["healthText"].as_str()),
            (Some(3), Some("OK"))
        );
        assert_eq!(
            shown["motors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|motor| motor["title"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["Motor 1", "Motor 2", "Motor 6"]
        );
        assert_eq!(
            (
                escs.by_id[&0].voltage,
                escs.by_id[&0].current,
                escs.by_id[&0].temperature
            ),
            (50.1, 2.5, 4100)
        );
    }
    struct Vampire {
        escs: crate::sensorfacts::Escs,
        now_ms: u64,
    }

    impl Backend for Vampire {
        fn get(&self, path: &str) -> String {
            let parameter = |name: &str, value: i64| {
                json!({ "kind": "fact", "name": name, "rawValue": value }).to_string()
            };
            match path {
                "vehicle.escs" => {
                    crate::vehiclefact::esc_list(&self.escs.online_at(self.now_ms)).to_string()
                }
                "vehicle.escs.steadyCurrents" => json!({
                    "kind": "value",
                    "value": self.escs.by_id.values().map(|esc| esc.steady_current).collect::<Vec<_>>(),
                })
                .to_string(),
                _ => path
                    .strip_prefix("vehicle.parameterManager.getParameter(-1,")
                    .and_then(|rest| rest.strip_suffix(')'))
                    .and_then(|name| match name {
                        "FRAME_CLASS" => Some(parameter(name, 2)),
                        "FRAME_TYPE" => Some(parameter(name, 1)),
                        "ESC_TLM_MAV_OFS" => Some(parameter(name, 0)),
                        _ => name
                            .strip_prefix("SERVO")
                            .and_then(|n| n.strip_suffix("_FUNCTION"))
                            .and_then(|n| n.parse::<i64>().ok())
                            .map(|output| {
                                parameter(
                                    name,
                                    if (9..=14).contains(&output) {
                                        output + 24
                                    } else {
                                        0
                                    },
                                )
                            }),
                    })
                    .unwrap_or_else(|| json!({ "kind": "null" }).to_string()),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => {
                    json!({ "kind": "object", "apmFirmware": true, "multiRotor": true }).to_string()
                }
                _ => self.get(path),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn an_ardupilot_esc_lands_on_the_motor_its_servo_output_drives() {
        use mavlink::dialects::ardupilotmega::{
            ESC_TELEMETRY_1_TO_4_DATA, ESC_TELEMETRY_5_TO_8_DATA, ESC_TELEMETRY_13_TO_16_DATA,
            MavMessage,
        };
        let mut escs = crate::sensorfacts::Escs::default();
        escs.apply(
            &MavMessage::ESC_TELEMETRY_1_TO_4(ESC_TELEMETRY_1_TO_4_DATA {
                count: [0, 5, 0, 0],
                temperature: [0, 23, 0, 0],
                ..Default::default()
            }),
            0,
        );
        escs.apply(
            &MavMessage::ESC_TELEMETRY_5_TO_8(ESC_TELEMETRY_5_TO_8_DATA {
                count: [0, 0, 0, 4],
                ..Default::default()
            }),
            0,
        );
        escs.apply(
            &MavMessage::ESC_TELEMETRY_13_TO_16(ESC_TELEMETRY_13_TO_16_DATA {
                current: [5040, 0, 0, 0],
                rpm: [4200, 0, 0, 0],
                count: [9, 0, 0, 0],
                temperature: [75, 0, 0, 0],
                ..Default::default()
            }),
            0,
        );
        let shown = esc_view(&Vampire { escs, now_ms: 0 }, &[]);
        assert_eq!(
            shown["motors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|motor| motor["title"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["Motor 5", "Motor 2", "ESC 8"],
            "ESC 13 is on SERVO13 = Motor 5 and leads because it runs warm; ESC 2's output drives nothing, so it is Motor 2; a hexa has no motor 8, so ESC 8 comes last"
        );
        let frame = shown["frame"].as_array().unwrap();
        assert_eq!(
            frame
                .iter()
                .map(|m| m["angle"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            vec![90.0, -90.0, -30.0, 150.0, 30.0, -150.0]
        );
        let five = &frame[4];
        assert_eq!(
            (
                five["state"].as_str(),
                five["current"].as_str(),
                five["rpm"].as_str(),
                five["currentLevel"].as_str(),
                five["temperatureLevel"].as_str()
            ),
            (
                Some("reporting"),
                Some("50.4"),
                Some("4200"),
                Some("normal"),
                Some("elevated")
            )
        );
        assert_eq!(
            (frame[0]["state"].as_str(), frame[1]["state"].as_str()),
            (Some("none"), Some("reporting"))
        );
        assert_eq!(
            shown["hints"],
            json!([
                "Motor 5 is warm at 75 \u{b0}C; 90 \u{b0}C is critical.",
                "Motors 1, 3, 4 and 6 send no ESC telemetry: check their ESCs' telemetry connection and numbering.",
                "ESC 8 matches no motor: its output drives none and the frame has fewer motors. Number each ESC after its motor, or shift DroneCAN ESCs with CAN_D1_UC_ESC_OF onto their motors' outputs."
            ])
        );
        assert_eq!(
            (
                shown["healthText"].as_str(),
                shown["healthyMotorsText"].as_str()
            ),
            (Some("ERR"), Some("2/6")),
            "a hexa with four silent motors is not healthy, whatever the reporting ESCs say"
        );
    }

    #[test]
    fn a_hobbywing_esc_numbered_after_its_motor_lands_there_without_an_offset() {
        use mavlink::dialects::ardupilotmega::{ESC_TELEMETRY_5_TO_8_DATA, MavMessage};
        let mut escs = crate::sensorfacts::Escs::default();
        escs.apply(
            &MavMessage::ESC_TELEMETRY_5_TO_8(ESC_TELEMETRY_5_TO_8_DATA {
                voltage: [4970, 0, 0, 0],
                count: [2757, 0, 0, 0],
                temperature: [23, 0, 0, 0],
                ..Default::default()
            }),
            0,
        );
        let shown = esc_view(&Vampire { escs, now_ms: 0 }, &[]);
        let frame = shown["frame"].as_array().unwrap();
        assert_eq!(
            frame
                .iter()
                .map(|m| m["state"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["none", "none", "none", "none", "reporting", "none"]
        );
        assert_eq!(
            shown["hints"],
            json!([
                "Motors 1, 2, 3, 4 and 6 send no ESC telemetry: check their ESCs' telemetry connection and numbering."
            ])
        );
    }

    fn fly(escs: &mut crate::sensorfacts::Escs, amps: [u16; 6], packets: [u16; 6], now_ms: u64) {
        use mavlink::dialects::ardupilotmega::{
            ESC_TELEMETRY_9_TO_12_DATA, ESC_TELEMETRY_13_TO_16_DATA, MavMessage,
        };
        escs.apply(
            &MavMessage::ESC_TELEMETRY_9_TO_12(ESC_TELEMETRY_9_TO_12_DATA {
                current: [amps[0] * 100, amps[1] * 100, amps[2] * 100, amps[3] * 100],
                count: [packets[0], packets[1], packets[2], packets[3]],
                ..Default::default()
            }),
            now_ms,
        );
        escs.apply(
            &MavMessage::ESC_TELEMETRY_13_TO_16(ESC_TELEMETRY_13_TO_16_DATA {
                current: [amps[4] * 100, amps[5] * 100, 0, 0],
                count: [packets[4], packets[5], 0, 0],
                ..Default::default()
            }),
            now_ms,
        );
    }

    #[test]
    fn an_esc_whose_packet_count_stops_goes_silent_on_the_frame_and_offline_on_the_toolbar() {
        let mut escs = crate::sensorfacts::Escs::default();
        fly(&mut escs, [10; 6], [1; 6], 0);
        fly(&mut escs, [10; 6], [2, 2, 2, 2, 2, 1], 1000);
        let fresh = esc_view(
            &Vampire {
                escs: escs.clone(),
                now_ms: 1000,
            },
            &[],
        );
        assert_eq!(
            (
                fresh["healthText"].as_str(),
                fresh["healthyMotorsText"].as_str()
            ),
            (Some("OK"), Some("6/6"))
        );
        fly(&mut escs, [10; 6], [3, 3, 3, 3, 3, 1], 3500);
        let stale = esc_view(&Vampire { escs, now_ms: 3500 }, &[]);
        assert_eq!(
            stale["frame"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["state"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "reporting",
                "reporting",
                "reporting",
                "reporting",
                "reporting",
                "lost"
            ],
            "ArduPilot keeps resending a dead ESC's last numbers; only its packet count stops, and a motor heard before is lost, not unwired"
        );
        assert_eq!(
            (
                stale["healthText"].as_str(),
                stale["motors"][0]["title"].as_str(),
                stale["motors"][0]["motor"].as_i64(),
                stale["hints"].clone(),
                stale["motors"][0]["reasons"].clone()
            ),
            (
                Some("ERR"),
                Some("Motor 6 - OFFLINE"),
                Some(6),
                json!(["Motor 6 stopped sending ESC telemetry."]),
                json!(["Motor 6 stopped sending ESC telemetry."])
            ),
            "the summary and the motor's own entry say the same words"
        );
    }

    #[test]
    fn current_is_judged_against_the_other_motors_once_the_difference_holds() {
        let levels = |escs: &crate::sensorfacts::Escs, now_ms: u64| {
            esc_view(
                &Vampire {
                    escs: escs.clone(),
                    now_ms,
                },
                &[],
            )["frame"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["currentLevel"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        let mut lifter = crate::sensorfacts::Escs::default();
        fly(&mut lifter, [60; 6], [1; 6], 0);
        assert!(
            levels(&lifter, 0).iter().all(|level| level == "normal"),
            "a heavy lifter at 60 A a motor is level when every motor carries it"
        );
        let mut hexa = crate::sensorfacts::Escs::default();
        fly(&mut hexa, [30; 6], [1; 6], 0);
        fly(&mut hexa, [60, 30, 30, 30, 30, 30], [2; 6], 200);
        assert_eq!(
            levels(&hexa, 200)[0],
            "normal",
            "a yaw kick of a fifth of a second is not an imbalance"
        );
        let held = (3..=8).fold(hexa, |mut escs, step| {
            fly(
                &mut escs,
                [60, 30, 30, 30, 30, 30],
                [step; 6],
                u64::from(step) * 1000,
            );
            escs
        });
        assert_eq!(
            levels(&held, 8000),
            vec!["critical", "normal", "normal", "normal", "normal", "normal"]
        );
        let shown = esc_view(
            &Vampire {
                escs: held,
                now_ms: 8000,
            },
            &[],
        );
        assert_eq!(
            (
                shown["healthText"].as_str(),
                shown["hints"].clone(),
                shown["motors"][0]["motor"].as_i64()
            ),
            (
                Some("ERR"),
                json!([
                    "Motor 1 draws 60 A while the others draw about 30 A: check its propeller and motor."
                ]),
                Some(1)
            ),
            "a red motor on the frame is never OK on the toolbar, and the sheet says why"
        );
    }

    #[test]
    fn a_hot_motor_says_how_hot_against_the_limit() {
        let hot = Rotor {
            motor: 3,
            angle: 0.0,
            at: Some(2),
            state: "reporting",
            temperature: 92.0,
            current: 31.0,
            rpm: 7400.0,
            temperature_level: "critical",
            current_level: "normal",
        };
        assert_eq!(
            (hot.level(), hot.reasons(Some(31.0))),
            (
                "critical",
                vec![(
                    2,
                    "Motor 3 is at 92 \u{b0}C, past the 90 \u{b0}C limit.".to_string()
                )]
            )
        );
    }
}
