use std::collections::{BTreeMap, BTreeSet};

use roxmltree::{Document, Node};
use serde_json::{Value, json};

use crate::compression::inflate_xz;
use crate::factmeta::{EnumEntry, MetaData, ValueType, value_type};
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
pub const WRITE_TIMEOUT_RETRIES: u32 = 3;
pub const FAILED_ACK_RETRIES: u32 = 3;
pub const REQUEST_RETRIES: u32 = 3;
pub const PARAM_WRITE_TIMEOUT_MS: u64 = 3000;
pub const PARAM_REQUEST_TIMEOUT_MS: u64 = 3500;
pub const PARAM_UPDATE_DELAY_MS: u64 = 500;
pub const PARAM_VALUE_BYTES: usize = 128;
pub const READING_STALE_MS: u64 = 10_000;
pub const PARAM_EXT_TYPE_UINT8: u8 = 1;
pub const PARAM_EXT_TYPE_INT8: u8 = 2;
pub const PARAM_EXT_TYPE_UINT16: u8 = 3;
pub const PARAM_EXT_TYPE_INT16: u8 = 4;
pub const PARAM_EXT_TYPE_UINT32: u8 = 5;
pub const PARAM_EXT_TYPE_INT32: u8 = 6;
pub const PARAM_EXT_TYPE_UINT64: u8 = 7;
pub const PARAM_EXT_TYPE_INT64: u8 = 8;
pub const PARAM_EXT_TYPE_REAL32: u8 = 9;
pub const PARAM_EXT_TYPE_REAL64: u8 = 10;
pub const PARAM_EXT_TYPE_CUSTOM: u8 = 11;
const XZ_MAGIC: [u8; 6] = [0xfd, b'7', b'z', b'X', b'Z', 0x00];
const NEUTRAL_LOCALE: &str = "en_us";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Unreadable,
    NotXml,
    MissingConstants,
    NoParameters,
    BadParameter,
}

impl ErrorKind {
    pub fn token(self) -> &'static str {
        match self {
            ErrorKind::Unreadable => "unreadable",
            ErrorKind::NotXml => "notXml",
            ErrorKind::MissingConstants => "missingConstants",
            ErrorKind::NoParameters => "noParameters",
            ErrorKind::BadParameter => "badParameter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub kind: ErrorKind,
    pub detail: String,
}

fn refusal(kind: ErrorKind, detail: impl Into<String>) -> Refusal {
    Refusal { kind, detail: detail.into() }
}

fn bad(detail: impl Into<String>) -> Refusal {
    refusal(ErrorKind::BadParameter, detail)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub meta: MetaData,
    pub write_only: bool,
}

impl Parameter {
    pub fn usable(&self) -> bool {
        !(self.meta.read_only && self.write_only)
    }

