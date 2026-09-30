use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde_json::{Value, json};

const SYSTEM_TIME: u32 = 2;
const DEBUG_VECT: u32 = 250;
const NAMED_VALUE_FLOAT: u32 = 251;
const NAMED_VALUE_INT: u32 = 252;
const DEBUG: u32 = 254;
const PAYLOAD_ROOM: usize = 280;
const V2_HEADER: usize = 10;
const V1_HEADER: usize = 6;

#[derive(Debug, Clone)]
struct Field {
    name: String,
    kind: String,
    array: usize,
    offset: usize,
}

#[derive(Debug, Clone)]
struct Info {
    name: String,
    fields: Vec<Field>,
    instance: Option<String>,
}

static TABLE: LazyLock<BTreeMap<u32, Info>> = LazyLock::new(|| {
    let parsed: BTreeMap<String, Value> = serde_json::from_str(include_str!("mavlinkfields.json")).unwrap_or_default();
    parsed
        .into_iter()
        .filter_map(|(id, entry)| {
            let fields = entry.get(1)?.as_array()?.iter().filter_map(|f| {
                Some(Field { name: f.get(0)?.as_str()?.to_string(), kind: f.get(1)?.as_str()?.to_string(), array: usize::try_from(f.get(2)?.as_u64()?).ok()?, offset: usize::try_from(f.get(3)?.as_u64()?).ok()? })
            });
            Some((id.parse().ok()?, Info { name: entry.get(0)?.as_str()?.to_string(), fields: fields.collect(), instance: entry.get(2).and_then(Value::as_str).map(str::to_string) }))
        })
        .collect()
});

pub fn payload(raw: &[u8], v2: bool) -> Vec<u8> {
    let header = if v2 { V2_HEADER } else { V1_HEADER };
    let length = usize::from(raw.get(1).copied().unwrap_or(0));
    let carried = raw.get(header..(header + length).min(raw.len())).unwrap_or(&[]);
    carried.iter().copied().chain(std::iter::repeat(0)).take(PAYLOAD_ROOM).collect()
}

fn size_of(kind: &str) -> usize {
    match kind {
        "char" | "uint8_t" | "int8_t" => 1,
        "uint16_t" | "int16_t" => 2,
        "uint32_t" | "int32_t" | "float" => 4,
        _ => 8,
    }
}

fn bytes<const N: usize>(payload: &[u8], at: usize) -> [u8; N] {
    payload.get(at..at + N).and_then(|b| b.try_into().ok()).unwrap_or([0; N])
}

fn latin1_until_nul(bytes: &[u8]) -> String {
    bytes.iter().take_while(|b| **b != 0).map(|b| char::from(*b)).collect()
}

fn integer_text(kind: &str, payload: &[u8], at: usize) -> Option<(String, f64)> {
    Some(match kind {
        "uint8_t" => (payload.get(at)?.to_string(), f64::from(*payload.get(at)?)),
        "int8_t" => ((*payload.get(at)? as i8).to_string(), f64::from(*payload.get(at)? as i8)),
        "uint16_t" => { let n = u16::from_le_bytes(bytes(payload, at)); (n.to_string(), f64::from(n)) }
        "int16_t" => { let n = i16::from_le_bytes(bytes(payload, at)); (n.to_string(), f64::from(n)) }
        "uint32_t" => { let n = u32::from_le_bytes(bytes(payload, at)); (n.to_string(), f64::from(n)) }
        "int32_t" => { let n = i32::from_le_bytes(bytes(payload, at)); (n.to_string(), f64::from(n)) }
        "uint64_t" => { let n = u64::from_le_bytes(bytes(payload, at)); (n.to_string(), n as f64) }
        "int64_t" => { let n = i64::from_le_bytes(bytes(payload, at)); (n.to_string(), n as f64) }
        _ => return None,
    })
}

pub fn instance_value(msgid: u32, payload: &[u8]) -> String {
    let Some(info) = TABLE.get(&msgid) else { return String::new() };
    let named = |field: &str| info.fields.iter().find(|f| f.name == field);
    let wanted = match (info.instance.as_deref(), msgid) {
        (Some(name), _) => named(name),
        (None, NAMED_VALUE_FLOAT | NAMED_VALUE_INT | DEBUG_VECT) => named("name"),
        (None, DEBUG) => named("ind"),
        _ => None,
    };
    let Some(field) = wanted else { return String::new() };
    match field.kind.as_str() {
        "char" if field.array > 0 => latin1_until_nul(payload.get(field.offset..field.offset + field.array).unwrap_or(&[])).trim().to_string(),
        "char" => payload.get(field.offset).map(|b| char::from(*b).to_string()).unwrap_or_default(),
        kind => integer_text(kind, payload, field.offset).map(|(text, _)| text).unwrap_or_default(),
    }
}

