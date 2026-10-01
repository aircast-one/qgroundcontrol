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
pub struct Function {
    pub label: String,
    pub note: String,
    pub note_if: Condition,
    pub exclude_from_testing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActuatorType {
    pub name: String,
    pub function_min: i64,
    pub function_max: i64,
    pub min: f64,
    pub max: f64,
    pub default: Option<f64>,
    pub per_item: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerParam {
    pub param: Param,
    pub identifier: String,
    pub function: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerGroup {
    pub label: String,
    pub count_param: String,
    pub fixed_count: i64,
    pub actuator_type: String,
    pub required: bool,
    pub params: Vec<Param>,
    pub per_item: Vec<MixerParam>,
    pub item_label_prefix: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerOption {
    pub option: Condition,
    pub kind: String,
    pub title: String,
    pub help_url: String,
    pub groups: Vec<MixerGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub show_ui_if: Condition,
    pub outputs: Vec<Output>,
    pub functions: std::collections::BTreeMap<i64, Function>,
    pub actuator_types: Vec<ActuatorType>,
    pub mixer: Vec<MixerOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestActuator {
    pub label: String,
    pub function: i64,
    pub min: f64,
    pub max: f64,
    pub default: Option<f64>,
    pub is_motor: bool,
}

pub const TEST_ACTIVE: &str = "actuatorTest.setActive";
pub const TEST_SET: &str = "actuatorTest.setChannelTo";
pub const TEST_STOP: &str = "actuatorTest.stopControl";

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
    let functions = json
        .get("functions_v1")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, f)| {
            let label = text(f, "label");
            (!label.is_empty()).then(|| {
                let note = f.get("note").cloned().unwrap_or(Value::Null);
                Some((key.parse::<i64>().ok()?, Function { label, note: text(&note, "text"), note_if: Condition::parse(&text(&note, "condition")), exclude_from_testing: f.get("exclude-from-actuator-testing").and_then(Value::as_bool).unwrap_or(false) }))
            })?
        })
        .collect();
    let mut actuator_types: Vec<ActuatorType> = json
        .pointer("/mixer_v1/actuator-types")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, t)| {
            let values = t.get("values").cloned().unwrap_or(Value::Null);
            let number = |v: &Value, key: &str| v.get(key).and_then(Value::as_f64).unwrap_or(0.0);
            ActuatorType {
                name: name.clone(),
                function_min: t.get("function-min").and_then(Value::as_i64).unwrap_or(0),
                function_max: t.get("function-max").and_then(Value::as_i64).unwrap_or(0),
                min: number(&values, "min"),
                max: number(&values, "max"),
                default: (!values.get("default-is-nan").and_then(Value::as_bool).unwrap_or(false)).then(|| number(&values, "default")),
                per_item: list(t, "per-item-parameters").map(Param::parse).collect(),
            }
        })
        .collect();
    actuator_types.sort_by(|a, b| a.name.cmp(&b.name));
    let mixer = json
        .pointer("/mixer_v1/config")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|option| MixerOption {
            option: Condition::parse(&text(option, "option")),
            kind: text(option, "type"),
            title: text(option, "title"),
            help_url: text(option, "help-url"),
            groups: list(option, "actuators")
                .map(|group| {
                    let fixed_count = group.get("count").and_then(Value::as_i64).unwrap_or(0);
                    MixerGroup {
                        label: text(group, "group-label"),
                        count_param: group.get("count").and_then(Value::as_str).unwrap_or("").to_string(),
                        fixed_count,
                        actuator_type: text(group, "actuator-type"),
                        required: group.get("required").and_then(Value::as_bool).unwrap_or(false),
                        params: list(group, "parameters").map(Param::parse).collect(),
                        per_item: list(group, "per-item-parameters")
                            .map(|item| MixerParam {
                                param: Param::parse(item),
                                identifier: text(item, "identifier"),
                                function: text(item, "function"),
                                values: list(item, "value").filter_map(Value::as_f64).collect(),
                            })
                            .filter(|item| !item.param.name.is_empty() || item.values.len() == 1 || item.values.len() as i64 == fixed_count)
                            .collect(),
                        item_label_prefix: match group.get("item-label-prefix") {
                            Some(Value::String(one)) => vec![one.clone()],
                            other => other.and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect(),
                        },
                    }
                })
                .collect(),
        })
        .collect();
    Ok(Metadata { show_ui_if: Condition::parse(&text(json, "show-ui-if")), outputs, functions, actuator_types, mixer })
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellSource {
    Parameter(String, i64),
    Fixed(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerCell {
    pub config: Param,
    pub function: String,
    pub source: CellSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerChannel {
    pub label: String,
    pub function: i64,
    pub cells: Vec<MixerCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerGroupState {
    pub group: MixerGroup,
    pub channels: Vec<MixerChannel>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MixerState {
    pub option: Option<MixerOption>,
    pub groups: Vec<MixerGroupState>,
    pub specific_labels: std::collections::BTreeMap<i64, String>,
}

pub fn mixer_state(metadata: &Metadata, value_of: &dyn Fn(&str) -> Option<i64>) -> MixerState {
    let Some(option) = metadata.mixer.iter().find(|o| o.option.evaluate(value_of)) else { return MixerState::default() };
    let (groups, specific_labels, _) = option.groups.iter().fold((Vec::new(), std::collections::BTreeMap::new(), std::collections::BTreeMap::<String, i64>::new()), |(groups, labels, type_counts), group| {
        let count = match group.count_param.is_empty() {
            true => group.fixed_count,
            false => value_of(&group.count_param).unwrap_or(group.fixed_count),
        };
        let start = type_counts.get(&group.actuator_type).copied().unwrap_or(0);
        let kind = metadata.actuator_types.iter().find(|t| t.name == group.actuator_type);
        let channels: Vec<(MixerChannel, Option<(i64, String)>)> = (0..count.max(0))
            .map(|index| {
                let type_index = start + index;
                let function = kind.map_or(0, |t| t.function_min + type_index);
                let base = metadata.functions.get(&function).map(|f| f.label.clone()).unwrap_or_default();
                let prefix = match group.item_label_prefix.len() {
                    1 => group.item_label_prefix[0].replace("${i}", &(index + 1).to_string()),
                    _ => group.item_label_prefix.get(index as usize).cloned().unwrap_or_default(),
                };
                let label = if kind.is_some() && !prefix.is_empty() { format!("{prefix} ({base})") } else { base };
                let type_cells = kind.into_iter().flat_map(|t| t.per_item.iter()).map(|config| MixerCell {
                    config: config.clone(),
                    function: String::new(),
                    source: CellSource::Parameter(config.name.replace("${i}", &(type_index + config.index_offset).to_string()), type_index + config.index_offset),
                });
                let item_cells = group.per_item.iter().map(|item| MixerCell {
                    config: item.param.clone(),
                    function: item.function.clone(),
                    source: match item.param.name.is_empty() {
                        true => CellSource::Fixed(if item.values.len() == 1 { item.values[0] } else { item.values.get(index as usize).copied().unwrap_or(0.0) }),
                        false => CellSource::Parameter(item.param.name.replace("${i}", &(index + item.param.index_offset).to_string()), index + item.param.index_offset),
                    },
                });
                let specific = (kind.is_some() && !prefix.is_empty()).then_some((function, prefix));
                (MixerChannel { label, function, cells: type_cells.chain(item_cells).collect() }, specific)
            })
            .collect();
        let labels = labels.into_iter().chain(channels.iter().filter_map(|(_, specific)| specific.clone())).collect();
        let counts = type_counts.into_iter().chain(std::iter::once((group.actuator_type.clone(), start + count.max(0)))).collect();
        let state = MixerGroupState { group: group.clone(), channels: channels.into_iter().map(|(channel, _)| channel).collect() };
        (groups.into_iter().chain(std::iter::once(state)).collect(), labels, counts)
    });
    MixerState { option: Some(option.clone()), groups, specific_labels }
}

pub fn specific_label(metadata: &Metadata, mixer: &MixerState, function: i64, enum_text: &dyn Fn(&str) -> Option<(i64, String)>) -> String {
    let type_param = |channel: &MixerChannel| {
        channel.cells.iter().find(|c| c.function == "type").and_then(|c| match &c.source {
            CellSource::Parameter(name, _) => enum_text(name),
            _ => None,
        })
    };
    let channels: Vec<&MixerChannel> = mixer.groups.iter().flat_map(|g| g.channels.iter()).collect();
    let typed = channels.iter().find(|c| c.function == function).and_then(|c| type_param(c));
    match typed {
        Some((raw, shown)) => match channels.iter().filter(|c| c.function != function).any(|c| type_param(c).is_some_and(|(other, _)| other == raw)) {
            true => format!("{shown} ({})", metadata.functions.get(&function).map(|f| f.label.as_str()).unwrap_or("")),
            false => shown,
        },
        None => mixer.specific_labels.get(&function).cloned().unwrap_or_else(|| metadata.functions.get(&function).map(|f| f.label.clone()).unwrap_or_default()),
    }
}

pub fn mixer_functions(mixer: &MixerState, required_only: bool) -> std::collections::BTreeSet<i64> {
    mixer.groups.iter().filter(|g| !required_only || g.group.required).flat_map(|g| g.channels.iter().map(|c| c.function)).filter(|f| *f != 0).collect()
}

fn function_params(output: &Output) -> Vec<String> {
    output
        .subgroups
        .iter()
        .flat_map(|subgroup| subgroup.channels.iter().flat_map(move |channel| subgroup.channel_configs.iter().filter(|c| c.function == "function").map(move |c| channel_param(c, channel))))
        .collect()
}

pub fn kept_outputs<'a>(metadata: &'a Metadata, exists: &dyn Fn(&str) -> bool) -> Vec<&'a Output> {
    metadata.outputs.iter().filter(|output| output.enable.is_some() || function_params(output).iter().any(|name| exists(name))).collect()
}

pub fn configured_functions(metadata: &Metadata, value_of: &dyn Fn(&str) -> Option<i64>) -> std::collections::BTreeSet<i64> {
    metadata.outputs.iter().flat_map(function_params).filter_map(|name| value_of(&name)).filter(|f| *f != 0).collect()
}

pub fn test_actuators(metadata: &Metadata, value_of: &dyn Fn(&str) -> Option<i64>, label_of: &dyn Fn(i64) -> String) -> Vec<TestActuator> {
    configured_functions(metadata, value_of)
        .into_iter()
        .filter(|f| !metadata.functions.get(f).is_some_and(|known| known.exclude_from_testing))
        .filter_map(|function| {
            let label = label_of(function);
            let typed = metadata.actuator_types.iter().find(|t| (t.function_min..=t.function_max).contains(&function));
            let (kind, is_motor) = match typed {
                Some(t) => (t, t.name == "motor"),
                None => (metadata.actuator_types.iter().find(|t| t.name == "DEFAULT")?, false),
            };
            Some(TestActuator { label, function, min: kind.min, max: kind.max, default: kind.default, is_motor })
        })
        .collect()
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

fn actuator_json(actuator: &TestActuator) -> Value {
    json!({ "label": actuator.label, "function": actuator.function, "min": actuator.min, "max": actuator.max, "default": actuator.default, "isMotor": actuator.is_motor })
}

pub fn outputs_json(backend: &dyn Backend, metadata: &Metadata) -> Value {
    let value_of = |name: &str| integer(backend, name);
    let exists = |name: &str| fact(backend, name).is_some();
    let mixer = mixer_state(metadata, &value_of);
    let enum_text = |name: &str| {
        let read = fact(backend, name)?;
        Some((integer(backend, name)?, read.get("valueString").and_then(Value::as_str).unwrap_or("").to_string()))
    };
    let label_of = |function: i64| specific_label(metadata, &mixer, function, &enum_text);
    let testing = test_actuators(metadata, &value_of, &label_of);
    let configured = configured_functions(metadata, &value_of);
    let all_motors = testing.iter().rev().find(|a| a.is_motor).map(|motor| TestActuator { label: "All Motors".to_string(), ..motor.clone() });
    json!({
        "showUi": metadata.show_ui_if.evaluate(&value_of),
        "testing": {
            "actuators": testing.iter().map(actuator_json).collect::<Vec<_>>(),
            "allMotors": all_motors.as_ref().map(actuator_json),
            "hadFailure": crate::hub::lock().active().is_some_and(|v| v.actuator_test.had_failure),
        },
        "hasUnsetRequiredFunctions": mixer_functions(&mixer, true).iter().any(|f| !configured.contains(f)),
        "geometry": geometry_json(backend, &mixer),
        "groups": kept_outputs(metadata, &exists).into_iter().map(|output| json!({
            "label": output.label,
            "notes": function_params(output).iter().filter_map(|name| value_of(name)).filter_map(|f| metadata.functions.get(&f)).filter(|f| !f.note.is_empty() && f.note_if.evaluate(&value_of)).map(|f| f.note.clone()).collect::<Vec<_>>(),
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

fn cell_json(backend: &dyn Backend, cell: &MixerCell) -> Value {
    match &cell.source {
        CellSource::Parameter(name, index) => control(backend, &cell.config, name, *index).unwrap_or(Value::Null),
        CellSource::Fixed(value) => json!({ "fixed": true, "label": cell.config.label, "valueString": format!("{value:.4}"), "advanced": cell.config.advanced }),
    }
}

fn geometry_json(backend: &dyn Backend, mixer: &MixerState) -> Value {
    let Some(option) = mixer.option.as_ref() else { return Value::Null };
    json!({
        "title": if option.title.is_empty() { "Geometry".to_string() } else { format!("Geometry: {}", option.title) },
        "helpUrl": option.help_url,
        "type": option.kind,
        "groups": mixer.groups.iter().map(|state| json!({
            "label": state.group.label,
            "count": (!state.group.count_param.is_empty()).then(|| fact(backend, &state.group.count_param).map(|f| decode(&f, &parameter_path(&state.group.count_param)))).flatten(),
            "columns": state.channels.first().map(|c| c.cells.iter().map(|cell| json!({ "label": cell.config.label, "advanced": cell.config.advanced })).collect::<Vec<_>>()).unwrap_or_default(),
            "channels": state.channels.iter().map(|channel| json!({ "label": channel.label, "cells": channel.cells.iter().map(|cell| cell_json(backend, cell)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
            "params": params_json(backend, &state.group.params),
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

pub fn test_action(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let action = match path {
        TEST_ACTIVE => json!({ "action": "actuatorTest", "op": "active", "on": given.get(0).and_then(Value::as_bool).unwrap_or(false) }),
        TEST_SET => json!({ "action": "actuatorTest", "op": "set", "function": given.get(0), "value": given.get(1) }),
        _ => json!({ "action": "actuatorTest", "op": "stop", "function": given.get(0).filter(|f| f.as_i64() != Some(-1)) }),
    };
    crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), path, args)
}

pub fn owns(path: &str) -> bool {
    [TEST_ACTIVE, TEST_SET, TEST_STOP].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selected_mixer_numbers_its_actuators_and_names_them_from_the_prefixes() {
        let parsed = parse(&example()).unwrap();
        let quad = |name: &str| match name {
            "CA_AIRFRAME" => Some(1),
            "CA_MC_R_COUNT" => Some(4),
            _ => None,
        };
        let state = mixer_state(&parsed, &quad);
        assert_eq!(state.option.as_ref().map(|o| o.kind.as_str()), Some("multirotor"));
        let motors = &state.groups[0];
        assert_eq!(motors.channels.iter().map(|c| (c.function, c.label.as_str())).collect::<Vec<_>>(), [(101, "Motor 1"), (102, "Motor 2"), (103, "Motor 3"), (104, "Motor 4")]);
        assert_eq!(motors.channels[1].cells.iter().map(|c| c.config.label.as_str()).collect::<Vec<_>>(), ["Reversible", "Position X", "Position Y", "Position Z", "Direction CCW", "Axis X"], "the actuator type's per-item parameters come first");
        assert_eq!(motors.channels[1].cells[0].source, CellSource::Parameter("CA_MOT_REV".into(), 0), "a type parameter counts by type index plus its offset");
        assert_eq!(motors.channels[1].cells[1].source, CellSource::Parameter("CA_MC_R1_PX".into(), 1));
        assert_eq!(mixer_functions(&state, true), [101, 102, 103, 104].into_iter().collect());

        let tilt = |name: &str| (name == "CA_AIRFRAME").then_some(5);
        let vtol = mixer_state(&parsed, &tilt);
        assert_eq!(vtol.groups[0].channels[2].label, "Rear Motor (Motor 3)");
        assert_eq!(vtol.groups[1].channels[0].function, 201, "servos number from their own type");
        assert_eq!(vtol.groups[0].channels[2].cells.iter().find(|c| c.config.label == "Position X").map(|c| c.source.clone()), Some(CellSource::Fixed(-0.75)));
        let none = |_: &str| None;
        assert_eq!(specific_label(&parsed, &vtol, 103, &none), "Rear Motor");
        assert_eq!(specific_label(&parsed, &vtol, 407, &none), "RC AUX 1");
        assert!(mixer_state(&parsed, &|_| Some(9)).option.is_none());
    }

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
        assert_eq!(parsed.functions[&101].label, "Motor 1");
        assert!(parsed.functions[&2032].exclude_from_testing);
        assert_eq!(parsed.actuator_types.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["DEFAULT", "motor", "servo"], "QMap order puts DEFAULT first");
    }

    #[test]
    fn the_test_list_is_the_configured_functions_once_each_typed_by_range() {
        let parsed = parse(&example()).unwrap();
        let functions = |name: &str| match name {
            "PWM_MAIN_FUNC1" => Some(101),
            "PWM_MAIN_FUNC2" => Some(102),
            "PWM_MAIN_FUNC3" => Some(201),
            "PWM_MAIN_FUNC4" => Some(2032),
            "PWM_AUX_FUNC1" => Some(101),
            "PWM_AUX_FUNC2" => Some(407),
            _ => Some(0),
        };
        let generic = |f: i64| parsed.functions.get(&f).map(|x| x.label.clone()).unwrap_or_default();
        let listed = test_actuators(&parsed, &functions, &generic);
        assert_eq!(listed.iter().map(|a| (a.function, a.label.as_str(), a.is_motor)).collect::<Vec<_>>(), [(101, "Motor 1", true), (102, "Motor 2", true), (201, "Servo 1", false), (407, "RC AUX 1", false)], "sorted, unique, camera capture excluded, RC AUX falls to DEFAULT");
        assert_eq!((listed[0].min, listed[0].max, listed[0].default), (0.0, 1.0, None), "a motor's default is NaN");
        assert_eq!((listed[3].min, listed[3].default), (-1.0, Some(-1.0)));
        assert!(kept_outputs(&parsed, &|_| false).iter().all(|o| o.enable.is_some()), "a group with no enable and no function parameter the vehicle has is dropped");
    }
}
