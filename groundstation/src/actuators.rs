use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::control::decode;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", crate::coreplan::CHANGED];

static CONDITION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9A-Za-z_-]+)([!=<>]+)(-?\d+)$").expect("condition pattern"));

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    AlwaysTrue,
    AlwaysFalse,
    Compare { parameter: String, operation: String, value: i64 },
}

impl Condition {
    pub fn parse(text: &str) -> Condition {
        match text {
            "" | "true" => Condition::AlwaysTrue,
            "false" => Condition::AlwaysFalse,
            _ => CONDITION.captures(text).map_or(Condition::AlwaysTrue, |c| {
                let operation = c[2].to_string();
                match ["==", "!=", ">", ">=", "<", "<="].contains(&operation.as_str()) {
                    true => Condition::Compare { parameter: c[1].to_string(), operation, value: c[3].parse().unwrap_or(0) },
                    false => Condition::AlwaysTrue,
                }
            }),
        }
    }

    pub fn evaluate(&self, value_of: &dyn Fn(&str) -> Option<i64>) -> bool {
        match self {
            Condition::AlwaysTrue => true,
            Condition::AlwaysFalse => false,
            Condition::Compare { parameter, operation, value } => value_of(parameter).is_some_and(|held| match operation.as_str() {
                ">" => held > *value,
                ">=" => held >= *value,
                "==" => held == *value,
                "!=" => held != *value,
                "<" => held < *value,
                _ => held <= *value,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub label: String,
    pub name: String,
    pub index_offset: i64,
    pub show_as: String,
    pub advanced: bool,
    pub function: String,
    pub show_if: Condition,
}

fn text(json: &Value, key: &str) -> String {
    json.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn list<'a>(json: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    json.get(key).and_then(Value::as_array).into_iter().flatten()
}

impl Param {
    fn parse(json: &Value) -> Param {
        Param {
            label: text(json, "label"),
            name: text(json, "name"),
            index_offset: json.get("index-offset").and_then(Value::as_i64).unwrap_or(0),
            show_as: text(json, "show-as"),
            advanced: json.get("advanced").and_then(Value::as_bool).unwrap_or(false),
            function: text(json, "function"),
            show_if: Condition::parse(&text(json, "show-if")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub label: String,
    pub param_index: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subgroup {
    pub label: String,
    pub primary: Option<Param>,
    pub params: Vec<Param>,
    pub channel_configs: Vec<Param>,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub label: String,
    pub show_subgroups_if: Condition,
    pub enable: Option<Param>,
    pub params: Vec<Param>,
    pub subgroups: Vec<Subgroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub show_ui_if: Condition,
    pub outputs: Vec<Output>,
}

pub fn parse(json: &Value) -> Result<Metadata, String> {
    let missing: Vec<&str> = ["outputs_v1", "functions_v1", "mixer_v1"].into_iter().filter(|key| json.get(key).is_none_or(Value::is_null)).collect();
    if !missing.is_empty() {
        return Err(format!("Missing required JSON sections: {}", missing.join(", ")));
    }
    let outputs = list(json, "outputs_v1")
        .map(|output| {
            let params: Vec<Param> = list(output, "parameters").map(Param::parse).collect();
            Output {
                label: text(output, "label"),
                show_subgroups_if: Condition::parse(&text(output, "show-subgroups-if")),
                enable: params.iter().rev().find(|p| p.function == "enable").cloned(),
                params: params.iter().filter(|p| p.function != "enable").cloned().collect(),
                subgroups: list(output, "subgroups")
                    .map(|subgroup| {
                        let config: Vec<Param> = list(subgroup, "parameters").map(Param::parse).collect();
                        Subgroup {
                            label: text(subgroup, "label"),
                            primary: config.iter().rev().find(|p| p.function == "primary").cloned(),
                            params: config.iter().filter(|p| p.function != "primary").cloned().collect(),
                            channel_configs: list(subgroup, "per-channel-parameters").map(Param::parse).collect(),
                            channels: list(subgroup, "channels").map(|c| Channel { label: text(c, "label"), param_index: c.get("param-index").and_then(Value::as_i64).unwrap_or(0) }).collect(),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(Metadata { show_ui_if: Condition::parse(&text(json, "show-ui-if")), outputs })
}

pub fn channel_param(config: &Param, channel: &Channel) -> String {
    config.name.replace("${i}", &(channel.param_index + config.index_offset).to_string())
}

fn parameter_path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn fact(backend: &dyn Backend, name: &str) -> Option<Value> {
    let read = object(&backend.get(&parameter_path(name)));
    (read.get("kind").and_then(Value::as_str) == Some("fact")).then_some(read)
}

fn integer(backend: &dyn Backend, name: &str) -> Option<i64> {
    let read = fact(backend, name)?;
    let raw = read.get("rawValue").or_else(|| read.get("value"))?;
    raw.as_i64().or_else(|| raw.as_bool().map(i64::from)).or_else(|| raw.as_f64().filter(|v| v.fract() == 0.0).map(|v| v as i64))
}

fn control(backend: &dyn Backend, param: &Param, name: &str, bit: i64) -> Option<Value> {
    let read = fact(backend, name)?;
    let mut decoded = decode(&read, &parameter_path(name));
    decoded["label"] = json!(param.label);
    decoded["showAs"] = json!(param.show_as);
    decoded["bit"] = json!(bit);
    decoded["advanced"] = json!(param.advanced);
    Some(decoded)
}

fn params_json(backend: &dyn Backend, params: &[Param]) -> Vec<Value> {
    params.iter().filter_map(|p| control(backend, p, &p.name, 0)).collect()
}

pub fn outputs_json(backend: &dyn Backend, metadata: &Metadata) -> Value {
    let value_of = |name: &str| integer(backend, name);
    json!({
        "showUi": metadata.show_ui_if.evaluate(&value_of),
        "groups": metadata.outputs.iter().map(|output| json!({
            "label": output.label,
            "enable": output.enable.as_ref().and_then(|p| control(backend, p, &p.name, 0)),
            "groupsVisible": output.show_subgroups_if.evaluate(&value_of),
            "params": params_json(backend, &output.params),
            "subgroups": output.subgroups.iter().map(|subgroup| json!({
                "label": subgroup.label,
                "primary": subgroup.primary.as_ref().and_then(|p| control(backend, p, &p.name, 0)),
                "params": params_json(backend, &subgroup.params),
                "columns": subgroup.channel_configs.iter().map(|c| json!({ "label": c.label, "advanced": c.advanced, "visible": c.show_if.evaluate(&value_of), "function": c.function })).collect::<Vec<_>>(),
                "channels": subgroup.channels.iter().map(|channel| json!({
                    "label": channel.label,
                    "configs": subgroup.channel_configs.iter().map(|config| control(backend, config, &channel_param(config, channel), channel.param_index + config.index_offset)).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

pub fn outputs_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let metadata = crate::hub::lock().active().and_then(|vehicle| vehicle.actuators_metadata.clone());
    let parsed = metadata.as_ref().map(parse);
    match parsed {
        Some(Ok(metadata)) => {
            let mut answer = outputs_json(backend, &metadata);
            answer["kind"] = json!("object");
            answer["class"] = json!("ActuatorOutputs");
            answer["available"] = json!(true);
            answer
        }
        Some(Err(reason)) => json!({ "kind": "object", "class": "ActuatorOutputs", "available": false, "reason": reason, "groups": [] }),
        None => json!({ "kind": "object", "class": "ActuatorOutputs", "available": false, "reason": "This vehicle sent no actuator metadata.", "groups": [] }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Value {
        serde_json::from_str(include_str!("../../test/Vehicle/ComponentInformation/actuators.example.json")).expect("the QGC example is JSON")
    }

    #[test]
    fn conditions_read_like_common_cc() {
        let values = |name: &str| match name {
            "PWM_MAIN_TIM0" => Some(-1),
            _ => None,
        };
        assert!(Condition::parse("").evaluate(&values) && Condition::parse("true").evaluate(&values));
        assert!(!Condition::parse("false").evaluate(&values));
        assert!(Condition::parse("PWM_MAIN_TIM0>=-1").evaluate(&values));
        assert!(!Condition::parse("PWM_MAIN_TIM0<-1").evaluate(&values));
        assert!(!Condition::parse("MISSING==1").evaluate(&values), "a parameter the vehicle lacks makes a comparison false");
        assert!(Condition::parse("what is this").evaluate(&values), "an unrecognised condition defaults to true");
    }

    #[test]
    fn the_example_outputs_parse_into_groups_subgroups_and_channels() {
        let parsed = parse(&example()).unwrap();
        let main = &parsed.outputs[0];
        assert_eq!(main.label, "MAIN");
        let first = &main.subgroups[0];
        assert_eq!(first.label, "MAIN 1-4");
        assert_eq!(first.primary.as_ref().map(|p| p.name.as_str()), Some("PWM_MAIN_TIM0"));
        assert_eq!(first.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["FMU_TIM0_EXTRA"]);
        assert_eq!(first.channel_configs.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["Function", "Disarmed", "Min", "Max", "Failsafe"]);
        assert_eq!(channel_param(&first.channel_configs[0], &first.channels[2]), "PWM_MAIN_FUNC3");
        assert!(first.channel_configs[1].advanced);
        assert!(parse(&json!({ "outputs_v1": [] })).unwrap_err().contains("functions_v1, mixer_v1"));
    }
}
