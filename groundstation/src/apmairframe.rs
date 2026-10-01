use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.apmFirmware", "vehicle.multiRotor", "vehicle.rover", "vehicle.sub"];
pub const AIRFRAME_SCREEN: &str = "apmAirframe";
pub const PICK_CLASS: &str = "apmAirframe.pickClass";
pub const PICK_TYPE: &str = "apmAirframe.pickType";
const FRAME_CLASS: &str = "FRAME_CLASS";
const FRAME_TYPE: &str = "FRAME_TYPE";
const UNKNOWN_IMAGE: &str = "AirframeUnknown";
pub const INVALID_TYPE: &str = "Invalid setting for FRAME_TYPE. Click to Reset.";

const CLASS_QUAD: i64 = 1;
const CLASS_HEX: i64 = 2;
const CLASS_OCTA: i64 = 3;
const CLASS_OCTAQUAD: i64 = 4;
const CLASS_Y6: i64 = 5;
const CLASS_HELI: i64 = 6;
const CLASS_TRI: i64 = 7;
const CLASS_DODECAHEXA: i64 = 12;
const CLASS_ROVER: i64 = 1;
const CLASS_BOAT: i64 = 2;
const TYPE_PLUS: i64 = 0;
const TYPE_X: i64 = 1;
const TYPE_V: i64 = 2;
const TYPE_H: i64 = 3;
const TYPE_V_TAIL: i64 = 4;
const TYPE_A_TAIL: i64 = 5;
const TYPE_Y6B: i64 = 10;
const TYPE_Y6F: i64 = 11;
const ANY_TYPE: i64 = -1;

const COPTER_IMAGES: &[(i64, i64, &str)] = &[
    (CLASS_QUAD, TYPE_X, "QuadRotorX"),
    (CLASS_QUAD, TYPE_PLUS, "QuadRotorPlus"),
    (CLASS_QUAD, TYPE_V, "QuadRotorWide"),
    (CLASS_QUAD, TYPE_H, "QuadRotorH"),
    (CLASS_QUAD, TYPE_V_TAIL, "QuadRotorVTail"),
    (CLASS_QUAD, TYPE_A_TAIL, "QuadRotorATail"),
    (CLASS_HEX, TYPE_X, "HexaRotorX"),
    (CLASS_HEX, TYPE_PLUS, "HexaRotorPlus"),
    (CLASS_OCTA, TYPE_X, "OctoRotorX"),
    (CLASS_OCTA, TYPE_PLUS, "OctoRotorPlus"),
    (CLASS_OCTA, TYPE_V, UNKNOWN_IMAGE),
    (CLASS_OCTA, TYPE_H, UNKNOWN_IMAGE),
    (CLASS_OCTAQUAD, TYPE_X, "OctoRotorXCoaxial"),
    (CLASS_OCTAQUAD, TYPE_PLUS, "OctoRotorPlusCoaxial"),
    (CLASS_OCTAQUAD, TYPE_V, UNKNOWN_IMAGE),
    (CLASS_OCTAQUAD, TYPE_H, UNKNOWN_IMAGE),
    (CLASS_Y6, TYPE_Y6B, "Y6B"),
    (CLASS_Y6, TYPE_Y6F, UNKNOWN_IMAGE),
    (CLASS_Y6, ANY_TYPE, "Y6A"),
    (CLASS_DODECAHEXA, TYPE_X, UNKNOWN_IMAGE),
    (CLASS_DODECAHEXA, TYPE_PLUS, UNKNOWN_IMAGE),
    (CLASS_HELI, ANY_TYPE, "Helicopter"),
    (CLASS_TRI, ANY_TYPE, "YPlus"),
];

const ROVER_IMAGES: &[(i64, &str)] = &[(CLASS_ROVER, "Rover"), (CLASS_BOAT, "Boat")];

fn param_path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn present(fact: &Value) -> bool {
    fact.get("kind").and_then(Value::as_str) == Some("fact") && !text(fact, "name").is_empty()
}

fn raw(fact: &Value) -> Option<i64> {
    fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).map(|v| v as i64)
}

pub fn enum_pairs(fact: &Value) -> Vec<(String, i64)> {
    let synthetic = text(fact, "unknownEnumLabel");
    let labels = fact.get("enumStrings").and_then(Value::as_array).cloned().unwrap_or_default();
    let values = fact.get("enumValues").and_then(Value::as_array).cloned().unwrap_or_default();
    labels
        .iter()
        .zip(values.iter())
        .filter_map(|(label, value)| Some((label.as_str()?.to_string(), value.as_f64().or_else(|| value.as_str()?.parse().ok())? as i64)))
        .filter(|(label, _)| synthetic.is_empty() || *label != synthetic)
        .collect()
}

