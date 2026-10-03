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
pub struct SupportedAction {
    pub kind: i64,
    pub label: &'static str,
    pub condition: Condition,
    pub actuator_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subgroup {
    pub actions: Vec<SupportedAction>,
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
    pub label_index_offset: i64,
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
    pub rules: Vec<Rule>,
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

const ACTION_KINDS: [(&str, i64, &str); 5] = [("beep", 1, "Beep"), ("3d-mode-on", 2, "3D mode: On"), ("3d-mode-off", 3, "3D mode: Off"), ("set-spin-direction1", 4, "Set Spin Direction 1"), ("set-spin-direction2", 5, "Set Spin Direction 2")];
pub const ACTION_TRIGGER: &str = "actuatorAction.trigger";
pub const MOTOR_INIT: &str = "motorAssignment.init";
pub const MOTOR_START: &str = "motorAssignment.start";
pub const MOTOR_SELECT: &str = "motorAssignment.selectMotor";
pub const MOTOR_SPIN: &str = "motorAssignment.spinCurrentMotor";
pub const MOTOR_ABORT: &str = "motorAssignment.abort";

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
                            actions: subgroup
                                .get("supported-actions")
                                .and_then(Value::as_object)
                                .into_iter()
                                .flatten()
                                .filter_map(|(name, action)| {
                                    let (kind, label) = ACTION_KINDS.iter().find(|(known, _, _)| known == name).map(|(_, kind, label)| (*kind, *label))?;
                                    Some(SupportedAction { kind, label, condition: Condition::parse(&text(action, "supported-if")), actuator_types: list(action, "actuator-types").filter_map(Value::as_str).map(str::to_string).collect() })
                                })
                                .collect(),
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
                label_index_offset: t.get("label-index-offset").and_then(Value::as_i64).unwrap_or(0),
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
    let rules = json
        .pointer("/mixer_v1/rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|rule| {
            let apply: Vec<String> = list(rule, "apply-identifiers").filter_map(Value::as_str).map(str::to_string).collect();
            let items = rule
                .get("items")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(key, items)| {
                    let parsed: Vec<RuleItem> = items.as_array()?.iter().map(|item| RuleItem {
                        min: item.get("min").and_then(Value::as_f64),
                        max: item.get("max").and_then(Value::as_f64),
                        default: item.get("default").and_then(Value::as_f64),
                        hidden: item.get("hidden").and_then(Value::as_bool).unwrap_or(false),
                        disabled: item.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                    }).collect();
                    (parsed.len() == apply.len()).then_some((key.parse::<i64>().ok()?, parsed))
                })
                .collect();
            Rule { select: text(rule, "select-identifier"), apply, items }
        })
        .collect();
    Ok(Metadata { show_ui_if: Condition::parse(&text(json, "show-ui-if")), outputs, functions, actuator_types, mixer, rules })
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
    pub identifier: String,
    pub source: CellSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleItem {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<f64>,
    pub hidden: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub select: String,
    pub apply: Vec<String>,
    pub items: std::collections::BTreeMap<i64, Vec<RuleItem>>,
}

pub const AXIS_DIRECTIONS: [&str; 7] = ["Custom", "Upwards", "Downwards", "Forwards", "Backwards", "Leftwards", "Rightwards"];
const AXIS_VECTORS: [(f64, f64, f64); 6] = [(0.0, 0.0, -1.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0), (-1.0, 0.0, 0.0), (0.0, -1.0, 0.0), (0.0, 1.0, 0.0)];
const AXIS_EPSILON: f64 = 0.00001;
pub const MIXER_SET: &str = "actuatorMixer.set";
pub const MIXER_AXIS: &str = "actuatorMixer.setAxis";

pub fn axis_direction(x: f64, y: f64, z: f64) -> usize {
    AXIS_VECTORS.iter().position(|(ax, ay, az)| (x - ax).abs() < AXIS_EPSILON && (y - ay).abs() < AXIS_EPSILON && (z - az).abs() < AXIS_EPSILON).map_or(0, |i| i + 1)
}

