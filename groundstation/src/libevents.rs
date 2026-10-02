use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::Value;

pub const ARGUMENT_BYTES: usize = 40;
pub const PROFILE: &str = "dev";
const LOG_ERROR: u8 = 3;
const LOG_WARNING: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Base {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
}

fn base(name: &str) -> Option<Base> {
    Some(match name {
        "uint8_t" => Base::U8,
        "int8_t" => Base::I8,
        "uint16_t" => Base::U16,
        "int16_t" => Base::I16,
        "uint32_t" => Base::U32,
        "int32_t" => Base::I32,
        "uint64_t" => Base::U64,
        "int64_t" => Base::I64,
        "float" => Base::F32,
        _ => return None,
    })
}

fn size(base: Base) -> usize {
    match base {
        Base::U8 | Base::I8 => 1,
        Base::U16 | Base::I16 => 2,
        Base::U32 | Base::I32 | Base::F32 => 4,
        Base::U64 | Base::I64 => 8,
    }
}

#[derive(Debug)]
pub struct Enum {
    base: Base,
    bitfield: bool,
    separator: String,
    entries: BTreeMap<u64, (String, String)>,
}

#[derive(Debug)]
struct Argument {
    base: Base,
    enumeration: Option<Arc<Enum>>,
}

#[derive(Debug)]
struct Event {
    group: String,
    kind: String,
    message: String,
    description: String,
    arguments: Vec<Argument>,
}