pub fn copter_image(class: i64, frame_type: i64) -> &'static str {
    COPTER_IMAGES
        .iter()
        .find(|(c, t, _)| *c == class && (frame_type == ANY_TYPE || *t == frame_type))
        .map_or(UNKNOWN_IMAGE, |(_, _, image)| image)
}

fn rover_image(class: i64) -> &'static str {
    ROVER_IMAGES.iter().find(|(c, _)| *c == class).map_or(UNKNOWN_IMAGE, |(_, image)| image)
}

#[derive(Debug, PartialEq)]
pub struct FrameClass {
    pub name: String,
    pub value: i64,
    pub default_type: Option<i64>,
    pub types: Vec<(String, i64)>,
}

pub fn frame_classes(classes: &[(String, i64)], types: &[(String, i64)], copter: bool) -> Vec<FrameClass> {
    classes
        .iter()
        .skip(1)
        .filter(|(_, value)| !(copter && *value == CLASS_HELI))
        .map(|(name, value)| {
            let rows: Vec<&(i64, i64, &str)> = COPTER_IMAGES.iter().filter(|(c, _, _)| copter && c == value).collect();
            FrameClass {
                name: name.clone(),
                value: *value,
                default_type: rows.first().map(|(_, t, _)| *t),
                types: rows.iter().filter(|(_, t, _)| *t != ANY_TYPE).filter_map(|(_, t, _)| types.iter().find(|(_, v)| v == t).cloned()).collect(),
            }
        })
        .collect()
}

pub fn type_valid(class: &FrameClass, frame_type: Option<i64>) -> bool {
    class.default_type.is_none() || class.types.iter().any(|(_, v)| Some(*v) == frame_type)
}

fn image(copter: bool, class: i64, frame_type: i64) -> String {
    format!("{}.svg", if copter { copter_image(class, frame_type) } else { rover_image(class) })
}

pub fn help_text(class_value: Option<i64>, class_label: &str, type_label: Option<&str>) -> String {
    match class_value {
        Some(0) | None => "Airframe is currently not set.".to_string(),
        Some(_) => format!(
            "Currently set to frame class '{class_label}'{}. To change this configuration, select the desired frame class below and then reboot the vehicle.",
            type_label.map(|t| format!(" and frame type '{t}'")).unwrap_or_default()
        ),
    }
}

struct Read {
    copter: bool,
    rover: bool,
    class_fact: Value,
    type_fact: Value,
}

fn read(backend: &dyn Backend) -> Option<Read> {
    let vehicle = object(&backend.get_fields("vehicle", "apmFirmware,sub,multiRotor,rover"));
    let class_fact = object(&backend.get(&param_path(FRAME_CLASS)));
    (flag(&vehicle, "apmFirmware") && !flag(&vehicle, "sub") && present(&class_fact)).then(|| Read {
        copter: flag(&vehicle, "multiRotor"),
        rover: flag(&vehicle, "rover"),
        class_fact,
        type_fact: object(&backend.get(&param_path(FRAME_TYPE))),
    })
}

fn classes_of(read: &Read) -> Vec<FrameClass> {
    match read.copter || read.rover {
        true => frame_classes(&enum_pairs(&read.class_fact), &enum_pairs(&read.type_fact), read.copter),
        false => Vec::new(),
    }
}

pub fn apm_airframe_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(read) = read(backend) else { return json!({ "kind": "object", "class": "ApmAirframe", "available": false }) };
    let class_value = raw(&read.class_fact);
    let type_value = present(&read.type_fact).then(|| raw(&read.type_fact)).flatten();
    let type_label = text(&read.type_fact, "enumStringValue");
    json!({
        "kind": "object",
        "class": "ApmAirframe",
        "available": true,
        "help": help_text(class_value, &text(&read.class_fact, "enumStringValue"), read.copter.then_some(type_label.as_str())),
        "frameClass": class_value,
        "frameType": type_value,
        "invalidText": INVALID_TYPE,
        "classes": classes_of(&read).iter().map(|c| {
            let chosen = Some(c.value) == class_value;
            json!({
                "name": c.name,
                "value": c.value,
                "chosen": chosen,
                "image": image(read.copter, c.value, if chosen { type_value.unwrap_or(ANY_TYPE) } else { ANY_TYPE }),
                "types": c.types.iter().map(|(name, value)| json!({ "name": name, "value": value })).collect::<Vec<_>>(),
                "valid": !chosen || type_valid(c, type_value),
            })
        }).collect::<Vec<_>>(),
    })
}

fn write(backend: &dyn Backend, name: &str, value: i64) -> Value {
    object(&backend.set(&format!("{}.rawValue", param_path(name)), &json!({ "value": value }).to_string()))
}