    pub fn readable(&self) -> bool {
        self.meta.has_control && !self.write_only
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exclusion {
    pub parameter: String,
    pub value: String,
    pub excludes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionRange {
    pub parameter: String,
    pub value: String,
    pub target: String,
    pub condition: String,
    pub options: Vec<EnumEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cause {
    pub parameter: String,
    pub value: String,
    pub condition: String,
}

impl Cause {
    fn of_exclusion(exclusion: &Exclusion) -> Cause {
        Cause { parameter: exclusion.parameter.clone(), value: exclusion.value.clone(), condition: String::new() }
    }

    fn of_range(range: &OptionRange) -> Cause {
        Cause { parameter: range.parameter.clone(), value: range.value.clone(), condition: range.condition.clone() }
    }

    fn json(&self) -> Value {
        json!({ "parameter": self.parameter, "value": self.value, "condition": self.condition })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub version: i64,
    pub model: String,
    pub vendor: String,
    pub settings: Vec<String>,
    pub parameters: Vec<Parameter>,
    pub exclusions: Vec<Exclusion>,
    pub ranges: Vec<OptionRange>,
    pub updates: BTreeMap<String, Vec<String>>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Default,
    Requested,
    Failed,
    Camera,
}

impl Source {
    pub fn token(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::Requested => "requested",
            Source::Failed => "failed",
            Source::Camera => "camera",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub value: Value,
    pub source: Source,
    pub stamped_ms: u64,
}

impl Reading {
    pub fn age_ms(&self, now_ms: u64) -> Option<u64> {
        (self.source != Source::Default).then(|| now_ms.saturating_sub(self.stamped_ms))
    }

    pub fn stale(&self, now_ms: u64) -> bool {
        self.source == Source::Camera && now_ms.saturating_sub(self.stamped_ms) > READING_STALE_MS
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub entries: Vec<EnumEntry>,
    pub resolved: bool,
    pub limited_by: Option<Cause>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Active {
    pub settings: Vec<String>,
    pub unresolved: Vec<String>,
    pub excluded_by: BTreeMap<String, Cause>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    NoControl,
    Applicable,
    Excluded,
    Unresolved,
}

impl Applicability {
    pub fn token(self) -> &'static str {
        match self {
            Applicability::NoControl => "noControl",
            Applicability::Applicable => "applicable",
            Applicability::Excluded => "excluded",
            Applicability::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    ActiveSettings { settings: Vec<String>, unresolved: Vec<String> },
    Options { parameter: String, options: Options },
    ScheduleUpdates { after_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Test {
    Equal,
    NotEqual,
    Greater,
    Smaller,
}

fn text(node: Node) -> String {
    node.descendants().filter_map(|n| n.is_text().then(|| n.text()).flatten()).collect::<String>().trim().to_string()
}

fn flag(node: Node, name: &str) -> Option<bool> {
    node.attribute(name).map(|value| value != "0")
}

fn child<'a>(node: Node<'a, 'a>, tag: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|c| c.has_tag_name(tag))
}

fn descendant<'a>(node: Node<'a, 'a>, tag: &str) -> Option<Node<'a, 'a>> {
    node.descendants().find(|c| c.has_tag_name(tag))
}

fn descendants<'a>(node: Node<'a, 'a>, tag: &str) -> Vec<Node<'a, 'a>> {
    node.descendants().filter(|c| c.has_tag_name(tag)).collect()
}

fn nested(node: Node, outer: &str, inner: &str) -> Vec<String> {
    descendant(node, outer).map(|list| descendants(list, inner).into_iter().map(text).filter(|t| !t.is_empty()).collect()).unwrap_or_default()
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_bool().map(|flag| flag as i64 as f64))
}

fn in_range(value_type: ValueType, candidate: f64) -> bool {
    match value_type {
        ValueType::Uint8 => (0.0..=255.0).contains(&candidate),
        ValueType::Int8 => (-128.0..=127.0).contains(&candidate),
        ValueType::Uint16 => (0.0..=65535.0).contains(&candidate),
        ValueType::Int16 => (-32768.0..=32767.0).contains(&candidate),
        ValueType::Uint32 => (0.0..=4_294_967_295.0).contains(&candidate),
        ValueType::Int32 => (-2_147_483_648.0..=2_147_483_647.0).contains(&candidate),
        ValueType::Uint64 => (0.0..=i64::MAX as f64).contains(&candidate),
        _ => (i64::MIN as f64..=i64::MAX as f64).contains(&candidate),
    }
}

pub fn typed(value_type: ValueType, raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    match value_type {
        ValueType::String | ValueType::Custom => Some(Value::String(raw.to_string())),
        ValueType::Bool => Some(Value::Bool(trimmed != "0" && !trimmed.eq_ignore_ascii_case("false"))),
        ValueType::Float | ValueType::Double => trimmed.parse::<f64>().ok().filter(|n| n.is_finite()).map(Value::from),
        _ => trimmed.parse::<f64>().ok().map(f64::round).filter(|n| in_range(value_type, *n)).map(|n| Value::from(n as i64)),
    }
}

pub fn coerce(value_type: ValueType, value: Value) -> Value {
    match (value_type, &value) {
        (ValueType::Bool, Value::Number(_)) => Value::Bool(number(&value).is_some_and(|n| n != 0.0)),
        (ValueType::Bool, Value::String(raw)) => Value::Bool(raw.trim() != "0" && !raw.trim().eq_ignore_ascii_case("false")),
        _ => value,
    }
}

pub fn same_typed(value_type: ValueType, current: &Value, wanted: &Value) -> bool {
    match (value_type, number(current), number(wanted)) {
        (ValueType::Float, Some(a), Some(b)) => a as f32 == b as f32,
        (_, Some(a), Some(b)) => a == b,
        _ => current == wanted,
    }
}

pub fn same_value(value_type: ValueType, current: &Value, literal: &str) -> bool {
    typed(value_type, literal).is_some_and(|wanted| same_typed(value_type, current, &wanted))
}

fn ordered(value_type: ValueType, current: &Value, literal: &str) -> Option<std::cmp::Ordering> {
    match (number(current), typed(value_type, literal).as_ref().and_then(number)) {
        (Some(a), Some(b)) => a.partial_cmp(&b),
        _ => Some(value_string(current).cmp(&literal.to_string())),
    }
}

fn value_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn both(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

pub fn mav_param_ext_type(value_type: ValueType) -> u8 {
    match value_type {
        ValueType::Uint8 | ValueType::Bool => PARAM_EXT_TYPE_UINT8,
        ValueType::Int8 => PARAM_EXT_TYPE_INT8,
        ValueType::Uint16 => PARAM_EXT_TYPE_UINT16,
        ValueType::Int16 => PARAM_EXT_TYPE_INT16,
        ValueType::Uint32 => PARAM_EXT_TYPE_UINT32,
        ValueType::Uint64 => PARAM_EXT_TYPE_UINT64,
        ValueType::Int64 => PARAM_EXT_TYPE_INT64,
        ValueType::Float => PARAM_EXT_TYPE_REAL32,
        ValueType::Double => PARAM_EXT_TYPE_REAL64,
        ValueType::String | ValueType::Custom => PARAM_EXT_TYPE_CUSTOM,
        _ => PARAM_EXT_TYPE_INT32,
    }
}

pub fn decode_param_value(param_type: u8, bytes: &[u8]) -> Option<Value> {
    let eight = |n: usize| -> Option<[u8; 8]> { bytes.get(..n).map(|slice| std::array::from_fn(|i| slice.get(i).copied().unwrap_or(0))) };
    match param_type {
        PARAM_EXT_TYPE_UINT8 => bytes.first().map(|b| Value::from(*b)),
        PARAM_EXT_TYPE_INT8 => bytes.first().map(|b| Value::from(*b as i8)),
        PARAM_EXT_TYPE_UINT16 => eight(2).map(|b| Value::from(u16::from_le_bytes([b[0], b[1]]))),
        PARAM_EXT_TYPE_INT16 => eight(2).map(|b| Value::from(i16::from_le_bytes([b[0], b[1]]))),
        PARAM_EXT_TYPE_UINT32 => eight(4).map(|b| Value::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
        PARAM_EXT_TYPE_INT32 => eight(4).map(|b| Value::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
        PARAM_EXT_TYPE_UINT64 => eight(8).map(|b| Value::from(u64::from_le_bytes(b))),
        PARAM_EXT_TYPE_INT64 => eight(8).map(|b| Value::from(i64::from_le_bytes(b))),
        PARAM_EXT_TYPE_REAL32 => eight(4).map(|b| Value::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64)),
        PARAM_EXT_TYPE_REAL64 => eight(8).map(|b| Value::from(f64::from_le_bytes(b))),
        PARAM_EXT_TYPE_CUSTOM => Some(Value::String(
            String::from_utf8_lossy(bytes.get(..PARAM_VALUE_BYTES).unwrap_or(bytes).split(|b| *b == 0).next().unwrap_or(&[])).to_string(),
        )),
        _ => None,
    }
}

fn locale_language(name: &str) -> String {
    let normalized = name.trim().to_ascii_lowercase().replace('-', "_");
    normalized.split('_').next().unwrap_or("").to_string()
}

pub fn localize(xml: &str, locale: &str) -> String {
    let normalized = locale.trim().to_ascii_lowercase().replace('-', "_");
    if normalized == NEUTRAL_LOCALE || normalized.is_empty() {
        return xml.to_string();
    }
    let Ok(document) = Document::parse(xml) else { return xml.to_string() };
    let Some(root) = descendant(document.root(), "localization") else { return xml.to_string() };
    let locales = descendants(root, "locale");
    let named = |node: &Node| node.attribute("name").map(|n| n.trim().to_ascii_lowercase().replace('-', "_")).unwrap_or_default();
    let exact = locales.iter().find(|node| named(node) == normalized);
    let language = locale_language(&normalized);
    let by_language = || locales.iter().find(|node| locale_language(&named(node)) == language);
    let Some(chosen) = exact.or_else(by_language) else { return xml.to_string() };
    descendants(*chosen, "strings").iter().filter_map(|node| Some((node.attribute("original")?, node.attribute("translated")?))).fold(xml.to_string(), |text, (original, translated)| {
        text.replace(&format!("\"{original}\""), &format!("\"{translated}\"")).replace(&format!(">{original}<"), &format!(">{translated}<"))
    })
}

struct RawRange {
    parameter: String,
    value: String,
    target: String,
    condition: String,
    options: Vec<(String, String)>,
}

fn option_ranges(option: Node, parameter: &str, value: &str) -> Result<Vec<RawRange>, Refusal> {
    let Some(root) = descendant(option, "parameterranges") else { return Ok(Vec::new()) };
    descendants(root, "parameterrange")
        .into_iter()
        .map(|range| {
            let target = range.attribute("parameter").ok_or_else(|| bad(format!("Malformed option range for parameter {parameter}")))?;
            let options: Result<Vec<(String, String)>, Refusal> = descendants(range, "roption")
                .into_iter()
                .map(|roption| {
                    let name = roption.attribute("name").ok_or_else(|| bad(format!("Malformed roption for parameter {parameter}")))?;
                    let raw = roption.attribute("value").ok_or_else(|| bad(format!("Malformed rvalue for parameter {parameter}")))?;
                    Ok((name.to_string(), raw.to_string()))
                })
                .collect();
            Ok(RawRange {
                parameter: parameter.to_string(),
                value: value.to_string(),
                target: target.to_string(),
                condition: range.attribute("condition").unwrap_or("").to_string(),
                options: options?,
            })
        })
        .collect::<Result<Vec<RawRange>, Refusal>>()
        .map(|ranges| ranges.into_iter().filter(|range| !range.options.is_empty()).collect())
}

struct Choice {
    entry: EnumEntry,
    exclusions: Vec<Exclusion>,
    ranges: Vec<RawRange>,
}

struct Parsed {
    parameter: Parameter,
    exclusions: Vec<Exclusion>,
    ranges: Vec<RawRange>,
    updates: Vec<String>,
    diagnostics: Vec<String>,
}

fn choices(root: Node, parameter: &str, value_type: ValueType) -> Result<Vec<Choice>, Refusal> {
    descendants(root, "option")
        .into_iter()
        .map(|option| {
            let label = option.attribute("name").ok_or_else(|| bad(format!("Malformed option for parameter {parameter}")))?;
            let raw = option.attribute("value").ok_or_else(|| bad(format!("Malformed value for parameter {parameter}")))?;
            let value = typed(value_type, raw).ok_or_else(|| bad(format!("Option {label} of parameter {parameter} declares value \"{raw}\", which its type cannot hold")))?;
            let excludes = nested(option, "exclusions", "exclude");
            let exclusions = (!excludes.is_empty()).then(|| Exclusion { parameter: parameter.to_string(), value: raw.to_string(), excludes }).into_iter().collect();
            Ok(Choice { entry: EnumEntry { label: label.to_string(), value }, exclusions, ranges: option_ranges(option, parameter, raw)? })
        })
        .collect()
}

fn parameter(node: Node) -> Result<Parsed, Refusal> {
    let name = node.attribute("name").ok_or_else(|| bad("Parameter entry missing parameter name"))?.to_string();
    let type_name = node.attribute("type").ok_or_else(|| bad(format!("Parameter {name} missing parameter type")))?;
    let value_type = value_type(type_name).ok_or_else(|| bad(format!("Unknown type for parameter {name}")))?;
    let description = child(node, "description").map(text).ok_or_else(|| bad(format!("Parameter {name} missing parameter description")))?;
    let write_only = flag(node, "writeonly").unwrap_or(false);
    let read_only = flag(node, "readonly").unwrap_or(false);
    let has_control = flag(node, "control").unwrap_or(true) && value_type != ValueType::Custom;
    let options = descendant(node, "options").map(|root| choices(root, &name, value_type)).unwrap_or_else(|| Ok(Vec::new()))?;
    let limits: Vec<(&str, &str, Option<Value>)> = ["default", "min", "max"].into_iter().filter_map(|key| node.attribute(key).map(|raw| (key, raw, typed(value_type, raw)))).collect();
    let limit = |wanted: &str| limits.iter().find(|(key, _, _)| *key == wanted).and_then(|(_, _, value)| value.clone());
    let rejected = limits
        .iter()
        .filter(|(_, _, value)| value.is_none())
        .map(|(key, raw, _)| format!("Parameter {name} declares {key}=\"{raw}\", which a {type_name} cannot hold, so the attribute is dropped"));
    let contradiction = (read_only && write_only).then(|| format!("Parameter {name} cannot be both read only and write only"));
    Ok(Parsed {
        parameter: Parameter {
            meta: MetaData {
                name: name.clone(),
                value_type,
                short_description: description.clone(),
                long_description: description,
                units: node.attribute("unit").map(str::to_string),
                decimal_places: node.attribute("decimalPlaces").and_then(|raw| raw.trim().parse().ok()),
                default: limit("default"),
                min: limit("min"),
                max: limit("max"),
                increment: node.attribute("step").and_then(|raw| raw.trim().parse().ok()),
                enums: options.iter().map(|choice| choice.entry.clone()).collect(),
                bitmask: false,
                has_control,
                qgc_reboot_required: false,
                vehicle_reboot_required: false,
                volatile_value: false,
                read_only,
                group: None,
                category: None,
            },
            write_only,
        },
        exclusions: options.iter().flat_map(|choice| choice.exclusions.clone()).collect(),
        updates: nested(node, "updates", "update"),
        diagnostics: rejected.chain(contradiction).collect(),
        ranges: options.into_iter().flat_map(|choice| choice.ranges).collect(),
    })
}

fn typed_range(range: &RawRange, target_type: Option<ValueType>) -> (Option<OptionRange>, Vec<String>) {
    let Some(value_type) = target_type else {
        return (None, vec![format!("Parameter {} limits {}, which the definition never declares, so the rule is dropped", range.parameter, range.target)]);
    };
    let judged: Vec<(String, Option<Value>)> = range.options.iter().map(|(label, raw)| (label.clone(), typed(value_type, raw))).collect();
    let options: Vec<EnumEntry> = judged.iter().filter_map(|(label, value)| value.clone().map(|value| EnumEntry { label: label.clone(), value })).collect();
    let notes: Vec<String> = range
        .options
        .iter()
        .zip(judged.iter())
        .filter(|(_, (_, value))| value.is_none())
        .map(|((_, raw), _)| format!("Parameter {} limits {} to \"{}\", which its type cannot hold, so the option is dropped", range.parameter, range.target, raw))
        .collect();
    match options.is_empty() {
        true => (None, notes),
        false => (
            Some(OptionRange {
                parameter: range.parameter.clone(),
                value: range.value.clone(),
                target: range.target.clone(),
                condition: range.condition.clone(),
                options,
            }),
            notes,
        ),
    }
}

pub fn parse(xml: &str) -> Result<Definition, Refusal> {
    let document = Document::parse(xml).map_err(|e| refusal(ErrorKind::NotXml, format!("Unable to parse camera definition file: {e}")))?;
    let root = document.root();
    let constants = descendant(root, "definition").ok_or_else(|| refusal(ErrorKind::MissingConstants, "Unable to load camera constants from camera definition"))?;
    let model = child(constants, "model").map(text).ok_or_else(|| refusal(ErrorKind::MissingConstants, "Camera definition has no model"))?;
    let vendor = child(constants, "vendor").map(text).ok_or_else(|| refusal(ErrorKind::MissingConstants, "Camera definition has no vendor"))?;
    let version = constants
        .attribute("version")
        .map(|raw| raw.trim().parse::<i64>().unwrap_or(0))
        .ok_or_else(|| refusal(ErrorKind::MissingConstants, "Camera definition has no version"))?;
    let nodes = descendants(descendant(root, "parameters").ok_or_else(|| refusal(ErrorKind::NoParameters, "No parameters to load from camera"))?, "parameter");
    let parsed: Vec<Parsed> = nodes.into_iter().map(parameter).collect::<Result<_, Refusal>>()?;
    let mut seen = BTreeSet::new();
    let unique: Vec<Parsed> = parsed.into_iter().filter(|entry| seen.insert(entry.parameter.meta.name.clone())).collect();
    let parameters: Vec<Parameter> = unique.iter().map(|entry| entry.parameter.clone()).collect();
    if parameters.is_empty() {
        return Err(refusal(ErrorKind::NoParameters, "Unable to load camera parameters from camera definition"));
    }
    let target_type = |target: &str| parameters.iter().find(|p| p.meta.name == target).map(|p| p.meta.value_type);
    let judged: Vec<(Option<OptionRange>, Vec<String>)> = unique.iter().flat_map(|entry| &entry.ranges).map(|range| typed_range(range, target_type(&range.target))).collect();
    Ok(Definition {
        version,
        model,
        vendor,
        settings: parameters.iter().filter(|p| p.meta.has_control).map(|p| p.meta.name.clone()).collect(),
        exclusions: unique.iter().flat_map(|entry| entry.exclusions.clone()).collect(),
        ranges: judged.iter().filter_map(|(range, _)| range.clone()).collect(),
        updates: unique.iter().filter(|entry| !entry.updates.is_empty()).map(|entry| (entry.parameter.meta.name.clone(), entry.updates.clone())).collect(),
        diagnostics: unique.iter().flat_map(|entry| entry.diagnostics.clone()).chain(judged.iter().flat_map(|(_, notes)| notes.clone())).collect(),
        parameters,
    })
}

pub fn from_bytes(bytes: &[u8], locale: &str) -> Result<Definition, Refusal> {
    let plain = match bytes.starts_with(&XZ_MAGIC) {
        true => inflate_xz(bytes).map_err(|e| refusal(ErrorKind::NotXml, e))?,
        false => bytes.to_vec(),
    };
    let xml = String::from_utf8(plain).map_err(|e| refusal(ErrorKind::NotXml, format!("camera definition is not UTF-8 text: {e}")))?;
    parse(&localize(&xml, locale))
}

impl Definition {
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.meta.name == name)
    }
}

fn defaults(definition: &Definition) -> BTreeMap<String, Reading> {
    definition
        .parameters
        .iter()
        .filter_map(|p| p.meta.default.clone().map(|value| (p.meta.name.clone(), Reading { value, source: Source::Default, stamped_ms: 0 })))
        .collect()
}

#[derive(Debug, Clone)]
pub struct CameraParameters {
    pub definition: Definition,
    readings: BTreeMap<String, Reading>,
    options: BTreeMap<String, Options>,
    active: Active,
    pending_updates: BTreeSet<String>,
}

impl CameraParameters {
    pub fn new(definition: Definition) -> Self {
        let settled = CameraParameters {
            readings: defaults(&definition),
            options: BTreeMap::new(),
            active: Active::default(),
            pending_updates: BTreeSet::new(),
            definition,
        };
        CameraParameters { active: settled.resolve_active(), options: settled.resolve_options(), ..settled }
    }

    pub fn is_basic(&self) -> bool {
        self.definition.parameters.is_empty()
    }

    pub fn has_controls(&self) -> bool {
        !self.definition.settings.is_empty()
    }

    pub fn all_confirmed(&self) -> bool {
        self.definition
            .parameters
            .iter()
            .filter(|p| p.readable())
            .all(|p| self.readings.get(&p.meta.name).is_some_and(|reading| reading.source == Source::Camera))
    }

    pub fn reading(&self, name: &str) -> Option<&Reading> {
        self.readings.get(name)
    }

    pub fn options(&self, name: &str) -> Option<&Options> {
        self.options.get(name)
    }

    pub fn active_settings(&self) -> &[String] {
        &self.active.settings
    }

    pub fn unresolved_settings(&self) -> &[String] {
        &self.active.unresolved
    }

    pub fn excluded_by(&self, name: &str) -> Option<&Cause> {
        self.active.excluded_by.get(name)
    }

    pub fn applicability(&self, name: &str) -> Applicability {
        match (self.definition.settings.iter().any(|setting| setting == name), self.active.excluded_by.contains_key(name), self.active.unresolved.iter().any(|setting| setting == name)) {
            (false, ..) => Applicability::NoControl,
            (_, true, _) => Applicability::Excluded,
            (_, _, true) => Applicability::Unresolved,
            _ => Applicability::Applicable,
        }
    }

    pub fn on_value(&mut self, name: &str, value: Value, source: Source, now_ms: u64) -> Vec<Out> {
        let Some(value_type) = self.definition.parameter(name).map(|p| p.meta.value_type) else { return Vec::new() };
        self.readings.insert(name.to_string(), Reading { value: coerce(value_type, value), source, stamped_ms: now_ms });
        self.pending_updates.extend(self.definition.updates.get(name).cloned().unwrap_or_default());
        self.settle()
    }

    pub fn take_pending_updates(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_updates).into_iter().collect()
    }

    pub fn condition(&self, condition: &str) -> Option<bool> {
        let tokens: Vec<&str> = condition.split(' ').filter(|token| !token.is_empty()).collect();
        tokens
            .chunks(2)
            .try_fold((true, true), |(result, and_op), chunk| {
                let test = self.condition_test(chunk[0])?;
                let next = chunk.get(1).map(|op| op.eq_ignore_ascii_case("and")).unwrap_or(true);
                Some((if and_op { result && test } else { result || test }, next))
            })
            .map(|(result, _)| result)
    }

    fn matches(&self, parameter: &str, literal: &str) -> Option<bool> {
        let value_type = self.definition.parameter(parameter)?.meta.value_type;
        let reading = self.readings.get(parameter)?;
        Some(same_value(value_type, &reading.value, literal))
    }

    fn compare(&self, parameter: &str, literal: &str) -> Option<std::cmp::Ordering> {
        let value_type = self.definition.parameter(parameter)?.meta.value_type;
        let reading = self.readings.get(parameter)?;
        ordered(value_type, &reading.value, literal)
    }

    fn condition_test(&self, test: &str) -> Option<bool> {
        let (operator, separator) = [("!=", Test::NotEqual), ("=", Test::Equal), (">", Test::Greater), ("<", Test::Smaller)]
            .into_iter()
            .find_map(|(sep, op)| test.contains(sep).then_some((op, sep)))?;
        let parts: Vec<&str> = test.split(separator).filter(|part| !part.is_empty()).collect();
        let [name, literal] = parts.as_slice() else { return None };
        match operator {
            Test::Equal => self.matches(name, literal),
            Test::NotEqual => self.matches(name, literal).map(|hit| !hit),
            Test::Greater => self.compare(name, literal).map(|order| order == std::cmp::Ordering::Greater),
            Test::Smaller => self.compare(name, literal).map(|order| order == std::cmp::Ordering::Less),
        }
    }

    fn resolve_active(&self) -> Active {
        let judged: Vec<(Option<bool>, &Exclusion)> = self.definition.exclusions.iter().map(|exclusion| (self.matches(&exclusion.parameter, &exclusion.value), exclusion)).collect();
        let hits = |setting: &String, wanted: Option<bool>| judged.iter().filter(move |(state, _)| *state == wanted).find(|(_, e)| e.excludes.contains(setting)).map(|(_, e)| *e);
        let excluded_by: BTreeMap<String, Cause> = self
            .definition
            .settings
            .iter()
            .filter_map(|setting| hits(setting, Some(true)).map(|exclusion| (setting.clone(), Cause::of_exclusion(exclusion))))
            .collect();
        let settings: Vec<String> = self.definition.settings.iter().filter(|setting| !excluded_by.contains_key(*setting)).cloned().collect();
        Active {
            unresolved: settings.iter().filter(|setting| hits(setting, None).is_some()).cloned().collect(),
            settings,
            excluded_by,
        }
    }

    fn resolve_options(&self) -> BTreeMap<String, Options> {
        let judged: Vec<(Option<bool>, &OptionRange)> = self.definition.ranges.iter().map(|range| (both(self.matches(&range.parameter, &range.value), self.condition(&range.condition)), range)).collect();
        self.definition
            .parameters
            .iter()
            .map(|p| {
                let mine = |wanted: Option<bool>| judged.iter().filter(move |(state, _)| *state == wanted).find(|(_, range)| range.target == p.meta.name);
                let resolved = mine(None).is_none();
                let options = match mine(Some(true)) {
                    Some((_, range)) => Options { entries: range.options.clone(), resolved, limited_by: Some(Cause::of_range(range)) },
                    None => Options { entries: p.meta.enums.clone(), resolved, limited_by: None },
                };
                (p.meta.name.clone(), options)
            })
            .collect()
    }

    fn settle(&mut self) -> Vec<Out> {
        let active = self.resolve_active();
        let options = self.resolve_options();
        let announced: Vec<Out> = options
            .iter()
            .filter(|(name, next)| self.options.get(*name) != Some(next))
            .map(|(name, next)| Out::Options { parameter: name.clone(), options: (*next).clone() })
            .collect();
        let moved = (active != self.active).then(|| Out::ActiveSettings { settings: active.settings.clone(), unresolved: active.unresolved.clone() });
        self.active = active;
        self.options = options;
        moved
            .into_iter()
            .chain(announced)
            .chain((!self.pending_updates.is_empty()).then_some(Out::ScheduleUpdates { after_ms: PARAM_UPDATE_DELAY_MS }))
            .collect()
    }
}

fn parameter_json(parameters: &CameraParameters, parameter: &Parameter, now_ms: u64) -> Value {
    let meta = &parameter.meta;
    let reading = parameters.reading(&meta.name);
    let options = parameters.options(&meta.name);
    let entries = options.map(|o| o.entries.as_slice()).unwrap_or(&[]);
    let selected = reading.and_then(|r| entries.iter().position(|entry| same_typed(meta.value_type, &r.value, &entry.value)));
    json!({
        "name": meta.name,
        "type": mav_param_ext_type(meta.value_type),
        "control": meta.has_control,
        "readOnly": meta.read_only,
        "writeOnly": parameter.write_only,
        "usable": parameter.usable(),
        "description": meta.short_description,
        "units": meta.units,
        "decimalPlaces": meta.decimal_places,
        "default": meta.default,
        "min": meta.min,
        "max": meta.max,
        "step": meta.increment,
        "applicable": parameters.applicability(&meta.name).token(),
        "excludedBy": parameters.excluded_by(&meta.name).map(Cause::json),
        "value": reading.map(|r| r.value.clone()),
        "valueKnown": reading.is_some(),
        "source": reading.map(|r| r.source.token()),
        "confirmedByCamera": reading.is_some_and(|r| r.source == Source::Camera),
        "pendingWrite": reading.is_some_and(|r| r.source == Source::Requested),
        "writeFailed": reading.is_some_and(|r| r.source == Source::Failed),
        "ageMs": reading.and_then(|r| r.age_ms(now_ms)),
        "stale": reading.is_some_and(|r| r.stale(now_ms)),
        "staleAfterMs": READING_STALE_MS,
        "selected": selected.map(|index| index as i64).unwrap_or(-1),
        "valueInOptions": (reading.is_some() && !entries.is_empty()).then_some(selected.is_some()),
        "options": entries.iter().map(|entry| json!({ "label": entry.label, "value": entry.value })).collect::<Vec<_>>(),
        "optionsResolved": options.is_some_and(|o| o.resolved),
        "limitedBy": options.and_then(|o| o.limited_by.as_ref()).map(Cause::json),
        "originalOptions": meta.enums.iter().map(|entry| json!({ "label": entry.label, "value": entry.value })).collect::<Vec<_>>(),
    })
}

fn definition_json(parameters: &CameraParameters, path: &str, now_ms: u64) -> Value {
    let definition = &parameters.definition;
    json!({
        "kind": "object",
        "class": "CameraDefinition",
        "path": path,
        "readable": true,
        "valid": true,
        "errorKind": Value::Null,
        "error": "",
        "version": definition.version,
        "model": definition.model,
        "vendor": definition.vendor,
        "basic": parameters.is_basic(),
        "hasControls": parameters.has_controls(),
        "allConfirmed": parameters.all_confirmed(),
        "diagnostics": definition.diagnostics,
        "settings": definition.settings,
        "activeSettings": parameters.active_settings(),
        "unresolvedSettings": parameters.unresolved_settings(),
        "updates": definition.updates,
        "count": definition.parameters.len(),
        "parameters": definition.parameters.iter().map(|parameter| parameter_json(parameters, parameter, now_ms)).collect::<Vec<_>>(),
        "exclusions": definition.exclusions.iter().map(|exclusion| json!({ "parameter": exclusion.parameter, "value": exclusion.value, "excludes": exclusion.excludes })).collect::<Vec<_>>(),
        "ranges": definition.ranges.iter().map(|range| json!({
            "parameter": range.parameter,
            "value": range.value,
            "target": range.target,
            "condition": range.condition,
            "options": range.options.iter().map(|entry| json!({ "label": entry.label, "value": entry.value })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

pub fn camera_definition_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|path| !path.is_empty()) else { return crate::read::refused("this needs the path of a camera definition file, and optionally a locale") };
    let locale = args.get(1).map(String::as_str).unwrap_or(NEUTRAL_LOCALE);
    let Ok(bytes) = std::fs::read(path) else {
        return json!({ "kind": "object", "class": "CameraDefinition", "path": path, "readable": false, "valid": false, "errorKind": ErrorKind::Unreadable.token(), "error": "camera definition file cannot be read" });
    };
    match from_bytes(&bytes, locale) {
        Err(refusal) => json!({ "kind": "object", "class": "CameraDefinition", "path": path, "readable": true, "valid": false, "errorKind": refusal.kind.token(), "error": refusal.detail }),
        Ok(definition) => definition_json(&CameraParameters::new(definition), path, crate::hub::now_ms()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../src/Camera/camera_definition_example.xml");
    const CONSTANTS: &str = "<definition version=\"1\"><model>m</model><vendor>v</vendor></definition>";

    struct Fake;
    impl Backend for Fake {
        fn get(&self, _path: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            self.get(path)
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    fn example() -> Definition {
        parse(EXAMPLE).unwrap()
    }

    fn built(body: &str) -> Result<Definition, Refusal> {
        parse(&format!("<mavlinkcamera>{CONSTANTS}<parameters>{body}</parameters></mavlinkcamera>"))
    }

    fn entries(parameters: &CameraParameters, name: &str) -> Vec<EnumEntry> {
        parameters.options(name).map(|o| o.entries.clone()).unwrap_or_default()
    }

    fn json_of(parameters: &CameraParameters, name: &str, now_ms: u64) -> Value {
        let parameter = parameters.definition.parameter(name).unwrap();
        parameter_json(parameters, parameter, now_ms)
    }

    #[test]
    fn the_bundled_example_definition_loads_with_its_constants_options_and_rules() {
        let definition = example();
        assert_eq!((definition.version, definition.model.as_str(), definition.vendor.as_str()), (1, "SD II", "Super Dupper Industries"));
        assert_eq!(definition.settings.first().map(String::as_str), Some("CAM_MODE"), "the settings list keeps the definition order, which is the order the head shows");
        assert_eq!(definition.parameters.len(), definition.settings.len(), "every parameter in the example carries a control");
        assert!(definition.diagnostics.is_empty(), "the bundled example is well formed, so nothing may be reported against it");
        let iso = definition.parameter("CAM_ISO").unwrap();
        assert_eq!((iso.meta.value_type, iso.meta.default.clone()), (ValueType::Uint32, Some(Value::from(100))));
        assert_eq!(iso.meta.enums.iter().map(|e| e.value.as_i64().unwrap()).collect::<Vec<_>>(), vec![100, 200, 400, 800, 1600, 3200, 6400]);
        let shutter = definition.parameter("CAM_SHUTTERSPD").unwrap();
        assert_eq!(shutter.meta.value_type, ValueType::Float);
        assert_eq!(shutter.meta.enums[4].label, "1/4", "an option label is a token from the definition, never a number this core formats");
        assert_eq!(shutter.meta.enums[4].value, Value::from(0.25));
        assert_eq!(definition.parameter("CAM_AUDIOREC").unwrap().meta.value_type, ValueType::Bool);
        assert!(definition.exclusions.iter().any(|e| e.parameter == "CAM_MODE" && e.value == "0" && e.excludes == vec!["CAM_VIDRES", "CAM_VIDFMT", "CAM_AUDIOREC"]));
        let video_iso = definition.ranges.iter().find(|r| r.parameter == "CAM_MODE" && r.target == "CAM_ISO").unwrap();
        assert_eq!(video_iso.options.len(), 6, "video mode limits ISO to six of its seven options");
        assert_eq!(video_iso.options[0].value, Value::from(100), "an roption is typed against the target parameter, not the parameter that carries the rule");
        assert!(definition.ranges.iter().any(|r| r.condition == "CAM_MODE=1 AND CAM_EXPMODE=1"));
    }

    #[test]
    fn a_definition_missing_a_required_piece_is_refused_and_never_half_loaded() {
        assert!(parse("<mavlinkcamera>").unwrap_err().detail.starts_with("Unable to parse camera definition file"));
        assert!(parse("<mavlinkcamera><parameters/></mavlinkcamera>").unwrap_err().detail.contains("camera constants"));
        assert_eq!(parse(&format!("<mavlinkcamera>{CONSTANTS}</mavlinkcamera>")).unwrap_err().detail, "No parameters to load from camera");
        assert_eq!(built("").unwrap_err().detail, "Unable to load camera parameters from camera definition");
        assert_eq!(built("<parameter type=\"uint32\"><description>d</description></parameter>").unwrap_err().detail, "Parameter entry missing parameter name");
        assert_eq!(built("<parameter name=\"P\"><description>d</description></parameter>").unwrap_err().detail, "Parameter P missing parameter type");
        assert_eq!(built("<parameter name=\"P\" type=\"quad\"><description>d</description></parameter>").unwrap_err().detail, "Unknown type for parameter P");
        assert_eq!(built("<parameter name=\"P\" type=\"uint32\"/>").unwrap_err().detail, "Parameter P missing parameter description");
        assert!(built("<parameter name=\"P\" type=\"uint32\"><description>d</description><options><option value=\"1\"/></options></parameter>").unwrap_err().detail.contains("Malformed option"));
        assert!(
            built("<parameter name=\"P\" type=\"uint32\"><description>d</description><options><option name=\"a\" value=\"1\"><parameterranges><parameterrange><roption name=\"x\" value=\"1\"/></parameterrange></parameterranges></option></options></parameter>")
                .unwrap_err()
                .detail
                .contains("Malformed option range")
        );
        let twice = built("<parameter name=\"P\" type=\"uint32\" default=\"1\"><description>first</description></parameter><parameter name=\"P\" type=\"uint32\"><description>second</description></parameter>").unwrap();
        assert_eq!(twice.parameters.len(), 1, "a duplicate parameter name keeps the first entry, it never doubles the setting");
        assert_eq!(twice.parameter("P").unwrap().meta.short_description, "first");
    }

    #[test]
    fn the_refusal_carries_a_kind_a_head_can_act_on_without_reading_the_english() {
        let kinds = |body: &str| parse(body).unwrap_err().kind;
        assert_eq!(kinds("<mavlinkcamera>"), ErrorKind::NotXml, "a file that is not XML is a download or flash problem, not a camera problem");
        assert_eq!(kinds("<mavlinkcamera><parameters/></mavlinkcamera>"), ErrorKind::MissingConstants);
        assert_eq!(kinds(&format!("<mavlinkcamera>{CONSTANTS}</mavlinkcamera>")), ErrorKind::NoParameters);
        assert_eq!(built("<parameter name=\"P\" type=\"quad\"><description>d</description></parameter>").unwrap_err().kind, ErrorKind::BadParameter, "a vendor bug in one parameter is a different action from a missing file");
        assert_eq!(from_bytes(&[0xfd, b'7', b'z', b'X', b'Z', 0x00, 0x00], NEUTRAL_LOCALE).unwrap_err().kind, ErrorKind::NotXml, "an xz payload that will not inflate is not a malformed camera definition");
        assert_eq!(
            [ErrorKind::Unreadable, ErrorKind::NotXml, ErrorKind::MissingConstants, ErrorKind::NoParameters, ErrorKind::BadParameter].map(ErrorKind::token),
            ["unreadable", "notXml", "missingConstants", "noParameters", "badParameter"],
            "the tokens are the contract the head switches its own translated sentence on, so they are pinned here and not derived from the enum order"
        );
        let missing = camera_definition_view(&Fake, &["/no/such/definition.xml".to_string()]);
        assert_eq!((missing["readable"].as_bool(), missing["errorKind"].as_str()), (Some(false), Some("unreadable")));
        let broken = camera_definition_view(&Fake, &[concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").to_string()]);
        assert_eq!(broken["errorKind"], "notXml", "a readable file that is not a camera definition says which of the three failures it is");
    }

    #[test]
    fn control_read_only_and_write_only_are_reported_as_the_definition_states_them() {
        let definition = built(
            "<parameter name=\"HIDDEN\" type=\"uint32\" control=\"0\"><description>d</description></parameter><parameter name=\"BLOB\" type=\"custom\"><description>d</description></parameter><parameter name=\"BOTH\" type=\"uint32\" readonly=\"1\" writeonly=\"1\"><description>d</description></parameter><parameter name=\"SHOWN\" type=\"uint32\" unit=\"m\" min=\"1\" max=\"9\" step=\"0.5\" decimalPlaces=\"2\"><description>d</description></parameter>",
        )
        .unwrap();
        assert_eq!(definition.settings, vec!["BOTH", "SHOWN"], "control=0 and a custom type are parameters without a control, so they are not settings");
        assert!(!definition.parameter("BLOB").unwrap().meta.has_control, "a custom type never carries a control, whatever the attribute says");
        let both = definition.parameter("BOTH").unwrap();
        assert!(both.meta.read_only && both.write_only, "a contradictory definition is reported as written");
        assert!(!both.usable(), "a value that can be neither read nor written is refused here, so no head has to invent the rule and none can render a control for it");
        assert!(definition.parameter("SHOWN").unwrap().usable());
        assert!(
            definition.diagnostics.iter().any(|note| note == "Parameter BOTH cannot be both read only and write only"),
            "the contradiction is named in the diagnostics, which is where a vendor bug belongs"
        );
        let shown = definition.parameter("SHOWN").unwrap();
        assert_eq!((shown.meta.units.as_deref(), shown.meta.decimal_places, shown.meta.increment), (Some("m"), Some(2), Some(0.5)));
        assert_eq!((shown.meta.min.clone(), shown.meta.max.clone()), (Some(Value::from(1)), Some(Value::from(9))));
        let parameters = CameraParameters::new(definition);
        assert_eq!(json_of(&parameters, "BOTH", 0)["usable"], false);
        assert_eq!(json_of(&parameters, "BLOB", 0)["applicable"], "noControl", "a parameter that never had a control is not a setting a rule took away");
    }

    #[test]
    fn a_definition_of_only_custom_parameters_still_has_parameters_worth_requesting() {
        let definition = built("<parameter name=\"BLOB\" type=\"custom\"><description>d</description></parameter>").unwrap();
        let parameters = CameraParameters::new(definition);
        assert!(parameters.definition.settings.is_empty(), "a custom type carries no control, so it is on no control list the head renders");
        assert!(!parameters.is_basic(), "the C++ keeps a custom parameter in _settings, so such a camera is not basic and its parameters are still read; deriving basic from the control list skips the read entirely");
        assert!(!parameters.has_controls(), "having parameters to request and having controls to draw are two questions, and they get two fields");
    }

    #[test]
    fn a_parameter_with_no_default_stays_unknown_rather_than_reading_as_zero() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"2\"><description>d</description></parameter><parameter name=\"B\" type=\"uint32\"><description>d</description></parameter>").unwrap();
        let mut parameters = CameraParameters::new(definition);
        assert_eq!(parameters.reading("B"), None, "no default and nothing from the camera is no data, not zero");
        assert_eq!(parameters.reading("A").map(|r| r.source), Some(Source::Default), "a value that only comes from the definition is not a value the camera confirmed");
        assert!(!parameters.all_confirmed(), "a definition whose parameters have not been read is provisional, and the head is told so rather than guessing");
        parameters.on_value("A", Value::from(0), Source::Camera, 1_000);
        assert_eq!(parameters.reading("A"), Some(&Reading { value: Value::from(0), source: Source::Camera, stamped_ms: 1_000 }), "zero from the camera is data");
        assert!(parameters.on_value("NOPE", Value::from(1), Source::Camera, 1_000).is_empty(), "a parameter the definition never declared is not recorded");
        assert_eq!(parameters.reading("NOPE"), None);
        parameters.on_value("B", Value::from(7), Source::Camera, 1_000);
        assert!(parameters.all_confirmed(), "once every readable setting has been heard the answer is settled");
    }

    #[test]
    fn a_reading_the_camera_has_not_confirmed_is_never_reported_as_one_it_has() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"1\"><description>d</description></parameter>").unwrap();
        let mut parameters = CameraParameters::new(definition);
        let states = |parameters: &CameraParameters| {
            let view = json_of(parameters, "A", 0);
            (view["source"].as_str().map(str::to_string), view["confirmedByCamera"].as_bool(), view["pendingWrite"].as_bool(), view["writeFailed"].as_bool())
        };
        assert_eq!(states(&parameters), (Some("default".to_string()), Some(false), Some(false), Some(false)));
        parameters.on_value("A", Value::from(2), Source::Requested, 0);
        assert_eq!(
            states(&parameters),
            (Some("requested".to_string()), Some(false), Some(true), Some(false)),
            "a value the operator asked for is in flight, and showing it as confirmed is how a dropped write becomes a shot taken at the old setting"
        );
        parameters.on_value("A", Value::from(2), Source::Failed, 0);
        assert_eq!(states(&parameters), (Some("failed".to_string()), Some(false), Some(false), Some(true)), "retries exhausted is its own state, not a silent revert and not a confirmation");
        parameters.on_value("A", Value::from(2), Source::Camera, 0);
        assert_eq!(states(&parameters), (Some("camera".to_string()), Some(true), Some(false), Some(false)));
    }

    #[test]
    fn a_confirmed_reading_ages_and_stops_counting_as_current() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"1\"><description>d</description></parameter>").unwrap();
        let mut parameters = CameraParameters::new(definition);
        let fresh = json_of(&parameters, "A", 60_000);
        assert_eq!((fresh["ageMs"].clone(), fresh["stale"].as_bool()), (Value::Null, Some(false)), "a definition default never arrived, so it has no age and cannot expire");
        parameters.on_value("A", Value::from(800), Source::Camera, 1_000);
        let held = json_of(&parameters, "A", 1_000 + READING_STALE_MS);
        assert_eq!((held["ageMs"].as_u64(), held["stale"].as_bool()), (Some(READING_STALE_MS), Some(false)), "the expiry is inclusive of its own limit");
        let expired = json_of(&parameters, "A", 1_001 + READING_STALE_MS);
        assert_eq!(
            (expired["ageMs"].as_u64(), expired["stale"].as_bool(), expired["value"].as_i64()),
            (Some(READING_STALE_MS + 1), Some(true), Some(800)),
            "a value confirmed before the link died looks exactly like one confirmed a moment ago, and the operator triggers on the strength of it"
        );
        assert_eq!(expired["staleAfterMs"].as_u64(), Some(READING_STALE_MS), "the budget travels with the payload so the head does not keep a second copy of it");
        parameters.on_value("A", Value::from(800), Source::Camera, 2_000 + READING_STALE_MS);
        assert!(!parameters.reading("A").unwrap().stale(2_000 + READING_STALE_MS), "a fresh confirmation clears the expiry, so staleness cannot latch");
    }

    #[test]
    fn exclusions_decide_which_settings_are_applicable_and_recover_when_the_mode_changes_back() {
        let mut parameters = CameraParameters::new(example());
        assert_eq!(
            parameters.applicability("CAM_ISO"),
            Applicability::Excluded,
            "the default exposure mode Auto excludes ISO from the start, the rules run on the defaults rather than waiting for the camera"
        );
        assert_eq!(parameters.applicability("CAM_PHOTOFMT"), Applicability::Excluded, "the default video mode excludes the photo settings");
        assert_eq!(parameters.applicability("CAM_VIDRES"), Applicability::Applicable);
        let out = parameters.on_value("CAM_MODE", Value::from(0), Source::Camera, 0);
        assert!(out.iter().any(|o| matches!(o, Out::ActiveSettings { settings, .. } if settings.contains(&"CAM_PHOTOFMT".to_string()) && !settings.contains(&"CAM_VIDRES".to_string()))));
        assert_eq!((parameters.applicability("CAM_PHOTOFMT"), parameters.applicability("CAM_VIDRES")), (Applicability::Applicable, Applicability::Excluded));
        assert!(
            parameters.on_value("CAM_MODE", Value::from(0), Source::Camera, 0).iter().all(|o| !matches!(o, Out::ActiveSettings { .. })),
            "the same value again changes nothing, so nothing is announced"
        );
        parameters.on_value("CAM_MODE", Value::from(1), Source::Camera, 0);
        assert_eq!((parameters.applicability("CAM_VIDRES"), parameters.applicability("CAM_PHOTOFMT")), (Applicability::Applicable, Applicability::Excluded), "the exclusion clears when the mode goes back");
        parameters.on_value("CAM_EXPMODE", Value::from(1), Source::Camera, 0);
        assert_eq!((parameters.applicability("CAM_ISO"), parameters.applicability("CAM_EV")), (Applicability::Applicable, Applicability::Excluded), "manual exposure brings ISO back and takes the compensation away");
    }

    #[test]
    fn a_range_rule_limits_the_target_options_and_is_restored_when_it_stops_applying() {
        let mut parameters = CameraParameters::new(example());
        let full = parameters.definition.parameter("CAM_ISO").unwrap().meta.enums.len();
        assert_eq!(entries(&parameters, "CAM_ISO").len(), full - 1, "the default mode is video, which limits the ISO option set from the start");
        let back = parameters.on_value("CAM_MODE", Value::from(0), Source::Camera, 0);
        assert!(
            back.iter().any(|o| matches!(o, Out::Options { parameter, options } if parameter == "CAM_ISO" && options.entries.len() == full)),
            "photo mode restores the full ISO set, a limited set is never a latch"
        );
        let again = parameters.on_value("CAM_MODE", Value::from(1), Source::Camera, 0);
        assert!(again.iter().any(|o| matches!(o, Out::Options { parameter, options } if parameter == "CAM_ISO" && options.entries.len() == full - 1)), "video mode limits it again");
        parameters.on_value("CAM_EXPMODE", Value::from(1), Source::Camera, 0);
        parameters.on_value("CAM_VIDRES", Value::from(2), Source::Camera, 0);
        let shutter = entries(&parameters, "CAM_SHUTTERSPD");
        assert_eq!(shutter.len(), 5, "60P with manual exposure limits the shutter to the speeds no slower than the frame rate");
        assert_eq!(shutter[0].label, "1/60");
        parameters.on_value("CAM_EXPMODE", Value::from(0), Source::Camera, 0);
        assert_eq!(entries(&parameters, "CAM_SHUTTERSPD").len(), 13, "the condition stopped holding, so the full shutter set comes back");
        let rebuilt = CameraParameters::new(parameters.definition.clone());
        assert_eq!(rebuilt.reading("CAM_MODE").map(|r| r.source), Some(Source::Default), "the camera resets its own settings and is read again, so the core rebuilds from the definition rather than asserting defaults over live state");
        assert_eq!(entries(&rebuilt, "CAM_ISO").len(), full - 1, "and puts the option sets back to what those defaults imply");
    }

    #[test]
    fn a_rule_whose_parameter_has_not_been_heard_from_is_unresolved_and_never_reads_as_unrestricted() {
        let definition = built(
            "<parameter name=\"MODE\" type=\"uint32\"><description>d</description><options><option name=\"Photo\" value=\"0\"><exclusions><exclude>GAIN</exclude></exclusions></option><option name=\"Video\" value=\"1\"><parameterranges><parameterrange parameter=\"ISO\"><roption name=\"100\" value=\"100\"/><roption name=\"200\" value=\"200\"/></parameterrange></parameterranges></option></options></parameter><parameter name=\"ISO\" type=\"uint32\" default=\"100\"><description>d</description><options><option name=\"100\" value=\"100\"/><option name=\"200\" value=\"200\"/><option name=\"400\" value=\"400\"/></options></parameter><parameter name=\"GAIN\" type=\"uint32\" default=\"0\"><description>d</description></parameter>",
        )
        .unwrap();
        let mut parameters = CameraParameters::new(definition);
        assert_eq!(parameters.reading("MODE"), None, "MODE declares no default, so nothing is known about it until the camera answers");
        let iso = parameters.options("ISO").unwrap();
        assert_eq!(
            (iso.entries.len(), iso.resolved),
            (3, false),
            "not knowing the mode is not knowing the ISO set; announcing the full set as complete is what puts 400 in front of an operator the camera will refuse"
        );
        assert_eq!(parameters.applicability("GAIN"), Applicability::Unresolved, "an exclusion whose parameter has not been heard from leaves the setting unresolved, not applicable");
        assert!(parameters.unresolved_settings().contains(&"GAIN".to_string()));
        assert_eq!(json_of(&parameters, "ISO", 0)["optionsResolved"], false, "the head cannot re-derive the third state, so it travels in the payload");
        assert_eq!(json_of(&parameters, "GAIN", 0)["applicable"], "unresolved");
        let out = parameters.on_value("MODE", Value::from(1), Source::Camera, 0);
        assert!(
            out.iter().any(|o| matches!(o, Out::Options { parameter, options } if parameter == "ISO" && options.entries.len() == 2 && options.resolved)),
            "the announcement carries whether the set it names is known complete"
        );
        assert_eq!((parameters.applicability("GAIN"), parameters.options("ISO").unwrap().resolved), (Applicability::Applicable, true));
        parameters.on_value("MODE", Value::from(0), Source::Camera, 0);
        assert_eq!((entries(&parameters, "ISO").len(), parameters.applicability("GAIN")), (3, Applicability::Excluded));
    }

    #[test]
    fn a_condition_that_cannot_be_settled_leaves_the_option_set_unresolved_rather_than_full() {
        let definition = built(
            "<parameter name=\"MODE\" type=\"uint32\" default=\"1\"><description>d</description><options><option name=\"Video\" value=\"1\"><parameterranges><parameterrange parameter=\"SPD\" condition=\"RES=2\"><roption name=\"fast\" value=\"1\"/></parameterrange></parameterranges></option></options></parameter><parameter name=\"RES\" type=\"uint32\"><description>d</description></parameter><parameter name=\"SPD\" type=\"uint32\" default=\"1\"><description>d</description><options><option name=\"slow\" value=\"0\"/><option name=\"fast\" value=\"1\"/></options></parameter>",
        )
        .unwrap();
        let mut parameters = CameraParameters::new(definition);
        assert_eq!(parameters.condition("RES=2"), None, "the condition names a parameter with no reading, so it has no answer");
        assert_eq!((entries(&parameters, "SPD").len(), parameters.options("SPD").unwrap().resolved), (2, false), "an unanswerable condition is not a condition that failed");
        parameters.on_value("RES", Value::from(2), Source::Camera, 0);
        assert_eq!((entries(&parameters, "SPD").len(), parameters.options("SPD").unwrap().resolved), (1, true));
        parameters.on_value("RES", Value::from(0), Source::Camera, 0);
        assert_eq!((entries(&parameters, "SPD").len(), parameters.options("SPD").unwrap().resolved), (2, true), "a condition that definitely fails restores the full set, and says the answer is settled");
    }

    #[test]
    fn the_payload_names_the_rule_that_took_a_setting_or_an_option_away() {
        let parameters = CameraParameters::new(example());
        let iso = json_of(&parameters, "CAM_ISO", 0);
        assert_eq!(iso["applicable"], "excluded");
        assert_eq!(
            iso["excludedBy"],
            json!({ "parameter": "CAM_EXPMODE", "value": "0", "condition": "" }),
            "a greyed control with no cause costs the operator the shot; the fix is one tap away and only this module knows which rule fired"
        );
        assert_eq!(
            iso["limitedBy"],
            json!({ "parameter": "CAM_MODE", "value": "1", "condition": "" }),
            "the shortened ISO list names the mode that shortened it, rather than making the head walk the rule table with its own copy of the matching rule"
        );
        let mode = json_of(&parameters, "CAM_MODE", 0);
        assert_eq!((mode["excludedBy"].clone(), mode["limitedBy"].clone()), (Value::Null, Value::Null), "a setting nothing acted on carries no cause");
        let audio = json_of(&parameters, "CAM_AUDIOREC", 0);
        assert_eq!(audio["excludedBy"], Value::Null, "video mode is the default, so the audio setting is not excluded and has nothing to name");
    }

    #[test]
    fn conditions_and_or_chain_left_to_right_and_an_unknown_term_stays_unknown() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"1\"><description>d</description></parameter><parameter name=\"B\" type=\"uint32\" default=\"20\"><description>d</description></parameter><parameter name=\"C\" type=\"uint32\"><description>d</description></parameter>").unwrap();
        let parameters = CameraParameters::new(definition);
        assert_eq!(parameters.condition(""), Some(true), "an empty condition always holds, the rule carrying it needs no test");
        assert_eq!(parameters.condition("A=1"), Some(true));
        assert_eq!(parameters.condition("A!=1"), Some(false));
        assert_eq!(parameters.condition("A=1 AND B=20"), Some(true));
        assert_eq!(parameters.condition("A=2 AND B=20"), Some(false));
        assert_eq!(parameters.condition("A=2 OR B=20"), Some(true));
        assert_eq!(parameters.condition("B>100"), Some(false), "numbers compare as numbers, so 20 is not above 100 however the digits sort");
        assert_eq!(parameters.condition("B<100"), Some(true));
        assert_eq!(parameters.condition("C=1"), None, "a term on a parameter with no value is unknown, never a silent false");
        assert_eq!(parameters.condition("A=1 OR C=1"), None, "one unknown term makes the whole condition unknown");
        assert_eq!(parameters.condition("A"), None, "a term with no operator is not a test");
        assert_eq!(parameters.condition("Z=1"), None, "a term naming a parameter the definition does not declare is unknown");
    }

    #[test]
    fn a_float_option_still_matches_when_the_camera_reports_it_as_a_real32() {
        let mut parameters = CameraParameters::new(example());
        let real32 = decode_param_value(PARAM_EXT_TYPE_REAL32, &0.016666f32.to_le_bytes()).unwrap();
        assert_ne!(real32, Value::from(0.016666), "the wire value is a float widened to double, not the literal from the definition");
        assert!(same_value(ValueType::Float, &real32, "0.016666"), "a float option is matched at float precision, or every rule keyed on it would go dead");
        parameters.on_value("CAM_MODE", Value::from(1), Source::Camera, 0);
        parameters.on_value("CAM_EXPMODE", Value::from(1), Source::Camera, 0);
        parameters.on_value("CAM_SHUTTERSPD", real32, Source::Camera, 0);
        assert_eq!(parameters.condition("CAM_SHUTTERSPD=0.016666"), Some(true));
    }

    #[test]
    fn the_payload_names_which_option_is_selected_rather_than_leaving_the_head_to_match_a_float() {
        let mut parameters = CameraParameters::new(example());
        parameters.on_value("CAM_MODE", Value::from(1), Source::Camera, 0);
        parameters.on_value("CAM_EXPMODE", Value::from(1), Source::Camera, 0);
        let real32 = decode_param_value(PARAM_EXT_TYPE_REAL32, &0.016666f32.to_le_bytes()).unwrap();
        parameters.on_value("CAM_SHUTTERSPD", real32, Source::Camera, 0);
        let shutter = json_of(&parameters, "CAM_SHUTTERSPD", 0);
        assert_eq!(
            (shutter["selected"].as_i64(), shutter["valueInOptions"].as_bool()),
            (Some(1), Some(true)),
            "the head cannot derive the float match rule from the payload, so a shutter the camera is really at would read as no selection at all"
        );
        assert_eq!(shutter["options"][1]["label"], "1/60", "the index is into the option set the head is showing, which is the limited one, not the definition's original list");
        parameters.on_value("CAM_SHUTTERSPD", Value::from(0.5), Source::Camera, 0);
        let outside = json_of(&parameters, "CAM_SHUTTERSPD", 0);
        assert_eq!(
            (outside["selected"].as_i64(), outside["valueInOptions"].as_bool(), outside["valueKnown"].as_bool()),
            (Some(-1), Some(false), Some(true)),
            "a camera sitting on a value the rules do not allow is an honest failure the operator must see, not a blank combo box"
        );
        let unread = json_of(&CameraParameters::new(example()), "CAM_AUDIOREC", 0);
        assert_eq!(unread["valueInOptions"], Value::Null, "a parameter with no options at all is not a parameter whose value fell outside them");
        let definition = built("<parameter name=\"A\" type=\"uint32\"><description>d</description><options><option name=\"one\" value=\"1\"/></options></parameter>").unwrap();
        let never = json_of(&CameraParameters::new(definition), "A", 0);
        assert_eq!((never["selected"].as_i64(), never["valueInOptions"].clone()), (Some(-1), Value::Null), "nothing read yet is distinguishable from read and out of range");
    }

    #[test]
    fn a_bool_rule_keeps_firing_when_the_camera_reports_it_as_a_uint8() {
        let definition = built(
            "<parameter name=\"REC\" type=\"bool\" default=\"1\"><description>d</description><options><option name=\"On\" value=\"1\"><exclusions><exclude>GAIN</exclude></exclusions></option><option name=\"Off\" value=\"0\"/></options></parameter><parameter name=\"GAIN\" type=\"uint32\" default=\"0\"><description>d</description></parameter>",
        )
        .unwrap();
        let mut parameters = CameraParameters::new(definition);
        assert_eq!(parameters.applicability("GAIN"), Applicability::Excluded, "the definition default of 1 matches the option value, so the exclusion fires before the camera says anything");
        let wire = decode_param_value(PARAM_EXT_TYPE_UINT8, &[1]).unwrap();
        assert_eq!(wire, Value::from(1), "a bool goes over the wire as a uint8, which is not the Bool the definition parsed");
        parameters.on_value("REC", wire, Source::Camera, 0);
        assert_eq!(
            parameters.applicability("GAIN"),
            Applicability::Excluded,
            "a PARAM_EXT_VALUE that changed nothing must not un-fire the rule, or every exclusion keyed on a bool goes dead the moment the camera confirms it"
        );
        assert_eq!(parameters.reading("REC").map(|r| r.value.clone()), Some(Value::Bool(true)), "the wire value is coerced to the declared type once, at the boundary, so every consumer sees one shape");
        assert_eq!(json_of(&parameters, "REC", 0)["selected"].as_i64(), Some(0), "and the option the camera is on is still the one named as selected");
        parameters.on_value("REC", decode_param_value(PARAM_EXT_TYPE_UINT8, &[0]).unwrap(), Source::Camera, 0);
        assert_eq!((parameters.applicability("GAIN"), parameters.reading("REC").map(|r| r.value.clone())), (Applicability::Applicable, Some(Value::Bool(false))));
    }

    #[test]
    fn an_attribute_the_type_cannot_hold_is_dropped_with_a_diagnostic_and_never_stored_as_a_reading() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"Auto\" min=\"nope\" max=\"9\"><description>d</description></parameter>").unwrap();
        let a = definition.parameter("A").unwrap();
        assert_eq!((a.meta.default.clone(), a.meta.min.clone(), a.meta.max.clone()), (None, None, Some(Value::from(9))), "an attribute that will not parse is dropped, not kept as a null the head has to coerce");
        assert_eq!(definition.diagnostics.len(), 2, "both rejected attributes are reported, which is what the C++ warning was for");
        assert!(definition.diagnostics.iter().all(|note| note.starts_with("Parameter A declares")));
        let parameters = CameraParameters::new(definition);
        assert_eq!(parameters.reading("A"), None, "a null default is not a value the camera has, and valueKnown with a null value is the state no payload may ever carry");
        let view = json_of(&parameters, "A", 0);
        assert_eq!((view["valueKnown"].as_bool(), view["value"].clone(), view["default"].clone()), (Some(false), Value::Null, Value::Null));
        assert!(!same_value(ValueType::Uint32, &Value::Null, "Auto"), "two values neither of which parsed are not equal, or every rule keyed on an unparseable literal would fire at once");
    }

    #[test]
    fn a_value_outside_its_declared_type_is_refused_the_way_the_cpp_range_checks_it() {
        assert_eq!(typed(ValueType::Uint8, "300"), None, "300 in a uint8 becomes 44 on the wire, so the definition is refused instead of quietly sending a different setting");
        assert_eq!(typed(ValueType::Uint32, "-1"), None, "-1 in a uint32 becomes 4294967295, which is not what the vendor wrote");
        assert_eq!(typed(ValueType::Int8, "-128"), Some(Value::from(-128)), "the limits themselves are in range");
        assert_eq!(typed(ValueType::Uint8, "255"), Some(Value::from(255)));
        assert_eq!(typed(ValueType::Float, "0.25"), Some(Value::from(0.25)));
        assert_eq!(typed(ValueType::Uint32, "nope"), None);
        let dropped = built("<parameter name=\"A\" type=\"uint8\" default=\"300\"><description>d</description></parameter>").unwrap();
        assert_eq!(dropped.parameter("A").unwrap().meta.default, None, "an out of range default is not the camera's default");
        assert_eq!(dropped.diagnostics.len(), 1);
        assert!(built("<parameter name=\"A\" type=\"uint8\"><description>d</description><options><option name=\"big\" value=\"300\"/></options></parameter>").unwrap_err().detail.contains("which its type cannot hold"),
            "an option the type cannot hold is refused like a missing option value, since the head would offer it and the camera would reject the write");
        let orphan = built(
            "<parameter name=\"A\" type=\"uint32\" default=\"1\"><description>d</description><options><option name=\"one\" value=\"1\"><parameterranges><parameterrange parameter=\"GONE\"><roption name=\"x\" value=\"1\"/></parameterrange></parameterranges></option></options></parameter>",
        )
        .unwrap();
        assert!(orphan.ranges.is_empty(), "a rule aimed at a parameter the definition never declares limits nothing, and is dropped rather than typed as a string");
        assert_eq!(orphan.diagnostics.len(), 1);
    }

    #[test]
    fn a_parameter_that_asks_for_updates_schedules_them_once_and_the_list_clears_when_taken() {
        let definition = built("<parameter name=\"A\" type=\"uint32\" default=\"1\"><description>d</description><updates><update>B</update></updates></parameter><parameter name=\"B\" type=\"uint32\" default=\"1\"><description>d</description></parameter>").unwrap();
        assert_eq!(definition.updates.get("A").cloned(), Some(vec!["B".to_string()]));
        let mut parameters = CameraParameters::new(definition);
        let out = parameters.on_value("A", Value::from(2), Source::Camera, 0);
        assert!(out.contains(&Out::ScheduleUpdates { after_ms: PARAM_UPDATE_DELAY_MS }), "the C++ waits exactly 500 ms before asking, so the number stays");
        assert_eq!(parameters.take_pending_updates(), vec!["B".to_string()]);
        assert!(parameters.take_pending_updates().is_empty(), "taking the list clears it, an update request is never asked for twice");
        assert!(
            parameters.on_value("B", Value::from(3), Source::Camera, 0).iter().all(|o| !matches!(o, Out::ScheduleUpdates { .. })),
            "a parameter nothing depends on schedules nothing"
        );
    }

    #[test]
    fn the_locale_is_chosen_by_exact_name_then_by_language_and_leaves_the_rest_alone() {
        let translated = parse(&localize(EXAMPLE, "pt_BR")).unwrap();
        assert_eq!(translated.parameter("CAM_MODE").unwrap().meta.short_description, "Modo de Operação");
        assert_eq!(translated.parameter("CAM_WBMODE").unwrap().meta.enums[0].label, "Automático", "an option name is translated too, it is the token the head shows");
        assert_eq!(parse(&localize(EXAMPLE, "pt")).unwrap().parameter("CAM_MODE").unwrap().meta.short_description, "Modo de Operação", "a bare language picks the first locale of that language");
        assert_eq!(parse(&localize(EXAMPLE, "pt-br")).unwrap().parameter("CAM_MODE").unwrap().meta.short_description, "Modo de Operação", "a dash is the same locale as an underscore");
        let untouched = parse(&localize(EXAMPLE, "uk_UA")).unwrap();
        assert_eq!(untouched.parameter("CAM_MODE").unwrap().meta.short_description, "Camera Mode", "a locale the definition never carries leaves the original strings in place");
        assert_eq!(localize(EXAMPLE, "en_US"), EXAMPLE, "the neutral locale is a no-op, not a pass over every string");
        assert_eq!(localize("<mavlinkcamera/>", "pt_BR"), "<mavlinkcamera/>", "a definition with no localization block survives being asked for one");
        assert_eq!(translated.parameter("CAM_MODE").unwrap().meta.enums.len(), 2, "translation replaces text, it never drops an option");
    }

    #[test]
    fn the_fact_type_decides_the_param_ext_type_and_an_unknown_wire_type_is_no_value() {
        assert_eq!(mav_param_ext_type(ValueType::Bool), PARAM_EXT_TYPE_UINT8);
        assert_eq!(mav_param_ext_type(ValueType::Float), PARAM_EXT_TYPE_REAL32);
        assert_eq!(mav_param_ext_type(ValueType::Double), PARAM_EXT_TYPE_REAL64);
        assert_eq!(mav_param_ext_type(ValueType::String), PARAM_EXT_TYPE_CUSTOM);
        assert_eq!(mav_param_ext_type(ValueType::Custom), PARAM_EXT_TYPE_CUSTOM);
        assert_eq!(mav_param_ext_type(ValueType::ElapsedSeconds), PARAM_EXT_TYPE_INT32, "an unsupported fact type falls back to int32 the way the C++ does");
        assert_eq!(decode_param_value(PARAM_EXT_TYPE_INT32, &(-7i32).to_le_bytes()), Some(Value::from(-7)));
        assert_eq!(decode_param_value(PARAM_EXT_TYPE_UINT64, &u64::MAX.to_le_bytes()), Some(Value::from(u64::MAX)));
        let mut custom = vec![0u8; PARAM_VALUE_BYTES];
        custom[..3].copy_from_slice(b"abc");
        assert_eq!(decode_param_value(PARAM_EXT_TYPE_CUSTOM, &custom), Some(Value::String("abc".into())), "a custom value ends at its first null, the padding is not part of it");
        assert_eq!(decode_param_value(PARAM_EXT_TYPE_CUSTOM + 1, &custom), None, "a wire type this core does not know is no value at all, never zero");
        assert_eq!(decode_param_value(PARAM_EXT_TYPE_REAL64, &[0u8; 2]), None, "a truncated payload is no value");
    }

    #[test]
    fn the_param_io_budgets_keep_the_cpp_numbers() {
        assert_eq!(
            [
                PARAM_EXT_TYPE_UINT8,
                PARAM_EXT_TYPE_INT8,
                PARAM_EXT_TYPE_UINT16,
                PARAM_EXT_TYPE_INT16,
                PARAM_EXT_TYPE_UINT32,
                PARAM_EXT_TYPE_INT32,
                PARAM_EXT_TYPE_UINT64,
                PARAM_EXT_TYPE_INT64,
                PARAM_EXT_TYPE_REAL32,
                PARAM_EXT_TYPE_REAL64,
                PARAM_EXT_TYPE_CUSTOM
            ],
            [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            "these go into param_ext_set.param_type, so a renumbering here makes the camera decode a different type out of the same 128 bytes"
        );
        assert_eq!((PARAM_WRITE_TIMEOUT_MS, PARAM_REQUEST_TIMEOUT_MS, PARAM_UPDATE_DELAY_MS, PARAM_VALUE_BYTES), (3000, 3500, 500, 128));
        assert_eq!(
            (WRITE_TIMEOUT_RETRIES, FAILED_ACK_RETRIES, REQUEST_RETRIES),
            (3, 3, 3),
            "three budgets that happen to share a number are still three budgets: the write timeout aborts on ++sent > 3 (four sends), a FAILED ack retries only while ++sent < 3, the request path aborts on ++requested > 3, and an IN_PROGRESS ack restarts the write timer without incrementing anything at all"
        );
    }

    #[test]
    fn the_view_reports_the_definition_it_was_handed_and_says_so_when_it_cannot() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/Camera/camera_definition_example.xml");
        let view = camera_definition_view(&Fake, &[path.to_string()]);
        assert_eq!(view["valid"], true);
        assert_eq!(view["errorKind"], Value::Null);
        assert_eq!(view["model"], "SD II");
        assert_eq!(view["version"], 1);
        assert_eq!((view["basic"].as_bool(), view["hasControls"].as_bool(), view["allConfirmed"].as_bool()), (Some(false), Some(true), Some(false)));
        assert_eq!(view["diagnostics"], json!([]));
        assert_eq!(view["updates"], json!({}), "nothing in the example asks for an update after a change, and the request list belongs to the definition rather than to every parameter payload");
        assert_eq!(view["parameters"][0]["name"], "CAM_MODE");
        assert_eq!(view["parameters"][0]["type"], PARAM_EXT_TYPE_UINT32, "a parameter carries its wire type as a number, the head names it");
        assert_eq!(view["parameters"][0]["confirmedByCamera"], false, "nothing has been heard from a camera, and the view says that rather than implying a reading");
        assert_eq!(view["unresolvedSettings"], json!([]), "every parameter in the example carries a default, so nothing is unresolved");
        let iso = view["parameters"].as_array().unwrap().iter().find(|p| p["name"] == "CAM_ISO").unwrap().clone();
        assert_eq!(iso["applicable"], "excluded", "the default exposure mode excludes ISO");
        assert_eq!(iso["options"][0]["label"], "100");
        assert_eq!(iso["optionsResolved"], true);
        assert_eq!(camera_definition_view(&Fake, &[])["kind"], "null");
    }
}