pub fn display_name(msgid: u32, instance: &str) -> String {
    let name = TABLE.get(&msgid).map(|i| i.name.clone()).unwrap_or_default();
    match instance.is_empty() {
        true => name,
        false => format!("{name} [{instance}]"),
    }
}

pub fn known(msgid: u32) -> bool {
    TABLE.contains_key(&msgid)
}

pub fn g_format(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    let precision = precision.max(1);
    let scientific = format!("{:.*e}", precision - 1, value);
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let trim = |text: String| match text.contains('.') {
        true => text.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => text,
    };
    match exponent >= -4 && exponent < precision as i32 {
        true => trim(format!("{:.*}", usize::try_from(precision as i32 - 1 - exponent).unwrap_or(0), value)),
        false => format!("{}e{}{:02}", trim(mantissa.to_string()), if exponent < 0 { '-' } else { '+' }, exponent.abs()),
    }
}

fn utc(ms: i64, pattern: &str) -> String {
    chrono::DateTime::from_timestamp_millis(ms).map(|t| t.format(pattern).to_string()).unwrap_or_default()
}

pub fn fields(msgid: u32, payload: &[u8]) -> Vec<Value> {
    let Some(info) = TABLE.get(&msgid) else { return Vec::new() };
    info.fields
        .iter()
        .flat_map(|field| {
            let elements: Vec<Option<usize>> = match field.array > 0 && field.kind != "char" {
                true => (0..field.array).map(Some).collect(),
                false => vec![None],
            };
            elements.into_iter().map(move |element| {
                let name = element.map_or_else(|| field.name.clone(), |j| format!("{}[{j}]", field.name));
                let at = field.offset + element.unwrap_or(0) * size_of(&field.kind);
                let value = match (field.kind.as_str(), element) {
                    ("char", _) if field.array > 0 => latin1_until_nul(payload.get(field.offset..field.offset + field.array - 1).unwrap_or(&[])),
                    ("char", _) => payload.get(at).map(|b| char::from(*b).to_string()).unwrap_or_default(),
                    ("float", _) => g_format(f64::from(f32::from_le_bytes(bytes(payload, at))), 10),
                    ("double", _) => g_format(f64::from_le_bytes(bytes(payload, at)), 15),
                    ("uint32_t", None) if msgid == SYSTEM_TIME => utc(i64::from(u32::from_le_bytes(bytes(payload, at))), "%H:%M:%S"),
                    ("uint64_t", None) if msgid == SYSTEM_TIME => utc(i64::try_from(u64::from_le_bytes(bytes(payload, at)) / 1000).unwrap_or(0), "%Y %m %d %H:%M:%S"),
                    (kind, _) => integer_text(kind, payload, at).map(|(text, _)| text).unwrap_or_default(),
                };
                json!({ "name": name, "type": field.kind, "value": value })
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: u32,
    pub comp_id: u8,
    pub instance: String,
    pub name: String,
    pub count: u64,
    last_count: u64,
    pub actual_rate_hz: f64,
    pub target_rate_hz: i32,
    pub selected: bool,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct System {
    pub id: u8,
    pub messages: Vec<Message>,
    pub selected: usize,
}

impl System {
    fn append(&mut self, mut message: Message) {
        let held = self.messages.get(self.selected).map(|m| (m.id, m.comp_id, m.instance.clone()));
        message.selected |= self.messages.is_empty();
        let at = self.messages.partition_point(|m| m.name < message.name);
        self.messages.insert(at, message);
        if let Some(index) = held.and_then(|(id, comp, instance)| self.messages.iter().position(|m| m.id == id && m.comp_id == comp && m.instance == instance)) {
            self.selected = index;
        }
    }

    pub fn select(&mut self, index: usize) {
        if index >= self.messages.len() {
            return;
        }
        self.selected = index;
        self.messages.iter_mut().enumerate().for_each(|(i, m)| m.selected = i == index);
    }
}

#[derive(Debug, Default)]
pub struct Inspector {
    pub systems: Vec<System>,
    pub active: Option<u8>,
    vehicles: Vec<u8>,
    followed: Option<u8>,
    refreshed_ms: Option<u64>,
}

pub const RATE_PERIOD_MS: u64 = 1000;
pub const INSPECTOR_CHANGED: &str = "core.inspector@changed";
const HEARTBEAT: u32 = 0;
const RADIO_STATUS: u32 = 109;

static INSPECTOR: std::sync::Mutex<Inspector> = std::sync::Mutex::new(Inspector { systems: Vec::new(), active: None, vehicles: Vec::new(), followed: None, refreshed_ms: None });

pub fn lock() -> std::sync::MutexGuard<'static, Inspector> {
    INSPECTOR.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn message_json(system: &System, message: &Message) -> Value {
    json!({
        "kind": "object", "class": "QGCMAVLinkMessage", "objectName": "", "children": ["fields"], "facts": [],
        "id": message.id, "compId": message.comp_id, "sysId": system.id, "name": message.name, "instanceValue": message.instance,
        "count": message.count, "actualRateHz": message.actual_rate_hz, "targetRateHz": message.target_rate_hz,
        "selected": message.selected, "fieldSelected": false,
    })
}

fn list_json(elements: Vec<Value>) -> Value {
    json!({ "kind": "object", "class": "QmlObjectListModel", "objectName": "", "children": [], "facts": [], "dirty": false, "count": elements.len(), "elements": elements })
}

fn field_json(message: &Message, field: &Value) -> Value {
    let name = field["name"].as_str().unwrap_or_default();
    json!({
        "kind": "object", "class": "QGCMAVLinkMessageField", "objectName": "", "children": [], "facts": [], "chartIndex": 0, "series": Value::Null,
        "name": name, "type": field["type"], "label": format!("{}: {name}", message.name), "selectable": field["type"] != "char",
        "value": if message.selected { field["value"].clone() } else { json!("") },
    })
}

impl Inspector {
    pub fn receive(&mut self, sysid: u8, compid: u8, msgid: u32, payload: Vec<u8>) {
        if !known(msgid) {
            return;
        }
        let instance = instance_value(msgid, &payload);
        let index = match self.systems.iter().position(|s| s.id == sysid) {
            Some(index) => index,
            None => {
                self.systems.push(System { id: sysid, ..System::default() });
                self.active = self.active.or(Some(sysid));
                self.systems.len() - 1
            }
        };
        let system = &mut self.systems[index];
        match system.messages.iter_mut().find(|m| m.id == msgid && m.comp_id == compid && m.instance == instance) {
            Some(message) => {
                message.count += 1;
                message.payload = payload;
            }
            None => {
                let name = display_name(msgid, &instance);
                system.append(Message { id: msgid, comp_id: compid, instance, name, count: 1, last_count: 0, actual_rate_hz: 0.0, target_rate_hz: 0, selected: false, payload });
            }
        }
    }

    pub fn observe(&mut self, frame: &crate::transport::Frame, vehicles: &[u8], active_vehicle: Option<u8>) {
        vehicles.iter().filter(|id| !self.vehicles.contains(id)).copied().collect::<Vec<_>>().into_iter().for_each(|id| self.vehicle_added(id));
        self.vehicles.iter().filter(|id| !vehicles.contains(id)).copied().collect::<Vec<_>>().into_iter().for_each(|id| self.vehicle_removed(id));
        self.vehicles = vehicles.to_vec();
        if active_vehicle != self.followed {
            self.followed = active_vehicle;
            self.follow_vehicle(active_vehicle);
        }
        let msgid = mavlink::Message::message_id(&frame.message);
        if !frame.v2 && msgid != HEARTBEAT && msgid != RADIO_STATUS {
            return;
        }
        if let mavlink::dialects::ardupilotmega::MavMessage::MESSAGE_INTERVAL(interval) = &frame.message {
            self.note_interval(frame.header.system_id, frame.header.component_id, u32::from(interval.message_id), interval.interval_us);
        }
        self.receive(frame.header.system_id, frame.header.component_id, msgid, payload(&frame.raw, frame.v2));
    }

    pub fn tick(&mut self, now_ms: u64) -> bool {
        let due = self.refreshed_ms.is_none_or(|last| now_ms.saturating_sub(last) >= RATE_PERIOD_MS);
        if due {
            self.refreshed_ms = Some(now_ms);
            self.refresh_rates();
        }
        due
    }

    pub fn get(&self, path: &str) -> Option<Value> {
        let rest = path.strip_prefix("mavlinkInspector.activeSystem")?;
        let system = self.active_system();
        let value = |v: Value| Some(json!({ "kind": "value", "value": v }));
        match rest {
            "" => Some(system.map_or_else(|| json!({ "kind": "null" }), |s| json!({ "kind": "object", "class": "QGCMAVLinkSystem", "objectName": "", "children": ["messages"], "facts": [], "id": s.id, "selected": s.selected }))),
            ".id" => value(json!(system.map(|s| s.id))),
            ".selected" => value(json!(system.map_or(0, |s| s.selected))),
            ".messages" => Some(list_json(system.map(|s| s.messages.iter().map(|m| message_json(s, m)).collect()).unwrap_or_default())),
            _ => {
                let (index, tail) = rest.strip_prefix(".messages.")?.split_once('.').map_or((rest.strip_prefix(".messages.")?, ""), |(i, t)| (i, t));
                let index: usize = index.parse().ok()?;
                let not_found = || Some(json!({ "found": false, "kind": "value", "value": null }));
                let Some((system, message)) = system.and_then(|s| s.messages.get(index).map(|m| (s, m))) else { return not_found() };
                match tail {
                    "" => Some(message_json(system, message)),
                    "fields" => Some(list_json(fields(message.id, &message.payload).iter().map(|f| field_json(message, f)).collect())),
                    field => message_json(system, message).get(field).cloned().map(|v| json!({ "kind": "value", "value": v })),
                }
            }
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        self.active_system_mut().is_some_and(|system| {
            let fits = index < system.messages.len();
            system.select(index);
            fits
        })
    }

    pub fn selected_target(&self) -> Option<(u8, u8, u32)> {
        let system = self.active_system()?;
        let message = system.messages.get(system.selected)?;
        Some((system.id, message.comp_id, message.id))
    }

    pub fn refresh_rates(&mut self) {
        self.systems.iter_mut().flat_map(|s| s.messages.iter_mut()).for_each(|m| {
            m.actual_rate_hz = 0.2 * m.actual_rate_hz + 0.8 * (m.count - m.last_count) as f64;
            m.last_count = m.count;
        });
    }

    pub fn vehicle_added(&mut self, id: u8) {
        match self.systems.iter_mut().find(|s| s.id == id) {
            Some(system) => system.messages.clear(),
            None => self.systems.push(System { id, ..System::default() }),
        }
    }

    pub fn vehicle_removed(&mut self, id: u8) {
        self.systems.retain(|s| s.id != id);
        if self.active == Some(id) {
            self.active = None;
        }
    }

    pub fn follow_vehicle(&mut self, id: Option<u8>) {
        self.active = id.filter(|id| self.systems.iter().any(|s| s.id == *id));
    }

    pub fn set_active(&mut self, id: u8) {
        self.active = self.systems.iter().any(|s| s.id == id).then_some(id);
    }

    pub fn note_interval(&mut self, sysid: u8, compid: u8, msgid: u32, interval_us: i32) {
        let rate = if interval_us > 0 { (1_000_000.0 / f64::from(interval_us)) as i32 } else { interval_us };
        if let Some(message) = self.systems.iter_mut().filter(|s| s.id == sysid).flat_map(|s| s.messages.iter_mut()).find(|m| m.comp_id == compid && m.id == msgid) {
            message.target_rate_hz = rate;
        }
    }

    pub fn active_system(&self) -> Option<&System> {
        self.systems.iter().find(|s| Some(s.id) == self.active)
    }

    pub fn active_system_mut(&mut self) -> Option<&mut System> {
        let active = self.active;
        self.systems.iter_mut().find(|s| Some(s.id) == active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_format_writes_doubles_the_way_qstring_number_g_does() {
        assert_eq!(g_format(f64::from(0.1f32), 10), "0.1000000015");
        assert_eq!(g_format(-1.5, 10), "-1.5");
        assert_eq!(g_format(0.0, 10), "0");
        assert_eq!(g_format(1e20, 10), "1e+20");
        assert_eq!(g_format(1.25e-5, 10), "1.25e-05");
        assert_eq!(g_format(123456789012.0, 10), "1.23456789e+11");
        assert_eq!(g_format(-35.363262, 15), "-35.363262");
        assert_eq!(g_format(f64::NAN, 10), "nan");
    }

    fn frame(msgid: u32, payload: &[u8]) -> Vec<u8> {
        let mut raw = vec![0xFD, payload.len() as u8, 0, 0, 0, 1, 1, msgid as u8, (msgid >> 8) as u8, (msgid >> 16) as u8];
        raw.extend_from_slice(payload);
        raw.extend([0, 0]);
        raw
    }

    #[test]
    fn a_heartbeat_lists_its_fields_in_xml_order_read_at_their_wire_offsets() {
        let beat = frame(0, &[0x04, 0, 0, 0, 2, 3, 81, 3]);
        let payload = payload(&beat, true);
        let listed: Vec<(String, String)> = fields(0, &payload).iter().map(|f| (f["name"].as_str().unwrap().to_string(), f["value"].as_str().unwrap().to_string())).collect();
        assert_eq!(listed, vec![("type".into(), "2".into()), ("autopilot".into(), "3".into()), ("base_mode".into(), "81".into()), ("custom_mode".into(), "4".into()), ("system_status".into(), "3".into()), ("mavlink_version".into(), "0".into())], "the trailing zero the sender trimmed reads back as zero");
    }

    #[test]
    fn text_arrays_stop_at_the_terminator_and_numeric_arrays_expand_by_element() {
        let mut status = vec![6u8];
        status.extend(b"ArduCopter V4.5.7");
        let listed = fields(253, &payload(&frame(253, &status), true));
        assert_eq!(listed[1]["value"], "ArduCopter V4.5.7");
        assert_eq!(listed[1]["type"], "char");
        let mut quaternion = vec![0u8; 8];
        [1.0f32, 0.5, 0.25, 0.125].iter().for_each(|v| quaternion.extend(v.to_le_bytes()));
        let listed = fields(61, &payload(&frame(61, &quaternion), true));
        let q: Vec<(String, String)> = listed.iter().filter(|f| f["name"].as_str().unwrap_or("").starts_with("q[")).map(|f| (f["name"].as_str().unwrap().to_string(), f["value"].as_str().unwrap().to_string())).collect();
        assert_eq!(q, vec![("q[0]".into(), "1".into()), ("q[1]".into(), "0.5".into()), ("q[2]".into(), "0.25".into()), ("q[3]".into(), "0.125".into())]);
    }

    #[test]
    fn a_named_value_is_listed_once_per_name_and_named_in_its_title() {
        let mut named = vec![0u8; 4];
        named.extend(0.5f32.to_le_bytes());
        named.extend(b"batt\0\0\0\0\0\0");
        let payload = payload(&frame(251, &named), true);
        assert_eq!(instance_value(251, &payload), "batt");
        assert_eq!(display_name(251, "batt"), "NAMED_VALUE_FLOAT [batt]");
    }

    #[test]
    fn messages_sort_by_name_the_first_is_selected_and_selection_follows_its_message() {
        let mut inspector = Inspector::default();
        inspector.receive(1, 1, 30, vec![0; 280]);
        inspector.receive(1, 1, 0, vec![0; 280]);
        inspector.receive(1, 1, 30, vec![0; 280]);
        let system = inspector.active_system().unwrap();
        assert_eq!(system.messages.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["ATTITUDE", "HEARTBEAT"]);
        assert_eq!((system.selected, system.messages[0].selected, system.messages[0].count), (0, true, 2), "the first message a system sends is selected, and counts start at one");
        inspector.receive(1, 1, 1, vec![0; 280]);
        inspector.receive(1, 1, 2, vec![0; 280]);
        let system = inspector.active_system_mut().unwrap();
        system.select(3);
        inspector.receive(1, 1, 24, vec![0; 280]);
        let system = inspector.active_system().unwrap();
        assert_eq!(system.messages[system.selected].name, "SYS_STATUS", "inserting a message ahead of the selection moves the index with it");
    }

    #[test]
    fn the_rate_is_a_smoothed_count_per_second_and_a_target_rate_comes_from_message_interval() {
        let mut inspector = Inspector::default();
        (0..5).for_each(|_| inspector.receive(1, 1, 30, vec![0; 280]));
        inspector.refresh_rates();
        assert!((inspector.active_system().unwrap().messages[0].actual_rate_hz - 4.0).abs() < 1e-9, "0.2 * 0 + 0.8 * 5");
        inspector.note_interval(1, 1, 30, 100_000);
        assert_eq!(inspector.active_system().unwrap().messages[0].target_rate_hz, 10);
        inspector.note_interval(1, 1, 30, -1);
        assert_eq!(inspector.active_system().unwrap().messages[0].target_rate_hz, -1);
    }
}