#[derive(Debug, Default)]
pub struct Definitions {
    events: HashMap<u32, Event>,
    protocols: HashMap<u8, BTreeSet<String>>,
    mode_groups: HashMap<u8, BTreeMap<usize, BTreeSet<u32>>>,
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn parse_enum(value: &Value) -> Option<Enum> {
    let entries = value.get("entries").and_then(Value::as_object).map(|entries| entries.iter().filter_map(|(key, entry)| Some((key.parse().ok()?, (text(entry, "name"), text(entry, "description"))))).collect()).unwrap_or_default();
    Some(Enum {
        base: base(value.get("type")?.as_str()?)?,
        bitfield: value.get("is_bitfield").and_then(Value::as_bool).unwrap_or(false),
        separator: value.get("separator").and_then(Value::as_str).unwrap_or("|").to_string(),
        entries,
    })
}

pub fn parse(text_json: &str) -> Result<Definitions, String> {
    let root: Value = serde_json::from_str(text_json).map_err(|e| format!("Event metadata could not be parsed: {e}"))?;
    if root.get("version").and_then(Value::as_i64).unwrap_or(0) < 1 {
        return Err("Event metadata has no supported version".into());
    }
    let components: Vec<(u8, &Value, String)> = root
        .get("components")
        .and_then(Value::as_object)
        .map(|c| c.iter().filter_map(|(key, component)| Some(((key.parse::<u32>().ok()? & 0xff) as u8, component, component.get("namespace")?.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    let enums: HashMap<(String, String), Arc<Enum>> = components
        .iter()
        .flat_map(|(_, component, namespace)| {
            component.get("enums").and_then(Value::as_object).into_iter().flatten().filter_map(move |(name, value)| Some(((namespace.clone(), name.clone()), Arc::new(parse_enum(value)?))))
        })
        .collect();
    let argument = |namespace: &str, value: &Value| -> Option<Argument> {
        let named = value.get("type")?.as_str()?;
        match base(named) {
            Some(base) => Some(Argument { base, enumeration: None }),
            None => {
                let (space, name) = named.split_once("::").unwrap_or((namespace, named));
                let enumeration = enums.get(&(space.to_string(), name.to_string()))?.clone();
                Some(Argument { base: enumeration.base, enumeration: Some(enumeration) })
            }
        }
    };
    let events = components
        .iter()
        .flat_map(|(id, component, namespace)| {
            let argument = &argument;
            component.get("event_groups").and_then(Value::as_object).into_iter().flatten().flat_map(move |(group, value)| {
                value.get("events").and_then(Value::as_object).into_iter().flatten().filter_map(move |(key, event)| {
                    let arguments = event.get("arguments").and_then(Value::as_array).map(|listed| listed.iter().map(|a| argument(namespace, a)).collect::<Option<Vec<_>>>()).unwrap_or(Some(Vec::new()))?;
                    let full_id = (key.parse::<u32>().ok()? & 0x00ff_ffff) | (u32::from(*id) << 24);
                    Some((full_id, Event { group: group.clone(), kind: text(event, "type"), message: event.get("message")?.as_str()?.to_string(), description: text(event, "description"), arguments }))
                })
            })
        })
        .fold(HashMap::new(), |mut found, (id, event)| {
            found.entry(id).or_insert(event);
            found
        });
    let protocols = components.iter().filter_map(|(id, component, _)| Some((*id, component.get("supported_protocols")?.as_array()?.iter().filter_map(|p| p.as_str().map(str::to_string)).collect()))).collect();
    let mode_groups = components
        .iter()
        .filter_map(|(id, component, _)| {
            let groups = component.get("navigation_mode_groups")?.get("groups")?.as_object()?;
            Some((*id, groups.iter().filter_map(|(index, modes)| Some((index.parse().ok()?, modes.as_array()?.iter().filter_map(|m| m.as_u64().and_then(|m| u32::try_from(m).ok())).collect()))).collect()))
        })
        .collect();
    Ok(Definitions { events, protocols, mode_groups })
}

pub struct Parsed<'a> {
    event: &'a Event,
    arguments: [u8; ARGUMENT_BYTES],
    pub log_levels: u8,
}

impl Definitions {
    pub fn parse_event(&self, id: u32, arguments: [u8; ARGUMENT_BYTES], log_levels: u8) -> Option<Parsed<'_>> {
        self.events.get(&id).map(|event| Parsed { event, arguments, log_levels })
    }

    pub fn mode_group(&self, component: u8, custom_mode: u32) -> Option<usize> {
        self.mode_groups.get(&component)?.iter().find(|(_, modes)| modes.contains(&custom_mode)).map(|(group, _)| *group)
    }

    pub fn supports_checks(&self, component: u8) -> bool {
        self.protocols.get(&component).is_some_and(|p| p.contains("health_and_arming_check"))
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn find_unescaped(chars: &[char], wanted: &[char], from: usize) -> Option<usize> {
    let mut at = from;
    while at < chars.len() {
        if chars[at] == '\\' {
            at += 2;
            continue;
        }
        if wanted.contains(&chars[at]) {
            return Some(at);
        }
        at += 1;
    }
    None
}

fn find_text(chars: &[char], needle: &str, from: usize) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (from..chars.len().saturating_sub(needle.len() - 1)).find(|at| chars[*at..*at + needle.len()] == needle[..])
}

impl Parsed<'_> {
    pub fn group(&self) -> &str {
        &self.event.group
    }

    pub fn kind(&self) -> &str {
        &self.event.kind
    }

    pub fn argument_count(&self) -> usize {
        self.event.arguments.len()
    }

    pub fn is_enum(&self, index: usize) -> bool {
        self.event.arguments.get(index).is_some_and(|a| a.enumeration.is_some())
    }

    pub fn enum_entries(&self, index: usize) -> Vec<(u64, String, String)> {
        self.event.arguments.get(index).and_then(|a| a.enumeration.as_ref()).map(|e| e.entries.iter().map(|(value, (name, description))| (*value, name.clone(), description.clone())).collect()).unwrap_or_default()
    }

    fn bytes(&self, index: usize) -> Option<(Base, &[u8])> {
        let offset: usize = self.event.arguments.get(..index)?.iter().map(|a| size(a.base)).sum();
        let base = self.event.arguments.get(index)?.base;
        self.arguments.get(offset..offset + size(base)).map(|bytes| (base, bytes))
    }

    pub fn int(&self, index: usize) -> u64 {
        let Some((base, b)) = self.bytes(index) else { return 0 };
        let raw = |n: usize| b[..n].iter().rev().fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
        match base {
            Base::U8 | Base::U16 | Base::U32 | Base::U64 => raw(size(base)),
            Base::I8 => i64::from(b[0] as i8) as u64,
            Base::I16 => i64::from(i16::from_le_bytes([b[0], b[1]])) as u64,
            Base::I32 => i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])) as u64,
            Base::I64 => raw(8),
            Base::F32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64,
        }
    }