pub fn axis_vector(direction: usize) -> Option<(f64, f64, f64)> {
    direction.checked_sub(1).and_then(|i| AXIS_VECTORS.get(i)).copied()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerChannel {
    pub label: String,
    pub function: i64,
    pub type_index: i64,
    pub cells: Vec<MixerCell>,
    pub rule: Option<Rule>,
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
                    identifier: String::new(),
                    source: CellSource::Parameter(config.name.replace("${i}", &(type_index + config.index_offset).to_string()), type_index + config.index_offset),
                });
                let item_cells = group.per_item.iter().map(|item| MixerCell {
                    config: item.param.clone(),
                    function: item.function.clone(),
                    identifier: item.identifier.clone(),
                    source: match item.param.name.is_empty() {
                        true => CellSource::Fixed(if item.values.len() == 1 { item.values[0] } else { item.values.get(index as usize).copied().unwrap_or(0.0) }),
                        false => CellSource::Parameter(item.param.name.replace("${i}", &(index + item.param.index_offset).to_string()), index + item.param.index_offset),
                    },
                });
                let specific = (kind.is_some() && !prefix.is_empty()).then_some((function, prefix));
                let rule = metadata.rules.iter().rev().find(|rule| group.per_item.iter().any(|item| !item.identifier.is_empty() && item.identifier == rule.select)).cloned();
                (MixerChannel { label, function, type_index, cells: type_cells.chain(item_cells).collect(), rule }, specific)
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

pub struct AssignmentPlan {
    pub assign_motors: bool,
    pub message: String,
}

pub fn assignment_plan(groups: &[Vec<String>], labels: &[String], selected: usize, first: i64, count: i64, value_of: &dyn Fn(&str) -> Option<i64>) -> Result<AssignmentPlan, String> {
    let motor = |name: &String| value_of(name).filter(|f| (first..first + count).contains(f));
    let assigned_in = |wanted: &dyn Fn(usize) -> bool| -> std::collections::BTreeSet<i64> { groups.iter().enumerate().filter(|(i, _)| wanted(*i)).flat_map(|(_, g)| g.iter().filter_map(motor)).collect() };
    let on_selected = assigned_in(&|i| i == selected);
    let anywhere: std::collections::BTreeSet<i64> = on_selected.union(&assigned_in(&|i| i != selected)).copied().collect();
    let fits = groups.get(selected).is_some_and(|g| g.len() as i64 >= count);
    let output = labels.get(selected).cloned().unwrap_or_default();
    let extra = match (anywhere.len(), on_selected.len(), fits) {
        (0, _, true) => Some(format!("<br />No motors are assigned yet.\nBy saying yes, all motors will be assigned to the first {count} channels of the selected output ({output})\n (you can also first assign all motors, then start the identification).<br />")),
        (n, 0, true) if n > 0 => Some(format!("<br />Motors are currently assigned to a different output.\nBy saying yes, all motors will be reassigned to the first {count} channels of the selected output ({output}).<br />")),
        (n, _, _) if (n as i64) < count => return Err("Not all motors are assigned yet. Either clear all existing assignments or assign all motors to an output.".to_string()),
        _ => None,
    };
    let message = format!("This will automatically spin individual motors at 15% thrust.<br /><br />\n<b>Warning: Only proceed if you removed all propellers</b>.<br />\n{}\n<br />\nThe procedure is as following:<br />\n- After confirming, the first motor starts to spin for 0.5 seconds.<br />\n- Then click on the motor that was spinning.<br />\n- The above steps are repeated for all motors.<br />\n- The motor output functions will automatically be reassigned by the selected order.<br />\n<br />\nDo you wish to proceed?", extra.clone().unwrap_or_default());
    Ok(AssignmentPlan { assign_motors: extra.is_some(), message })
}

pub fn assignment_start_writes(groups: &[Vec<String>], selected: usize, first: i64, count: i64, value_of: &dyn Fn(&str) -> Option<i64>) -> Vec<(String, i64)> {
    let cleared = groups.iter().flatten().filter(|name| value_of(name).is_some_and(|f| (first..first + count).contains(&f))).map(|name| (name.clone(), 0));
    let assigned = groups.get(selected).into_iter().flatten().take(count.max(0) as usize).enumerate().map(|(i, name)| (name.clone(), first + i as i64));
    cleared.chain(assigned).collect()
}

pub fn assignment_finish_writes(groups: &[Vec<String>], first: i64, selected_motors: &[i64], value_of: &dyn Fn(&str) -> Option<i64>) -> Vec<(String, i64)> {
    let count = selected_motors.len() as i64;
    groups
        .iter()
        .flatten()
        .filter_map(|name| {
            let function = value_of(name).filter(|f| (first..first + count).contains(f))?;
            Some((name.clone(), first + selected_motors[(function - first) as usize]))
        })
        .collect()
}

pub fn function_params(output: &Output) -> Vec<String> {
    output
        .subgroups
        .iter()
        .flat_map(|subgroup| subgroup.channels.iter().flat_map(move |channel| subgroup.channel_configs.iter().filter(|c| c.function == "function").map(move |c| channel_param(c, channel))))
        .collect()
}