pub fn run(backend: &dyn Backend, action: &str, args: &str) -> Value {
    let Some(read) = read(backend) else { return json!({ "ok": false, "reason": "This page is for an ArduPilot copter or rover." }) };
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let picked = given.first().and_then(Value::as_i64);
    let classes = classes_of(&read);
    match action {
        PICK_CLASS => {
            let Some(class) = classes.iter().find(|c| Some(c.value) == picked) else { return json!({ "ok": false, "reason": "Pick one of the listed frame classes." }) };
            let current_type = raw(&read.type_fact);
            let unchanged = raw(&read.class_fact) == Some(class.value) && type_valid(class, current_type);
            match (unchanged, read.copter && present(&read.type_fact), class.default_type.filter(|t| *t != ANY_TYPE)) {
                (true, _, _) => json!({ "ok": true }),
                (false, true, Some(default_type)) => {
                    let typed = write(backend, FRAME_TYPE, default_type);
                    match typed.get("ok").and_then(Value::as_bool) {
                        Some(false) => typed,
                        _ => write(backend, FRAME_CLASS, class.value),
                    }
                }
                _ => write(backend, FRAME_CLASS, class.value),
            }
        }
        PICK_TYPE => {
            let chosen = classes.iter().find(|c| Some(c.value) == raw(&read.class_fact));
            match chosen.and_then(|c| c.types.iter().find(|(_, v)| Some(*v) == picked)) {
                Some((_, value)) => write(backend, FRAME_TYPE, *value),
                None => json!({ "ok": false, "reason": "Pick one of the frame types listed for this class." }),
            }
        }
        _ => json!({ "ok": false, "reason": format!("{action} is not an airframe action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [PICK_CLASS, PICK_TYPE].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, i64)]) -> Vec<(String, i64)> {
        list.iter().map(|(n, v)| (n.to_string(), *v)).collect()
    }

    fn classes() -> Vec<(String, i64)> {
        pairs(&[("Undefined", 0), ("Quad", 1), ("Hexa", 2), ("Y6", 5), ("Heli", 6), ("Tri", 7)])
    }

    fn types() -> Vec<(String, i64)> {
        pairs(&[("Plus", 0), ("X", 1), ("V", 2), ("H", 3), ("V-Tail", 4), ("A-Tail", 5), ("Y6B", 10), ("BetaFlightX", 12)])
    }

    #[test]
    fn copter_classes_skip_undefined_and_heli_and_offer_only_the_types_their_layout_supports() {
        let listed = frame_classes(&classes(), &types(), true);
        assert_eq!(listed.iter().map(|c| c.value).collect::<Vec<_>>(), [1, 2, 5, 7]);
        let quad = &listed[0];
        assert_eq!(quad.default_type, Some(TYPE_X), "the first row of the table is the default");
        assert_eq!(quad.types.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [1, 0, 2, 3, 4, 5], "BetaFlightX is in the enum but not in the table");
        assert_eq!(listed[2].types.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [10], "Y6F is missing from this enum and the any-type row is not a choice");
        assert_eq!(listed[3].default_type, Some(ANY_TYPE));
        assert!(listed[3].types.is_empty(), "Tri hides the frame type list");
    }

    #[test]
    fn a_type_outside_the_class_list_is_invalid_and_rovers_never_are() {
        let listed = frame_classes(&classes(), &types(), true);
        assert!(type_valid(&listed[0], Some(TYPE_H)));
        assert!(!type_valid(&listed[1], Some(TYPE_H)), "Hexa has no H layout");
        assert!(!type_valid(&listed[0], None));
        let rovers = frame_classes(&pairs(&[("Undefined", 0), ("Rover", 1), ("Boat", 2), ("BalanceBot", 3)]), &[], false);
        assert_eq!(rovers.len(), 3, "rover lists every class including the last");
        assert!(rovers.iter().all(|c| c.default_type.is_none() && type_valid(c, Some(99))));
    }

    #[test]
    fn images_and_help_follow_the_controller() {
        assert_eq!(copter_image(CLASS_QUAD, TYPE_H), "QuadRotorH");
        assert_eq!(copter_image(CLASS_QUAD, ANY_TYPE), "QuadRotorX");
        assert_eq!(copter_image(CLASS_Y6, 3), UNKNOWN_IMAGE, "the any-type row only answers a lookup without a type");
        assert_eq!(copter_image(CLASS_HEX, TYPE_H), UNKNOWN_IMAGE);
        assert_eq!(image(false, CLASS_BOAT, ANY_TYPE), "Boat.svg");
        assert_eq!(help_text(Some(0), "Undefined", Some("X")), "Airframe is currently not set.");
        assert_eq!(
            help_text(Some(1), "Quad", Some("X")),
            "Currently set to frame class 'Quad' and frame type 'X'. To change this configuration, select the desired frame class below and then reboot the vehicle."
        );
        assert_eq!(help_text(Some(1), "Rover", None), "Currently set to frame class 'Rover'. To change this configuration, select the desired frame class below and then reboot the vehicle.");
    }
}