    fn float(&self, index: usize) -> f32 {
        self.bytes(index).map(|(_, b)| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0.0)
    }

    fn formatted(&self, index: usize, digits: Option<usize>, unit: &str) -> String {
        let argument = &self.event.arguments[index];
        let value = self.int(index);
        if let Some(enumeration) = &argument.enumeration {
            let describe = |v: u64| enumeration.entries.get(&v).map(|(_, d)| d.clone()).unwrap_or_else(|| format!("(unknown: {value})"));
            return match enumeration.bitfield {
                true => (0..size(enumeration.base) * 8).map(|bit| 1u64 << bit).filter(|bit| value & bit != 0).map(describe).collect::<Vec<_>>().join(&enumeration.separator),
                false => describe(value),
            };
        }
        let number = match argument.base {
            Base::F32 => match digits {
                Some(digits) => format!("{:.*}", digits, self.float(index)),
                None => format!("{}", self.float(index)),
            },
            Base::I8 | Base::I16 | Base::I32 | Base::I64 => (value as i64).to_string(),
            _ => value.to_string(),
        };
        match unit {
            "" => number,
            "m_v" => format!("{number} m"),
            unit => format!("{number} {unit}"),
        }
    }

    pub fn message(&self) -> String {
        self.processed(&self.event.message)
    }

    pub fn description(&self) -> String {
        self.processed(&self.event.description)
    }

    fn processed(&self, template: &str) -> String {
        let mut chars: Vec<char> = template.chars().collect();
        let mut escape_start = 0;
        let mut at = 0;
        let escape_up_to = |chars: &[char], start: usize, end: usize| -> (String, usize) {
            let head: String = chars[..start].iter().collect();
            let escaped = escape(&chars[start..end].iter().collect::<String>());
            let length = head.chars().count() + escaped.chars().count();
            (head + &escaped, length)
        };
        while at < chars.len() {
            match chars[at] {
                '\\' => {
                    chars.remove(at);
                    at += 1;
                }
                '<' => {
                    let (Some(tag_end), Some(content_start)) = (find_unescaped(&chars, &['>', ' '], at), find_unescaped(&chars, &['>'], at)) else {
                        at += 1;
                        continue;
                    };
                    let tag: String = chars[at + 1..tag_end].iter().collect();
                    let Some(closing) = find_text(&chars, &format!("</{tag}>"), tag_end) else {
                        at += 1;
                        continue;
                    };
                    let content: String = chars[content_start + 1..closing].iter().collect();
                    let attribute = (chars[tag_end] == ' ')
                        .then(|| {
                            let attributes: String = chars[tag_end + 1..content_start].iter().collect();
                            let equal = attributes.find("=\"")?;
                            let rest = &attributes[equal + 2..];
                            Some((attributes[..equal].to_string(), rest[..rest.find('"')?].to_string()))
                        })
                        .flatten();
                    let (replacement, skip) = match tag.as_str() {
                        "param" => {
                            let link = format!("<a href=\"param://{0}\">{0}</a>", escape(&content));
                            let length = link.chars().count();
                            (link, length)
                        }
                        "a" => {
                            let href = attribute.as_ref().filter(|(name, value)| name == "href" && !value.is_empty()).map(|(_, value)| value.clone()).unwrap_or_else(|| content.clone());
                            let link = format!("<a href=\"{href}\">{}</a>", escape(&content));
                            let length = link.chars().count();
                            (link, length)
                        }
                        "profile" => {
                            let shown = match attribute.as_ref().filter(|(name, value)| name == "name" && !value.is_empty()) {
                                Some((_, value)) => match value.strip_prefix('!') {
                                    Some(excluded) => excluded != PROFILE,
                                    None => value == PROFILE,
                                },
                                None => true,
                            };
                            (if shown { content } else { String::new() }, 0)
                        }
                        _ => (String::new(), 0),
                    };
                    let (head, head_length) = escape_up_to(&chars, escape_start, at);
                    let tail: String = chars[closing + tag.chars().count() + 3..].iter().collect();
                    chars = format!("{head}{replacement}{tail}").chars().collect();
                    at = head_length + skip;
                    escape_start = at;
                }
                '{' => {
                    let Some(end) = chars[at..].iter().position(|c| *c == '}').map(|p| p + at) else {
                        at += 1;
                        continue;
                    };
                    let format: String = chars[at + 1..end].iter().collect();
                    let digits_end = format.find(|c: char| !c.is_ascii_digit()).unwrap_or(format.len());
                    let Some(index) = format[..digits_end].parse::<usize>().ok().and_then(|i| i.checked_sub(1)).filter(|i| *i < self.event.arguments.len()) else {
                        at += 1;
                        continue;
                    };
                    let rest = &format[digits_end..];
                    let (decimals, unit) = match rest.strip_prefix(":.") {
                        Some(after) => {
                            let split = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
                            (after[..split].parse().ok(), &after[split..])
                        }
                        None => (None, rest.strip_prefix(':').unwrap_or(rest)),
                    };
                    let argument = self.formatted(index, decimals, unit);
                    let (head, head_length) = escape_up_to(&chars, escape_start, at);
                    let tail: String = chars[end + 1..].iter().collect();
                    chars = format!("{head}{argument}{tail}").chars().collect();
                    at = head_length + argument.chars().count();
                    escape_start = at;
                }
                _ => at += 1,
            }
        }
        let (whole, _) = escape_up_to(&chars, escape_start, chars.len());
        whole.trim_matches([' ', '\n', '\r', '\t', '\u{c}', '\u{b}']).to_string()
    }
}