pub fn function_type(metadata: &Metadata, function: i64) -> Option<&str> {
    metadata.actuator_types.iter().rfind(|t| t.name != "DEFAULT" && (t.function_min..=t.function_max).contains(&function)).map(|t| t.name.as_str())
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionGroup {
    pub kind: i64,
    pub label: &'static str,
    pub actions: Vec<(String, i64)>,
}

pub fn action_groups(metadata: &Metadata, outputs: &[&Output], value_of: &dyn Fn(&str) -> Option<i64>) -> Vec<ActionGroup> {
    let candidates: Vec<(i64, &Subgroup)> = outputs
        .iter()
        .flat_map(|output| output.subgroups.iter().flat_map(move |subgroup| subgroup.channels.iter().flat_map(move |channel| subgroup.channel_configs.iter().filter(|c| c.function == "function").map(move |c| (channel_param(c, channel), subgroup)))))
        .filter_map(|(name, subgroup)| Some((value_of(&name)?, subgroup)))
        .filter(|(function, _)| *function != 0)
        .collect();
    let (groups, _) = candidates.into_iter().fold((Vec::<ActionGroup>::new(), std::collections::BTreeSet::new()), |(groups, added), (function, subgroup)| {
        let Some(known) = metadata.functions.get(&function).filter(|_| !added.contains(&function)) else { return (groups, added) };
        let kind = function_type(metadata, function);
        let usable: Vec<&SupportedAction> = subgroup
            .actions
            .iter()
            .filter(|action| action.condition.evaluate(value_of))
            .filter(|action| action.actuator_types.is_empty() || kind.is_some_and(|k| action.actuator_types.iter().any(|t| t == k)))
            .collect();
        let added = match usable.is_empty() {
            true => added,
            false => added.into_iter().chain(std::iter::once(function)).collect(),
        };
        let groups = usable.into_iter().fold(groups, |groups, action| match groups.iter().position(|g| g.kind == action.kind) {
            Some(at) => groups.into_iter().enumerate().map(|(i, g)| if i == at { ActionGroup { actions: g.actions.into_iter().chain(std::iter::once((known.label.clone(), function))).collect(), ..g } } else { g }).collect(),
            None => groups.into_iter().chain(std::iter::once(ActionGroup { kind: action.kind, label: action.label, actions: vec![(known.label.clone(), function)] })).collect(),
        });
        (groups, added)
    });
    groups
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

type FunctionEnumInfo = (std::collections::BTreeMap<i64, String>, std::collections::BTreeSet<i64>);

fn function_enum_info(metadata: &Metadata, mixer: &MixerState, label_of: &dyn Fn(i64) -> String) -> FunctionEnumInfo {
    let used: std::collections::BTreeMap<i64, String> = mixer_functions(mixer, false).into_iter().map(|f| (f, label_of(f))).collect();
    let removed = metadata.actuator_types.iter().filter(|t| t.name != "DEFAULT").flat_map(|t| t.function_min..=t.function_max).filter(|f| !used.contains_key(f)).collect();
    (used, removed)
}

pub fn output_function_fact(backend: &dyn Backend, path: &str, fact: Value) -> Value {
    if !path.starts_with("vehicle.parameterManager.getParameter(-1,") {
        return fact;
    }
    let metadata = crate::hub::lock().active().and_then(|v| v.actuators_metadata.clone()).and_then(|m| parse(&m).ok());
    let Some(metadata) = metadata else { return fact };
    let exists = |name: &str| self::fact(backend, name).is_some();
    let is_function = kept_outputs(&metadata, &exists).iter().any(|output| function_params(output).iter().any(|name| parameter_path(name) == path));
    if !is_function {
        return fact;
    }
    let value_of = |name: &str| integer(backend, name);
    let mixer = mixer_state(&metadata, &value_of);
    let enum_text = |name: &str| Some((integer(backend, name)?, self::fact(backend, name)?.get("valueString").and_then(Value::as_str).unwrap_or("").to_string()));
    let label_of = |function: i64| specific_label(&metadata, &mixer, function, &enum_text);
    let (used, removed) = function_enum_info(&metadata, &mixer, &label_of);
    function_fact(fact, &used, &removed)
}

pub fn rewrite_function_enum(strings: &[String], values: &[i64], used: &std::collections::BTreeMap<i64, String>, removed: &std::collections::BTreeSet<i64>) -> (Vec<String>, Vec<i64>) {
    let entries: Vec<(i64, String)> = values.iter().copied().zip(strings.iter().cloned()).collect();
    let with_used = used.iter().fold(entries, |entries, (function, label)| match entries.iter().position(|(v, _)| v == function) {
        Some(at) => entries.into_iter().enumerate().map(|(i, e)| if i == at { (e.0, label.clone()) } else { e }).collect(),
        None => {
            let at = entries.iter().position(|(v, _)| v > function).unwrap_or(entries.len());
            entries[..at].iter().cloned().chain(std::iter::once((*function, label.clone()))).chain(entries[at..].iter().cloned()).collect()
        }
    });
    with_used.into_iter().filter(|(v, _)| !removed.contains(v)).map(|(v, l)| (l, v)).unzip()
}

fn function_fact(read: Value, used: &std::collections::BTreeMap<i64, String>, removed: &std::collections::BTreeSet<i64>) -> Value {
    let strings: Vec<String> = read.get("enumStrings").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    if strings.is_empty() {
        return read;
    }
    let values: Vec<i64> = read.get("enumValues").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str()?.parse().ok())).collect()).unwrap_or_default();
    let (labels, numbers) = rewrite_function_enum(&strings, &values, used, removed);
    let current = read.get("rawValue").or_else(|| read.get("value")).and_then(Value::as_f64).map(|v| v as i64);
    let index = current.and_then(|c| numbers.iter().position(|n| *n == c));
    let mut rewritten = read;
    rewritten["enumStrings"] = json!(labels);
    rewritten["enumValues"] = json!(numbers);
    if let Some(index) = index {
        rewritten["enumIndex"] = json!(index);
        rewritten["valueString"] = json!(labels[index]);
        rewritten["enumStringValue"] = json!(labels[index]);
    }
    rewritten
}

fn function_control(backend: &dyn Backend, param: &Param, name: &str, bit: i64, used: &std::collections::BTreeMap<i64, String>, removed: &std::collections::BTreeSet<i64>) -> Option<Value> {
    if param.function != "function" {
        return control(backend, param, name, bit);
    }
    let read = function_fact(fact(backend, name)?, used, removed);
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
    let (used_labels, removed_functions) = function_enum_info(metadata, &mixer, &label_of);
    let held = crate::hub::lock().active().map(|v| (v.actuator_test.had_failure, v.motor_assignment.active(), v.motor_assignment.message.clone(), v.motor_assignment.highlighted()));
    let (had_failure, assigning, assignment_message, highlighted) = held.unwrap_or_default();
    let motors_drawn = !motor_geometry(metadata, &mixer, &|name| number_of(backend, name)).is_empty();
    let all_motors = testing.iter().rev().find(|a| a.is_motor).map(|motor| TestActuator { label: "All Motors".to_string(), ..motor.clone() });
    json!({
        "showUi": metadata.show_ui_if.evaluate(&value_of),
        "testing": {
            "actuators": testing.iter().map(actuator_json).collect::<Vec<_>>(),
            "allMotors": all_motors.as_ref().map(actuator_json),
            "hadFailure": had_failure,
        },
        "actions": action_groups(metadata, &kept_outputs(metadata, &exists), &value_of).iter().map(|group| json!({
            "label": group.label,
            "type": group.kind,
            "actions": group.actions.iter().map(|(label, function)| json!({ "label": label, "function": function })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "motorAssignment": {
            "multirotor": mixer.option.as_ref().is_some_and(|o| o.kind == "multirotor"),
            "enabled": motors_drawn,
            "active": assigning,
            "message": assignment_message,
            "highlighted": highlighted,
        },
        "hasUnsetRequiredFunctions": mixer_functions(&mixer, true).iter().any(|f| !configured.contains(f)),
        "geometry": geometry_json(backend, metadata, &mixer, &value_of),
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
                    "configs": subgroup.channel_configs.iter().map(|config| function_control(backend, config, &channel_param(config, channel), channel.param_index + config.index_offset, &used_labels, &removed_functions)).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn rule_item<'a>(channel: &'a MixerChannel, cell: &MixerCell, value_of: &dyn Fn(&str) -> Option<i64>) -> Option<&'a RuleItem> {
    let rule = channel.rule.as_ref()?;
    let apply = rule.apply.iter().position(|id| !cell.identifier.is_empty() && *id == cell.identifier)?;
    let selector = channel.cells.iter().find(|c| c.identifier == rule.select)?;
    let CellSource::Parameter(name, _) = &selector.source else { return None };
    rule.items.get(&value_of(name)?)?.get(apply)
}

fn axis_names(channel: &MixerChannel) -> Option<[String; 3]> {
    let named = |function: &str| channel.cells.iter().find(|c| c.function == function).and_then(|c| match &c.source {
        CellSource::Parameter(name, _) => Some(name.clone()),
        CellSource::Fixed(_) => None,
    });
    Some([named("axisx")?, named("axisy")?, named("axisz")?])
}

fn number_of(backend: &dyn Backend, name: &str) -> Option<f64> {
    let read = fact(backend, name)?;
    read.get("rawValue").or_else(|| read.get("value")).and_then(Value::as_f64)
}

fn channel_cells(backend: &dyn Backend, channel: &MixerChannel, value_of: &dyn Fn(&str) -> Option<i64>) -> Vec<Value> {
    let axes = axis_names(channel);
    let direction = axes.as_ref().and_then(|[x, y, z]| Some(axis_direction(number_of(backend, x)?, number_of(backend, y)?, number_of(backend, z)?)));
    channel
        .cells
        .iter()
        .flat_map(|cell| {
            let rule = rule_item(channel, cell, value_of);
            let axis_hidden = direction.is_some_and(|d| d != 0) && ["axisx", "axisy", "axisz"].contains(&cell.function.as_str());
            let mut shown = match &cell.source {
                CellSource::Parameter(name, index) => control(backend, &cell.config, name, *index).unwrap_or_else(|| json!({ "unavailable": true, "label": cell.config.label, "advanced": cell.config.advanced })),
                CellSource::Fixed(value) => json!({ "fixed": true, "label": cell.config.label, "valueString": format!("{value:.4}"), "advanced": cell.config.advanced }),
            };
            if let Value::Object(map) = &mut shown {
                map.insert("hidden".to_string(), json!(rule.is_some_and(|r| r.hidden) || axis_hidden));
                map.insert("disabled".to_string(), json!(rule.is_some_and(|r| r.disabled)));
                map.insert("channelFunction".to_string(), json!(channel.function));
                if let CellSource::Parameter(name, _) = &cell.source {
                    map.insert("param".to_string(), json!(name));
                }
            }
            let virtual_axis = (cell.function == "axisx").then(|| {
                axes.clone().zip(direction).map(|(names, index)| {
                    let first = rule;
                    json!({ "axis": true, "label": "Axis", "options": AXIS_DIRECTIONS, "index": index, "params": names, "hidden": first.is_some_and(|r| r.hidden), "disabled": first.is_some_and(|r| r.disabled), "advanced": cell.config.advanced })
                })
            }).flatten();
            virtual_axis.into_iter().chain(std::iter::once(shown))
        })
        .collect()
}

pub fn constrained(rule: &Rule, items_for: i64, current: f64, apply: usize, select_changed: bool) -> Option<f64> {
    let item = rule.items.get(&items_for)?.get(apply)?;
    let clamped = item.min.filter(|min| current < *min).or_else(|| item.max.filter(|max| current > *max));
    match (select_changed, item.default) {
        (true, Some(default)) => Some(default),
        _ => clamped,
    }
}

pub fn mixer_set(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let (Some(name), Some(value), Some(function)) = (given.get(0).and_then(Value::as_str), given.get(1).and_then(Value::as_f64), given.get(2).and_then(Value::as_i64)) else {
        return json!({ "ok": false, "reason": "actuatorMixer.set takes a parameter, its value and the actuator function." });
    };
    let path = parameter_path(name);
    let before = integer(backend, name);
    let written = crate::factwrite::write(backend, &path, &json!({ "value": value }).to_string());
    if !crate::read::flag(&written, "ok") {
        return written;
    }
    let metadata = crate::hub::lock().active().and_then(|v| v.actuators_metadata.clone()).and_then(|m| parse(&m).ok());
    let Some(metadata) = metadata else { return written };
    let value_of = |n: &str| integer(backend, n);
    let mixer = mixer_state(&metadata, &value_of);
    let Some(channel) = mixer.groups.iter().flat_map(|g| g.channels.iter()).find(|c| c.function == function) else { return written };
    let Some(rule) = channel.rule.as_ref() else { return written };
    let Some(selector) = channel.cells.iter().find(|c| c.identifier == rule.select) else { return written };
    let CellSource::Parameter(select_name, _) = &selector.source else { return written };
    let Some(selected) = value_of(select_name) else { return written };
    let select_changed = select_name == name && before != Some(selected);
    channel
        .cells
        .iter()
        .filter_map(|cell| Some((rule.apply.iter().position(|id| !cell.identifier.is_empty() && *id == cell.identifier)?, cell)))
        .filter_map(|(apply, cell)| match &cell.source {
            CellSource::Parameter(param, _) => Some((param.clone(), constrained(rule, selected, number_of(backend, param)?, apply, select_changed)?)),
            CellSource::Fixed(_) => None,
        })
        .for_each(|(param, value)| {
            crate::factwrite::write(backend, &parameter_path(&param), &json!({ "value": value }).to_string());
        });
    written
}

pub fn mixer_axis(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let names: Vec<&str> = given.get(0).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let direction = given.get(1).and_then(Value::as_u64).and_then(|d| usize::try_from(d).ok()).unwrap_or(0);
    let Some((x, y, z)) = axis_vector(direction).filter(|_| names.len() == 3) else {
        return json!({ "ok": direction == 0, "reason": if direction == 0 { Value::Null } else { json!("An axis direction needs its three axis parameters.") } });
    };
    let all = names.iter().zip([x, y, z]).all(|(name, value)| crate::read::flag(&crate::factwrite::write(backend, &parameter_path(name), &json!({ "value": value }).to_string()), "ok"));
    json!({ "ok": all })
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorGeometry {
    pub index: i64,
    pub label: i64,
    pub position: (f64, f64, f64),
    pub counter_clockwise: bool,
}

pub fn motor_geometry(metadata: &Metadata, mixer: &MixerState, raw_of: &dyn Fn(&str) -> Option<f64>) -> Vec<MotorGeometry> {
    mixer
        .groups
        .iter()
        .filter(|state| state.group.actuator_type == "motor")
        .flat_map(|state| {
            let offset = metadata.actuator_types.iter().find(|t| t.name == state.group.actuator_type).map_or(0, |t| t.label_index_offset);
            state.channels.iter().filter_map(move |channel| {
                let value = |function: &str| channel.cells.iter().find(|c| c.function == function).and_then(|c| match &c.source {
                    CellSource::Parameter(name, _) => raw_of(name).map(|raw| (raw, c.config.show_as.as_str())),
                    CellSource::Fixed(v) => Some((*v, "")),
                });
                let position = (value("posx")?.0, value("posy")?.0, value("posz")?.0);
                let counter_clockwise = value("spin-dir").is_some_and(|(raw, show_as)| if show_as == "true-if-positive" { raw > 0.0 } else { raw != 0.0 });
                Some(MotorGeometry { index: channel.type_index, label: channel.type_index + offset, position, counter_clockwise })
            })
        })
        .collect()
}

fn geometry_json(backend: &dyn Backend, metadata: &Metadata, mixer: &MixerState, value_of: &dyn Fn(&str) -> Option<i64>) -> Value {
    let Some(option) = mixer.option.as_ref() else { return Value::Null };
    json!({
        "title": if option.title.is_empty() { "Geometry".to_string() } else { format!("Geometry: {}", option.title) },
        "helpUrl": option.help_url,
        "type": option.kind,
        "motors": motor_geometry(metadata, mixer, &|name| number_of(backend, name))
            .iter()
            .map(|m| json!({ "index": m.index, "label": m.label, "x": m.position.0, "y": m.position.1, "z": m.position.2, "counterClockwise": m.counter_clockwise }))
            .collect::<Vec<_>>(),
        "groups": mixer.groups.iter().map(|state| json!({
            "label": state.group.label,
            "count": (!state.group.count_param.is_empty()).then(|| fact(backend, &state.group.count_param).map(|f| decode(&f, &parameter_path(&state.group.count_param)))).flatten(),
            "columns": state.channels.first().map(|c| c.cells.iter().map(|cell| json!({ "label": cell.config.label, "advanced": cell.config.advanced })).collect::<Vec<_>>()).unwrap_or_default(),
            "channels": state.channels.iter().map(|channel| json!({ "label": channel.label, "cells": channel_cells(backend, channel, value_of) })).collect::<Vec<_>>(),
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
    match path {
        MIXER_SET => return mixer_set(backend, args),
        MIXER_AXIS => return mixer_axis(backend, args),
        MOTOR_INIT | MOTOR_START | MOTOR_SELECT | MOTOR_SPIN | MOTOR_ABORT => {
            let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
            let action = match path {
                MOTOR_INIT => json!({ "action": "motorAssignment", "op": "init", "output": given.get(0) }),
                MOTOR_START => json!({ "action": "motorAssignment", "op": "start" }),
                MOTOR_SELECT => json!({ "action": "motorAssignment", "op": "select", "motor": given.get(0) }),
                MOTOR_SPIN => json!({ "action": "motorAssignment", "op": "spinAgain" }),
                _ => json!({ "action": "motorAssignment", "op": "abort" }),
            };
            return crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), path, args);
        }
        ACTION_TRIGGER => {
            let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
            let action = json!({ "action": "actuatorAction", "type": given.get(0), "function": given.get(1) });
            return crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), path, args);
        }
        _ => {}
    }
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let action = match path {
        TEST_ACTIVE => json!({ "action": "actuatorTest", "op": "active", "on": given.get(0).and_then(Value::as_bool).unwrap_or(false) }),
        TEST_SET => json!({ "action": "actuatorTest", "op": "set", "function": given.get(0), "value": given.get(1) }),
        _ => json!({ "action": "actuatorTest", "op": "stop", "function": given.get(0).filter(|f| f.as_i64() != Some(-1)) }),
    };
    crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), path, args)
}

pub fn owns(path: &str) -> bool {
    [TEST_ACTIVE, TEST_SET, TEST_STOP, MIXER_SET, MIXER_AXIS, ACTION_TRIGGER, MOTOR_INIT, MOTOR_START, MOTOR_SELECT, MOTOR_SPIN, MOTOR_ABORT].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruled() -> Metadata {
        parse(&json!({
            "outputs_v1": [],
            "functions_v1": { "201": { "label": "Servo 1" } },
            "mixer_v1": {
                "actuator-types": { "servo": { "function-min": 201, "function-max": 208, "values": { "min": -1, "max": 1 } } },
                "config": [{ "option": "", "actuators": [{ "group-label": "Control Surfaces", "count": 1, "actuator-type": "servo", "per-item-parameters": [
                    { "label": "Type", "name": "CA_SV_CS${i}_TYPE", "identifier": "servo-type", "function": "type" },
                    { "label": "Roll Torque", "name": "CA_SV_CS${i}_TRQ_R", "identifier": "servo-torque-roll" },
                    { "label": "Pitch Torque", "name": "CA_SV_CS${i}_TRQ_P", "identifier": "servo-torque-pitch" }
                ] }] }],
                "rules": [{ "select-identifier": "servo-type", "apply-identifiers": ["servo-torque-roll", "servo-torque-pitch"], "items": {
                    "1": [{ "min": -1, "max": 0, "default": -0.5 }, { "hidden": true, "default": 0 }],
                    "2": [{ "min": 0 }]
                } }]
            }
        }))
        .unwrap()
    }

    #[test]
    fn a_cell_whose_parameter_is_missing_stays_as_not_available_like_a_null_fact_channel_config_instance() {
        struct Missing;
        impl Backend for Missing {
            fn get(&self, _p: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.get("") }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let metadata = ruled();
        let state = mixer_state(&metadata, &|_| None);
        let cells = channel_cells(&Missing, &state.groups[0].channels[0], &|_| None);
        assert_eq!(cells.len(), 3, "ActuatorComponent.qml keeps the grid cell and its ActuatorFact shows '(Param not available)'");
        assert_eq!((cells[1]["unavailable"].clone(), cells[1]["label"].clone(), cells[1]["hidden"].clone()), (json!(true), json!("Roll Torque"), json!(false)));
    }

    #[test]
    fn a_rule_follows_the_selector_and_clamps_or_resets_what_it_applies_to() {
        let metadata = ruled();
        assert_eq!(metadata.rules[0].items.len(), 1, "an item list of the wrong length is dropped, as QGC drops it");
        let state = mixer_state(&metadata, &|_| None);
        let channel = &state.groups[0].channels[0];
        assert_eq!(channel.rule.as_ref().map(|r| r.select.as_str()), Some("servo-type"));
        let rule = channel.rule.as_ref().unwrap();
        assert_eq!(constrained(rule, 1, 0.4, 0, false), Some(0.0), "above max is clamped");
        assert_eq!(constrained(rule, 1, -0.2, 0, false), None, "inside the range is left alone");
        assert_eq!(constrained(rule, 1, -0.2, 0, true), Some(-0.5), "a new selection applies the default");
        assert_eq!(constrained(rule, 7, 0.4, 0, true), None, "a selection with no items changes nothing");
        let values = |name: &str| (name == "CA_SV_CS0_TYPE").then_some(1);
        let pitch = &channel.cells[2];
        assert!(rule_item(channel, pitch, &values).is_some_and(|item| item.hidden));
    }

    #[test]
    fn actions_group_by_type_and_list_each_configured_function_once() {
        let parsed = parse(&example()).unwrap();
        let aux = &parsed.outputs[1].subgroups[0];
        assert_eq!(aux.actions.iter().map(|a| (a.kind, a.label)).collect::<Vec<_>>(), [(4, "Set Spin Direction 1"), (5, "Set Spin Direction 2")]);
        let outputs: Vec<&Output> = parsed.outputs.iter().collect();
        let dshot = |name: &str| match name {
            "PWM_AUX_TIM0" => Some(-5),
            "PWM_AUX_FUNC1" => Some(101),
            "PWM_AUX_FUNC2" => Some(201),
            "PWM_AUX_FUNC3" => Some(101),
            _ => Some(0),
        };
        let groups = action_groups(&parsed, &outputs, &dshot);
        assert_eq!(groups.iter().map(|g| (g.label, g.actions.clone())).collect::<Vec<_>>(), [("Set Spin Direction 1", vec![("Motor 1".to_string(), 101)]), ("Set Spin Direction 2", vec![("Motor 1".to_string(), 101)])], "the servo is not a motor and Motor 1 is listed once");
        let pwm = |name: &str| if name == "PWM_AUX_TIM0" { Some(400) } else { dshot(name) };
        assert!(action_groups(&parsed, &outputs, &pwm).is_empty(), "supported-if holds only on DShot");
    }

    #[test]
    fn motors_with_all_three_positions_are_drawn_numbered_from_the_label_offset() {
        let parsed = parse(&example()).unwrap();
        let quad = |name: &str| match name {
            "CA_AIRFRAME" => Some(1),
            "CA_MC_R_COUNT" => Some(2),
            _ => None,
        };
        let state = mixer_state(&parsed, &quad);
        let raw = |name: &str| match name {
            "CA_MC_R0_PX" => Some(0.15),
            "CA_MC_R0_PY" => Some(0.25),
            "CA_MC_R0_PZ" => Some(0.0),
            "CA_MC_R0_KM" => Some(0.05),
            "CA_MC_R1_PX" => Some(-0.15),
            "CA_MC_R1_PY" => Some(-0.25),
            "CA_MC_R1_PZ" => Some(0.0),
            "CA_MC_R1_KM" => Some(-0.05),
            _ => None,
        };
        let motors = motor_geometry(&parsed, &state, &raw);
        assert_eq!(motors, [
            MotorGeometry { index: 0, label: 1, position: (0.15, 0.25, 0.0), counter_clockwise: true },
            MotorGeometry { index: 1, label: 2, position: (-0.15, -0.25, 0.0), counter_clockwise: false },
        ]);
        let tilt = |name: &str| (name == "CA_AIRFRAME").then_some(5);
        assert!(motor_geometry(&parsed, &mixer_state(&parsed, &tilt), &|_| None).is_empty(), "fixed X and Y but no Z is not a full position, so nothing is drawn");
    }

    #[test]
    fn the_function_dropdown_names_used_motors_specifically_and_drops_unused_ones() {
        let strings: Vec<String> = ["Disabled", "Motor 1", "Motor 2", "Motor 3", "Servo 1"].iter().map(|s| s.to_string()).collect();
        let values = [0, 101, 102, 103, 201];
        let used: std::collections::BTreeMap<i64, String> = [(101, "Front Left Motor".to_string()), (102, "Front Right Motor".to_string()), (202, "Right Elevon".to_string())].into_iter().collect();
        let removed: std::collections::BTreeSet<i64> = [103, 104, 201].into_iter().collect();
        let (labels, numbers) = rewrite_function_enum(&strings, &values, &used, &removed);
        assert_eq!(numbers, [0, 101, 102, 202]);
        assert_eq!(labels, ["Disabled", "Front Left Motor", "Front Right Motor", "Right Elevon"], "a used function missing from the list is inserted in order");
        let read = function_fact(json!({ "kind": "fact", "rawValue": 102, "value": 102, "enumStrings": strings, "enumValues": values, "enumIndex": 2, "valueString": "Motor 2" }), &used, &removed);
        assert_eq!((read["enumIndex"].clone(), read["valueString"].clone()), (json!(2), json!("Front Right Motor")));
    }

    #[test]
    fn the_virtual_axis_names_the_six_unit_directions() {
        assert_eq!(AXIS_DIRECTIONS[axis_direction(0.0, 0.0, -1.0)], "Upwards");
        assert_eq!(AXIS_DIRECTIONS[axis_direction(0.0, 1.0, 0.0)], "Rightwards");
        assert_eq!(axis_direction(0.3, 0.0, -1.0), 0, "anything else is Custom");
        assert_eq!(axis_vector(3), Some((1.0, 0.0, 0.0)));
        assert_eq!(axis_vector(0), None, "Custom writes nothing");
    }

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