pub fn external_level(log_levels: u8) -> u8 {
    log_levels & 0x0f
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sequence {
    Older,
    Equal,
    Newer,
}

pub fn compare(expected: u16, incoming: u16) -> Sequence {
    match incoming.wrapping_sub(expected) {
        0 => Sequence::Equal,
        diff if diff > u16::MAX / 2 => Sequence::Older,
        _ => Sequence::Newer,
    }
}

pub const RETRY_MS: u64 = 100;

#[derive(Debug, Default)]
pub struct Receiver {
    latest: Option<u16>,
    current: Option<u16>,
    last_timestamp_ms: u32,
    pub retry: Option<(u64, u16)>,
}

pub struct Incoming {
    pub sequence: u16,
    pub timestamp_ms: u32,
    pub for_us: bool,
}

impl Receiver {
    fn request(&mut self, sequence: u16, now_ms: u64) -> Option<u16> {
        self.retry = Some((now_ms + RETRY_MS, sequence));
        Some(sequence)
    }

    pub fn on_event(&mut self, event: &Incoming, now_ms: u64) -> (bool, Option<u16>) {
        if self.last_timestamp_ms == 0 {
            self.last_timestamp_ms = event.timestamp_ms;
        }
        if event.timestamp_ms.saturating_add(10_000) < self.last_timestamp_ms && self.last_timestamp_ms < u32::MAX - 60_000 {
            self.latest = None;
            self.current = None;
        }
        let latest = *self.latest.get_or_insert(event.sequence.wrapping_sub(1));
        match compare(latest.wrapping_add(1), event.sequence) {
            Sequence::Older => return (false, None),
            Sequence::Newer => return (false, self.request(latest.wrapping_add(1), now_ms)),
            Sequence::Equal => self.latest = Some(event.sequence),
        }
        self.last_timestamp_ms = event.timestamp_ms;
        self.retry = None;
        let behind = self.current.filter(|current| compare(event.sequence, *current) == Sequence::Newer).and_then(|_| self.request(event.sequence.wrapping_add(1), now_ms));
        (event.for_us, behind)
    }

    pub fn on_current(&mut self, sequence: u16, reset: bool, now_ms: u64) -> Option<u16> {
        if reset {
            self.latest = None;
        }
        let latest = *self.latest.get_or_insert(sequence);
        self.current = Some(sequence);
        (compare(latest, sequence) == Sequence::Newer).then(|| self.request(latest.wrapping_add(1), now_ms)).flatten()
    }

    pub fn on_error(&mut self, sequence: u16, oldest_available: u16, now_ms: u64) -> Option<u16> {
        let latest = self.latest?;
        (compare(latest.wrapping_add(1), sequence) == Sequence::Equal).then(|| {
            self.latest = Some(oldest_available.wrapping_sub(1));
            oldest_available
        }).and_then(|next| self.request(next, now_ms))
    }

    pub fn on_tick(&mut self, now_ms: u64) -> Option<u16> {
        let (due, _) = self.retry?;
        (now_ms >= due).then(|| self.latest.map(|l| l.wrapping_add(1))).flatten().and_then(|next| self.request(next, now_ms))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HealthComponent {
    bitmask: u64,
    arming_error: bool,
    arming_warning: bool,
    error: bool,
    warning: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub message: String,
    pub description: String,
    pub groups: u64,
    pub log_levels: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Results {
    groups: Vec<(bool, bool)>,
    health: BTreeMap<String, HealthComponent>,
    checks: Vec<Check>,
}

impl Results {
    pub fn can_arm(&self, group: usize) -> bool {
        self.groups.get(group).is_some_and(|(arm, _)| *arm)
    }

    pub fn checks_for(&self, group: usize) -> Vec<&Check> {
        self.checks.iter().filter(|c| group < 64 && c.groups & (1u64 << group) != 0).collect()
    }

    pub fn gps_state(&self) -> Option<&'static str> {
        self.health.get("gps").map(|gps| match () {
            _ if gps.error || gps.arming_error => "red",
            _ if gps.warning || gps.arming_warning => "yellow",
            _ => "green",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    ArmingSummary,
    Other,
}

#[derive(Debug)]
pub struct Checks {
    expect: Expect,
    chunk: i64,
    chunks: BTreeMap<i64, Results>,
    pub results: Option<Results>,
}

impl Default for Checks {
    fn default() -> Self {
        Self { expect: Expect::ArmingSummary, chunk: 0, chunks: BTreeMap::new(), results: None }
    }
}

impl Checks {
    pub fn reset(&mut self) {
        self.expect = Expect::ArmingSummary;
    }

    pub fn handle(&mut self, event: &Parsed) -> bool {
        let summary = event.kind() == "summary";
        let arming = event.group() == "arming_check";
        let health = event.group() == "health";
        match (arming || health, summary, arming) {
            (false, _, _) => false,
            (true, true, true) => {
                self.reset();
                if event.argument_count() >= 5 && event.is_enum(1) && event.is_enum(3) {
                    self.chunk = event.int(0) as i64;
                    let (error, warning, can_arm, can_run) = (event.int(1), event.int(2), event.int(3), event.int(4));
                    let groups = event.enum_entries(3).into_iter().fold(Vec::new(), |mut groups: Vec<(bool, bool)>, (bit, _, _)| {
                        let index = bit.checked_ilog2().unwrap_or(0) as usize;
                        if index >= groups.len() {
                            groups.resize(index + 1, (false, false));
                        }
                        groups[index] = (can_arm & bit != 0, can_run & bit != 0);
                        groups
                    });
                    let health = event.enum_entries(1).into_iter().map(|(bit, name, _)| (name, HealthComponent { bitmask: bit, arming_error: error & bit != 0, arming_warning: warning & bit != 0, ..HealthComponent::default() })).collect();
                    self.chunks.insert(self.chunk, Results { groups, health, checks: Vec::new() });
                    self.expect = Expect::Other;
                }
                false
            }
            (true, false, _) if self.expect == Expect::Other => {
                let check = Check { message: event.message(), description: event.description(), groups: event.int(0), log_levels: event.log_levels };
                self.chunks.entry(self.chunk).or_default().checks.push(check);
                false
            }
            (true, true, false) if self.expect == Expect::Other => {
                let accepted = event.argument_count() >= 4 && event.int(0) as i64 == self.chunk;
                if accepted {
                    let (error, warning) = (event.int(2), event.int(3));
                    let chunk = self.chunks.entry(self.chunk).or_default();
                    chunk.health.values_mut().for_each(|component| {
                        component.error = error & component.bitmask != 0;
                        component.warning = warning & component.bitmask != 0;
                    });
                    self.results = Some(self.combined());
                }
                self.reset();
                accepted
            }
            _ => {
                self.reset();
                false
            }
        }
    }

    fn combined(&self) -> Results {
        self.chunks.values().fold(Results::default(), |mut all, chunk| {
            if all.groups.is_empty() {
                all.groups = chunk.groups.clone();
            } else if all.groups.len() == chunk.groups.len() {
                all.groups.iter_mut().zip(&chunk.groups).for_each(|(a, c)| *a = (a.0 && c.0, a.1 && c.1));
            }
            chunk.health.iter().for_each(|(name, component)| {
                all.health
                    .entry(name.clone())
                    .and_modify(|known| {
                        *known = HealthComponent { bitmask: 0, arming_error: known.arming_error || component.arming_error, arming_warning: known.arming_warning || component.arming_warning, error: known.error || component.error, warning: known.warning || component.warning };
                    })
                    .or_insert_with(|| component.clone());
            });
            all.checks.extend(chunk.checks.iter().cloned());
            all
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub can_arm: bool,
    pub can_takeoff: bool,
    pub can_start_mission: bool,
    pub has_warnings_or_errors: bool,
    pub gps_state: String,
    pub problems: Vec<(String, String, &'static str)>,
}

pub fn severity(log_levels: u8) -> &'static str {
    match external_level(log_levels) {
        level if level <= LOG_ERROR => "error",
        level if level <= LOG_WARNING => "warning",
        _ => "",
    }
}

pub fn report(results: &Results, group: usize, takeoff_group: Option<usize>, mission_group: Option<usize>, previous: Option<&Report>) -> Report {
    let problems: Vec<(String, String, &'static str)> = results.checks_for(group).into_iter().map(|c| (c.message.clone(), c.description.replace('\n', "<br/>"), severity(c.log_levels))).collect();
    Report {
        can_arm: results.can_arm(group),
        can_takeoff: takeoff_group.map(|g| results.can_arm(g)).unwrap_or(previous.is_none_or(|p| p.can_takeoff)),
        can_start_mission: mission_group.map(|g| results.can_arm(g)).unwrap_or(previous.is_none_or(|p| p.can_start_mission)),
        has_warnings_or_errors: problems.iter().any(|(_, _, s)| !s.is_empty()),
        gps_state: results.gps_state().map(str::to_string).or_else(|| previous.map(|p| p.gps_state.clone())).unwrap_or_default(),
        problems,
    }
}

const MAX_PENDING: usize = 50;
const LOG_INFO: u8 = 6;

#[derive(Debug)]
pub struct Raw {
    pub id: u32,
    pub arguments: [u8; ARGUMENT_BYTES],
    pub log_levels: u8,
}

#[derive(Debug, PartialEq)]
pub enum Delivered {
    Checks,
    Message { severity: u8, text: String },
}

#[derive(Debug, Default)]
pub struct Session {
    pub definitions: Option<Definitions>,
    pub receiver: Receiver,
    pending: Vec<Raw>,
    pub checks: Checks,
    pub report: Option<Report>,
}

impl Session {
    pub fn supports_checks(&self, component: u8) -> bool {
        self.definitions.as_ref().is_some_and(|d| d.supports_checks(component))
    }

    pub fn load(&mut self, definitions: Definitions, component: u8) -> Vec<Delivered> {
        self.definitions = Some(definitions);
        std::mem::take(&mut self.pending).into_iter().filter_map(|raw| self.deliver(raw, component)).collect()
    }

    pub fn deliver(&mut self, raw: Raw, component: u8) -> Option<Delivered> {
        let Some(definitions) = &self.definitions else {
            if self.pending.len() > MAX_PENDING {
                self.pending.clear();
            }
            self.pending.push(raw);
            return None;
        };
        let parsed = definitions.parse_event(raw.id, raw.arguments, raw.log_levels)?;
        if self.checks.handle(&parsed) {
            return Some(Delivered::Checks);
        }
        let level = external_level(parsed.log_levels);
        if parsed.group() != "default" || level > LOG_INFO {
            return None;
        }
        let appended = (parsed.kind() == "append_health_and_arming_messages" && parsed.argument_count() > 0)
            .then(|| {
                let group = definitions.mode_group(component, parsed.int(0) as u32)?;
                let checks = self.checks.results.as_ref().map(|r| r.checks_for(group)).unwrap_or_default();
                let serious: Vec<&str> = checks.iter().filter(|c| external_level(c.log_levels) <= LOG_WARNING).map(|c| c.message.as_str()).collect();
                Some(if serious.is_empty() { checks.iter().map(|c| c.message.as_str()).collect() } else { serious })
            })
            .flatten()
            .unwrap_or_default();
        let message = parsed.message();
        let joined = match appended.as_slice() {
            [] => String::new(),
            [only] => (*only).to_string(),
            many => many.iter().map(|m| format!("- {m}\n")).collect(),
        };
        let separator = if !message.is_empty() && !joined.is_empty() { "\n" } else { "" };
        let text = format!("{message}{separator}{joined}");
        (!text.is_empty()).then_some(Delivered::Message { severity: level, text })
    }

    pub fn refresh(&mut self, component: u8, custom_mode: u32, takeoff_mode: Option<u32>, mission_mode: Option<u32>) {
        let Some(definitions) = &self.definitions else { return };
        let Some(results) = &self.checks.results else { return };
        let Some(group) = definitions.mode_group(component, custom_mode) else { return };
        let takeoff = takeoff_mode.and_then(|m| definitions.mode_group(component, m));
        let mission = mission_mode.and_then(|m| definitions.mode_group(component, m));
        self.report = Some(report(results, group, takeoff, mission, self.report.as_ref()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#"{"version":2,"components":{"1":{"namespace":"px4",
        "supported_protocols":["health_and_arming_check"],
        "navigation_mode_groups":{"groups":{"0":[65536],"3":[67371008],"17":[33816576]}},
        "enums":{
          "health_component_t":{"type":"uint64_t","is_bitfield":true,"entries":{"2":{"name":"gps","description":"GPS"},"4":{"name":"baro","description":"Barometer"}}},
          "navigation_mode_group_t":{"type":"uint32_t","is_bitfield":true,"entries":{"1":{"name":"manual","description":"Manual"},"8":{"name":"mission","description":"Mission"},"131072":{"name":"takeoff","description":"Takeoff"}}}},
        "event_groups":{
          "arming_check":{"events":{
            "1":{"name":"arming_check_summary","type":"summary","message":"Arming checks","arguments":[{"name":"chunk_idx","type":"uint8_t"},{"name":"error","type":"health_component_t"},{"name":"warning","type":"health_component_t"},{"name":"can_arm","type":"navigation_mode_group_t"},{"name":"can_run","type":"navigation_mode_group_t"}]},
            "2":{"name":"check_gps","message":"GPS PDOP {3:.1} too high","description":"<profile name=\"dev\">Set <param>EKF2_GPS_CHECK</param>.\n</profile><profile name=\"normal\">hidden</profile>","arguments":[{"name":"modes","type":"navigation_mode_group_t"},{"name":"health_component_index","type":"uint8_t"},{"name":"pdop","type":"float"}]}}},
          "health":{"events":{
            "3":{"name":"health_summary","type":"summary","message":"Health","arguments":[{"name":"chunk_idx","type":"uint8_t"},{"name":"is_present","type":"health_component_t"},{"name":"error","type":"health_component_t"},{"name":"warning","type":"health_component_t"}]}}}}}}}"#;

    fn bytes(parts: &[&[u8]]) -> [u8; ARGUMENT_BYTES] {
        let flat: Vec<u8> = parts.concat();
        std::array::from_fn(|i| flat.get(i).copied().unwrap_or(0))
    }

    #[test]
    fn a_px4_arming_report_reads_as_libevents_builds_it() {
        let definitions = parse(METADATA).unwrap();
        assert!(definitions.supports_checks(1));
        assert_eq!(definitions.mode_group(1, 65536), Some(0));
        assert_eq!(definitions.mode_group(1, 33816576), Some(17));
        let summary = bytes(&[&[0], &2u64.to_le_bytes(), &0u64.to_le_bytes(), &(1u32 | 131072).to_le_bytes(), &(1u32 | 8 | 131072).to_le_bytes()]);
        let check = bytes(&[&(1u32 | 8).to_le_bytes(), &[1], &2.25f32.to_le_bytes()]);
        let health = bytes(&[&[0], &6u64.to_le_bytes(), &2u64.to_le_bytes(), &0u64.to_le_bytes()]);
        let mut checks = Checks::default();
        assert!(!checks.handle(&definitions.parse_event(0x0100_0001, summary, 0).unwrap()));
        assert!(!checks.handle(&definitions.parse_event(0x0100_0002, check, 3).unwrap()));
        assert!(checks.handle(&definitions.parse_event(0x0100_0003, health, 0).unwrap()));
        let results = checks.results.unwrap();
        let mode = report(&results, 0, Some(17), Some(3), None);
        assert!(mode.can_arm && mode.can_takeoff && !mode.can_start_mission, "the takeoff group is bit 17, mission bit 3");
        assert_eq!(mode.gps_state, "red");
        assert_eq!(mode.problems, vec![("GPS PDOP 2.2 too high".to_string(), "Set <a href=\"param://EKF2_GPS_CHECK\">EKF2_GPS_CHECK</a>.".to_string(), "error")]);
        assert!(mode.has_warnings_or_errors);
        assert!(report(&results, 17, None, None, None).problems.is_empty(), "the check only names manual and mission");
    }

    #[test]
    fn a_check_out_of_order_resets_the_chunk() {
        let definitions = parse(METADATA).unwrap();
        let mut checks = Checks::default();
        let check = bytes(&[&1u32.to_le_bytes(), &[1], &1.0f32.to_le_bytes()]);
        assert!(!checks.handle(&definitions.parse_event(0x0100_0002, check, 3).unwrap()));
        assert!(checks.results.is_none());
    }

    #[test]
    fn a_gap_asks_for_the_missing_event_and_duplicates_are_dropped() {
        let mut receiver = Receiver::default();
        let event = |sequence| Incoming { sequence, timestamp_ms: 1000, for_us: true };
        assert_eq!(receiver.on_event(&event(10), 0), (true, None));
        assert_eq!(receiver.on_event(&event(10), 0), (false, None));
        assert_eq!(receiver.on_event(&event(13), 0), (false, Some(11)));
        assert_eq!(receiver.on_tick(50), None);
        assert_eq!(receiver.on_tick(100), Some(11));
        assert_eq!(receiver.on_event(&event(11), 120), (true, None));
        assert_eq!(receiver.on_current(15, false, 130), Some(12));
        assert_eq!(compare(u16::MAX, 0), Sequence::Newer);
    }

    #[test]
    fn text_outside_tags_is_html_escaped_and_unknown_tags_drop_their_content() {
        let definitions = parse(r#"{"version":1,"components":{"1":{"namespace":"px4","event_groups":{"default":{"events":{"5":{"name":"x","message":"a < b <b>gone</b> \\{1\\} <a href=\"https://x\">doc</a>"}}}}}}}"#).unwrap();
        let parsed = definitions.parse_event(0x0100_0005, [0; ARGUMENT_BYTES], 6).unwrap();
        assert_eq!(parsed.message(), "a &lt; b  {1} <a href=\"https://x\">doc</a>");
    }
}
