use std::collections::{BTreeMap, BTreeSet};

pub const ALL_COMPONENTS: u8 = 0;
pub const NO_INDEX: u16 = 65535;
pub const INITIAL_REQUEST_TIMEOUT_MS: u64 = 5000;
pub const WAITING_TIMEOUT_MS: u64 = 3000;
const MAX_INITIAL_REQUEST_LIST_RETRY: u32 = 4;
const MAX_INITIAL_LOAD_RETRY_SINGLE_PARAM: u32 = 5;
const MAX_READ_WRITE_RETRY: u32 = 5;
const MAX_BATCH_SIZE: usize = 10;
const HASH_CHECK: &str = "_HASH_CHECK";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    F32(f32),
    Unsupported(u8),
}

impl ParamValue {
    pub fn decode(param_type: u8, bits: f32) -> Option<ParamValue> {
        let raw = bits.to_le_bytes();
        Some(match param_type {
            1 => ParamValue::U8(raw[0]),
            2 => ParamValue::I8(raw[0] as i8),
            3 => ParamValue::U16(u16::from_le_bytes([raw[0], raw[1]])),
            4 => ParamValue::I16(i16::from_le_bytes([raw[0], raw[1]])),
            5 => ParamValue::U32(u32::from_le_bytes(raw)),
            6 => ParamValue::I32(i32::from_le_bytes(raw)),
            9 => ParamValue::F32(bits),
            7 | 8 | 10 => ParamValue::Unsupported(param_type),
            _ => return None,
        })
    }

    pub fn decode_cast(param_type: u8, value: f32) -> Option<ParamValue> {
        Some(match param_type {
            1 => ParamValue::U8(value as u8),
            2 => ParamValue::I8(value as i8),
            3 => ParamValue::U16(value as u16),
            4 => ParamValue::I16(value as i16),
            5 => ParamValue::U32(value as u32),
            6 => ParamValue::I32(value as i32),
            _ => return ParamValue::decode(param_type, value),
        })
    }

    pub fn encode_cast(self) -> f32 {
        match self {
            ParamValue::F32(_) | ParamValue::Unsupported(_) => self.encode(),
            other => other.as_f64() as f32,
        }
    }

    pub fn from_f64(param_type: u8, value: f64) -> Option<ParamValue> {
        if !value.is_finite() {
            return None;
        }
        let whole = (value.fract() == 0.0).then(|| value as i64);
        Some(match param_type {
            1 => ParamValue::U8(u8::try_from(whole?).ok()?),
            2 => ParamValue::I8(i8::try_from(whole?).ok()?),
            3 => ParamValue::U16(u16::try_from(whole?).ok()?),
            4 => ParamValue::I16(i16::try_from(whole?).ok()?),
            5 => ParamValue::U32(u32::try_from(whole?).ok()?),
            6 => ParamValue::I32(i32::try_from(whole?).ok()?),
            9 => ParamValue::F32((value as f32).is_finite().then_some(value as f32)?),
            _ => return None,
        })
    }


    pub fn param_type(self) -> u8 {
        match self {
            ParamValue::U8(_) => 1,
            ParamValue::I8(_) => 2,
            ParamValue::U16(_) => 3,
            ParamValue::I16(_) => 4,
            ParamValue::U32(_) => 5,
            ParamValue::I32(_) => 6,
            ParamValue::F32(_) => 9,
            ParamValue::Unsupported(t) => t,
        }
    }

    pub fn encode(self) -> f32 {
        let bytes = match self {
            ParamValue::U8(v) => [v, 0, 0, 0],
            ParamValue::I8(v) => [v as u8, 0, 0, 0],
            ParamValue::U16(v) => { let b = v.to_le_bytes(); [b[0], b[1], 0, 0] }
            ParamValue::I16(v) => { let b = v.to_le_bytes(); [b[0], b[1], 0, 0] }
            ParamValue::U32(v) => v.to_le_bytes(),
            ParamValue::I32(v) => v.to_le_bytes(),
            ParamValue::F32(v) => v.to_le_bytes(),
            ParamValue::Unsupported(_) => [0, 0, 0, 0],
        };
        f32::from_le_bytes(bytes)
    }

    fn size(self) -> usize {
        match self {
            ParamValue::U8(_) | ParamValue::I8(_) => 1,
            ParamValue::U16(_) | ParamValue::I16(_) => 2,
            _ => 4,
        }
    }

    pub fn as_f64(self) -> f64 {
        match self {
            ParamValue::U8(v) => v as f64,
            ParamValue::I8(v) => v as f64,
            ParamValue::U16(v) => v as f64,
            ParamValue::I16(v) => v as f64,
            ParamValue::U32(v) => v as f64,
            ParamValue::I32(v) => v as f64,
            ParamValue::F32(v) => v as f64,
            ParamValue::Unsupported(_) => f64::NAN,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    RequestList { component: u8 },
    ReadByIndex { component: u8, index: u16 },
    ReadByName { component: u8, name: String },
    Set { component: u8, name: String, value: ParamValue },
    StartInitialTimer,
    StopInitialTimer,
    StartWaitingTimer,
    StopWaitingTimer,
    Progress(f64),
    Added { component: u8, name: String },
    Ready { missing: bool },
    SaveCache { component: u8 },
    ReadFailed { component: u8, name: String },
    WriteFailed { component: u8, name: String },
    NoResponse,
}

#[derive(Debug, Default)]
pub struct Params {
    pub default_component: u8,
    pub px4: bool,
    facts: BTreeMap<u8, BTreeMap<String, ParamValue>>,
    counts: BTreeMap<u8, u16>,
    waiting_index: BTreeMap<u8, BTreeMap<u16, u32>>,
    waiting_read: BTreeMap<u8, BTreeMap<String, u32>>,
    waiting_write: BTreeMap<u8, BTreeMap<String, u32>>,
    pending_write: BTreeMap<u8, BTreeMap<String, ParamValue>>,
    write_batch: usize,
    read_batch: usize,
    failed_index: BTreeMap<u8, Vec<u16>>,
    batch_queue: Vec<u16>,
    batch_active: bool,
    initial_timer_active: bool,
    initial_retry: u32,
    unanswered: bool,
    initial_complete: bool,
    waiting_for_default: bool,
    ready: bool,
    missing: bool,
    total_count: usize,
    cache: Option<(BTreeMap<String, ParamValue>, BTreeSet<String>)>,
    hash_check_pending: bool,
}

fn acknowledges(written: ParamValue, echoed: ParamValue) -> bool {
    match (written, echoed) {
        (ParamValue::F32(a), ParamValue::F32(b)) => (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() * 100_000.0 <= a.abs().min(b.abs()),
        _ => written == echoed,
    }
}

pub fn cache_crc(cache: &BTreeMap<String, ParamValue>, volatile: &BTreeSet<String>) -> u32 {
    cache
        .iter()
        .filter(|(name, _)| !volatile.contains(*name))
        .fold(0, |crc, (name, value)| crate::bootloader::crc32(&value.encode().to_le_bytes()[..value.size()], crate::bootloader::crc32(name.as_bytes(), crc)))
}

fn hash_of(value: ParamValue) -> u32 {
    u32::from_le_bytes(value.encode().to_le_bytes())
}

pub const PACK_URI: &str = "@PARAM/param.pck?withdefaults=1";
const PACK_MAGIC: u16 = 0x671B;
const PACK_MAGIC_WITH_DEFAULTS: u16 = 0x671C;

pub struct PackEntry {
    pub name: String,
    pub value: ParamValue,
    pub default: Option<ParamValue>,
}

fn pack_value(ptype: u8, bytes: &[u8]) -> Option<(ParamValue, usize)> {
    Some(match ptype {
        1 => (ParamValue::I8(*bytes.first()? as i8), 1),
        2 => (ParamValue::I16(i16::from_le_bytes(bytes.get(..2)?.try_into().ok()?)), 2),
        3 => (ParamValue::I32(i32::from_le_bytes(bytes.get(..4)?.try_into().ok()?)), 4),
        4 => (ParamValue::F32(f32::from_le_bytes(bytes.get(..4)?.try_into().ok()?)), 4),
        _ => return None,
    })
}

pub fn parse_pack(bytes: &[u8]) -> Result<Vec<PackEntry>, String> {
    let word = |at: usize| bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or("the parameter file has no header");
    let (magic, count, total) = (word(0)?, word(2)?, word(4)?);
    if magic != PACK_MAGIC && magic != PACK_MAGIC_WITH_DEFAULTS {
        return Err("the parameter file does not start with the magic number".to_string());
    }
    if count != total {
        return Err(format!("the parameter file holds {count} of {total} parameters"));
    }
    let mut entries = Vec::new();
    let mut at = 6;
    let mut previous = String::new();
    loop {
        while bytes.get(at) == Some(&0) {
            at += 1;
        }
        let Some(&first) = bytes.get(at) else { break };
        let (ptype, with_default) = (first & 0x0F, (first >> 4) & 0x01 == 1);
        let lengths = *bytes.get(at + 1).ok_or("the parameter file ends inside a header")?;
        let (name_len, common_len) = (usize::from((lengths >> 4) & 0x0F) + 1, usize::from(lengths & 0x0F));
        if name_len + common_len > 16 || common_len > previous.len() {
            return Err("the parameter file has a malformed name".to_string());
        }
        let tail = bytes.get(at + 2..at + 2 + name_len).ok_or("the parameter file ends inside a name")?;
        let name = format!("{}{}", &previous[..common_len], String::from_utf8_lossy(tail));
        at += 2 + name_len;
        let (value, width) = pack_value(ptype, &bytes[at..]).ok_or_else(|| format!("{name} has type {ptype}, which the parameter file cannot carry"))?;
        at += width;
        let default = match with_default {
            true => {
                let (default, width) = pack_value(ptype, &bytes[at..]).ok_or_else(|| format!("{name} ends before its default"))?;
                at += width;
                Some(default)
            }
            false => None,
        };
        previous = name.clone();
        entries.push(PackEntry { name, value, default });
    }
    match entries.len() == usize::from(count) {
        true => Ok(entries),
        false => Err(format!("the parameter file announced {count} parameters and held {}", entries.len())),
    }
}

impl Params {
    pub fn load_pack(&mut self, component: u8, entries: &[PackEntry]) -> Vec<Action> {
        let values: BTreeMap<String, ParamValue> = entries.iter().map(|e| (e.name.clone(), e.value)).collect();
        self.counts.insert(component, u16::try_from(values.len()).unwrap_or(u16::MAX));
        self.total_count += values.len();
        self.facts.insert(component, values);
        self.initial_timer_active = false;
        let added = entries.iter().map(|e| Action::Added { component, name: e.name.clone() });
        added.chain([Action::StopInitialTimer, Action::Progress(1.0)]).chain(self.check_initial_load_complete()).collect()
    }

    pub fn new(default_component: u8, px4: bool) -> Self {
        Params { default_component, px4, ..Default::default() }
    }

    pub fn unanswered(&self) -> bool {
        self.unanswered
    }

    pub fn components(&self) -> Vec<u8> {
        self.facts.iter().filter(|(_, facts)| !facts.is_empty()).map(|(component, _)| *component).collect()
    }

    pub fn component_label(&self, component: u8) -> String {
        if self.facts.len() > 1 { format!("comp: {component}") } else { String::new() }
    }

    pub fn pending_writes(&self) -> bool {
        self.pending_write.values().any(|names| !names.is_empty())
    }

    pub fn missing(&self) -> bool {
        self.missing
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn writing(&self, component: u8, name: &str) -> bool {
        self.waiting_write.get(&component).is_some_and(|waiting| waiting.contains_key(name))
    }

    pub fn value(&self, component: u8, name: &str) -> Option<ParamValue> {
        self.facts.get(&component)?.get(name).copied()
    }

    pub fn entries(&self, component: u8) -> impl Iterator<Item = (&String, &ParamValue)> {
        self.facts.get(&component).into_iter().flatten()
    }

    pub fn names(&self, component: u8) -> Vec<String> {
        self.facts.get(&component).map(|m| m.keys().cloned().collect()).unwrap_or_default()
    }

    pub fn use_cache(&mut self, cache: BTreeMap<String, ParamValue>, volatile: BTreeSet<String>) {
        self.cache = Some((cache, volatile)).filter(|(c, _)| !c.is_empty());
    }

    pub fn start(&mut self) -> Vec<Action> {
        match self.px4 && self.cache.is_some() && !self.initial_complete {
            true => {
                self.hash_check_pending = true;
                self.initial_timer_active = true;
                vec![Action::StartInitialTimer, Action::ReadByName { component: self.default_component, name: HASH_CHECK.to_string() }]
            }
            false => self.refresh_all(ALL_COMPONENTS),
        }
    }

    fn hash_answered(&mut self, component: u8, value: ParamValue) -> Vec<Action> {
        self.hash_check_pending = false;
        let (cache, volatile) = self.cache.clone().unwrap_or_default();
        let crc = cache_crc(&cache, &volatile);
        match component == self.default_component && crc == hash_of(value) {
            true => {
                let entries: Vec<PackEntry> = cache.iter().map(|(name, value)| PackEntry { name: name.clone(), value: *value, default: None }).collect();
                self.load_pack(component, &entries)
                    .into_iter()
                    .chain([Action::Set { component, name: HASH_CHECK.to_string(), value: ParamValue::U32(crc) }])
                    .collect()
            }
            false => self.refresh_all(ALL_COMPONENTS),
        }
    }

    pub fn refresh_all(&mut self, component: u8) -> Vec<Action> {
        self.unanswered = false;
        let timer = (!self.initial_complete).then(|| {
            self.initial_timer_active = true;
            Action::StartInitialTimer
        });
        for (cid, count) in &self.counts {
            if component == ALL_COMPONENTS || component == *cid {
                self.waiting_index.insert(*cid, (0..*count).map(|i| (i, 0)).collect());
            }
        }
        timer.into_iter().chain([Action::RequestList { component }]).collect()
    }

    pub fn refresh(&mut self, component: u8, name: &str) -> Vec<Action> {
        let waiting = self.waiting_read.entry(component).or_default();
        if waiting.insert(name.to_string(), 0).is_none() {
            self.read_batch += 1;
        }
        vec![self.progress(), Action::StartWaitingTimer, Action::ReadByName { component, name: name.to_string() }]
    }

    pub fn write(&mut self, component: u8, name: &str, value: ParamValue) -> Vec<Action> {
        if self.waiting_write.entry(component).or_default().insert(name.to_string(), 0).is_none() {
            self.write_batch += 1;
        }
        self.pending_write.entry(component).or_default().insert(name.to_string(), value);
        if let Some(known) = self.facts.get_mut(&component).and_then(|facts| facts.get_mut(name)) {
            *known = value;
        }
        vec![self.progress(), Action::StartWaitingTimer, Action::Set { component, name: name.to_string(), value }]
    }

    fn progress(&mut self) -> Action {
        let waiting_index: usize = self.waiting_index.values().map(BTreeMap::len).sum();
        let waiting_write: usize = self.waiting_write.values().map(BTreeMap::len).sum();
        let waiting_read: usize = self.waiting_read.values().map(BTreeMap::len).sum();
        let fraction = |batch: usize, waiting: usize| batch.saturating_sub(waiting).max(1) as f64 / (batch + 1) as f64;
        if waiting_index > 0 {
            return Action::Progress((self.total_count.saturating_sub(waiting_index)) as f64 / self.total_count.max(1) as f64);
        }
        if waiting_write > 0 {
            return Action::Progress(fraction(self.write_batch, waiting_write));
        }
        self.write_batch = 0;
        if waiting_read > 0 {
            return Action::Progress(fraction(self.read_batch, waiting_read));
        }
        self.read_batch = 0;
        Action::Progress(0.0)
    }

    pub fn on_param_value(&mut self, component: u8, name: &str, count: u16, index: u16, value: ParamValue) -> Vec<Action> {
        if self.px4 && name == HASH_CHECK {
            return match self.hash_check_pending {
                true => self.hash_answered(component, value),
                false => Vec::new(),
            };
        }
        if index == NO_INDEX && self.initial_timer_active {
            return Vec::new();
        }
        self.initial_timer_active = false;
        let mut actions = vec![Action::StopInitialTimer, Action::StopWaitingTimer];
        if !self.counts.contains_key(&component) {
            self.counts.insert(component, count);
            self.total_count += count as usize;
        }
        if !self.waiting_index.contains_key(&component) {
            self.waiting_index.insert(component, (0..count).map(|i| (i, 0)).collect());
            self.waiting_read.entry(component).or_default();
            self.waiting_write.entry(component).or_default();
        }
        let reads_before = self.index_reads_waiting(component);
        let waiting = self.waiting_index.get_mut(&component).unwrap();
        if waiting.remove(&index).is_some() {
            self.batch_queue.retain(|i| *i != index);
            actions.extend(self.fill_batch_queue(false));
        }
        self.waiting_read.entry(component).or_default().remove(name);
        let acknowledged = self.pending_write.get(&component).and_then(|written| written.get(name)).is_none_or(|written| acknowledges(*written, value));
        if acknowledged {
            self.waiting_write.entry(component).or_default().remove(name);
            self.pending_write.entry(component).or_default().remove(name);
        }
        let total_waiting: usize = self.waiting_index.values().map(BTreeMap::len).sum::<usize>() + self.waiting_read.values().map(BTreeMap::len).sum::<usize>() + self.waiting_write.values().map(BTreeMap::len).sum::<usize>();
        if total_waiting > 0 || !self.facts.contains_key(&self.default_component) {
            actions.push(Action::StartWaitingTimer);
        }
        actions.push(self.progress());
        let facts = self.facts.entry(component).or_default();
        if !facts.contains_key(name) {
            actions.push(Action::Added { component, name: name.to_string() });
        }
        facts.insert(name.to_string(), value);
        let refreshed = self.px4 && self.initial_complete && reads_before > 0 && self.index_reads_waiting(component) == 0;
        actions.extend(self.check_initial_load_complete());
        actions.extend(refreshed.then_some(Action::SaveCache { component }));
        actions
    }

    fn index_reads_waiting(&self, component: u8) -> usize {
        self.waiting_index.get(&component).map_or(0, BTreeMap::len)
    }

    fn fill_batch_queue(&mut self, timeout: bool) -> Vec<Action> {
        if !self.batch_active {
            return Vec::new();
        }
        if timeout {
            self.batch_queue.clear();
        }
        let mut actions = Vec::new();
        for (component, waiting) in self.waiting_index.iter_mut() {
            for index in waiting.keys().copied().collect::<Vec<_>>() {
                if self.batch_queue.contains(&index) {
                    continue;
                }
                if self.batch_queue.len() > MAX_BATCH_SIZE {
                    break;
                }
                let retries = waiting.entry(index).or_insert(0);
                *retries += 1;
                if *retries > MAX_INITIAL_LOAD_RETRY_SINGLE_PARAM {
                    self.failed_index.entry(*component).or_default().push(index);
                    waiting.remove(&index);
                } else {
                    self.batch_queue.push(index);
                    actions.push(Action::ReadByIndex { component: *component, index });
                }
            }
        }
        actions
    }

    pub fn on_waiting_timeout(&mut self) -> Vec<Action> {
        self.batch_active = true;
        let mut actions = self.fill_batch_queue(true);
        let mut requested = !actions.is_empty();
        if !requested && !self.waiting_for_default && !self.facts.contains_key(&self.default_component) {
            self.waiting_for_default = true;
            return vec![Action::StartWaitingTimer];
        }
        self.waiting_for_default = false;
        actions.extend(self.check_initial_load_complete());
        let mut batch = 0usize;
        let mut failed: Vec<(u8, String)> = Vec::new();
        if !requested {
            'writes: for (component, waiting) in self.waiting_write.iter_mut() {
                for name in waiting.keys().cloned().collect::<Vec<_>>() {
                    requested = true;
                    let retries = waiting.entry(name.clone()).or_insert(0);
                    *retries += 1;
                    if *retries <= MAX_READ_WRITE_RETRY {
                        let value = self.pending_write.get(component).and_then(|f| f.get(&name)).copied();
                        if let Some(value) = value {
                            actions.push(Action::Set { component: *component, name: name.clone(), value });
                        }
                        batch += 1;
                        if batch > MAX_BATCH_SIZE {
                            break 'writes;
                        }
                    } else {
                        waiting.remove(&name);
                        failed.push((*component, name.clone()));
                        actions.push(Action::WriteFailed { component: *component, name });
                    }
                }
            }
        }
        failed.iter().for_each(|(component, name)| {
            self.pending_write.entry(*component).or_default().remove(name);
        });
        let refreshes: Vec<Action> = failed.iter().flat_map(|(component, name)| self.refresh(*component, name)).collect();
        actions.extend(refreshes);
        if !requested {
            'reads: for (component, waiting) in self.waiting_read.iter_mut() {
                for name in waiting.keys().cloned().collect::<Vec<_>>() {
                    requested = true;
                    let retries = waiting.entry(name.clone()).or_insert(0);
                    *retries += 1;
                    if *retries <= MAX_READ_WRITE_RETRY {
                        actions.push(Action::ReadByName { component: *component, name });
                        batch += 1;
                        if batch > MAX_BATCH_SIZE {
                            break 'reads;
                        }
                    } else {
                        waiting.remove(&name);
                        actions.push(Action::ReadFailed { component: *component, name });
                    }
                }
            }
        }
        if requested {
            actions.push(Action::StartWaitingTimer);
        }
        actions
    }

    pub fn on_initial_timeout(&mut self) -> Vec<Action> {
        if self.hash_check_pending {
            self.hash_check_pending = false;
            return self.refresh_all(ALL_COMPONENTS);
        }
        self.initial_retry += 1;
        if self.initial_retry <= MAX_INITIAL_REQUEST_LIST_RETRY {
            let mut actions = self.refresh_all(ALL_COMPONENTS);
            actions.push(Action::StartInitialTimer);
            actions
        } else {
            self.initial_timer_active = false;
            self.unanswered = true;
            vec![Action::NoResponse]
        }
    }

    fn check_initial_load_complete(&mut self) -> Vec<Action> {
        if self.initial_complete || self.waiting_index.values().any(|w| !w.is_empty()) || !self.facts.contains_key(&self.default_component) {
            return Vec::new();
        }
        self.initial_complete = true;
        self.missing = self.failed_index.values().any(|f| !f.is_empty());
        self.ready = true;
        let save = (self.px4 && !self.missing).then_some(Action::SaveCache { component: self.default_component });
        std::iter::once(Action::Ready { missing: self.missing }).chain(save).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached() -> BTreeMap<String, ParamValue> {
        [("SYS_AUTOSTART", ParamValue::I32(4001)), ("MPC_XY_VEL_MAX", ParamValue::F32(12.0)), ("COM_FLIGHT_UUID", ParamValue::I32(77))]
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect()
    }

    fn volatile() -> BTreeSet<String> {
        ["COM_FLIGHT_UUID".to_string()].into_iter().collect()
    }

    #[test]
    fn the_cache_hash_is_px4s_crc_over_name_then_value_skipping_volatile_ones() {
        let expected = [("MPC_XY_VEL_MAX", 12.0f32.to_le_bytes()), ("SYS_AUTOSTART", 4001i32.to_le_bytes())]
            .iter()
            .fold(0, |crc, (name, bytes)| crate::bootloader::crc32(bytes, crate::bootloader::crc32(name.as_bytes(), crc)));
        assert_eq!(cache_crc(&cached(), &volatile()), expected, "ParameterManager::_tryCacheHashLoad: sorted names, each name then its value bytes, volatile parameters left out");
        let mut changed = cached();
        changed.insert("COM_FLIGHT_UUID".to_string(), ParamValue::I32(78));
        assert_eq!(cache_crc(&changed, &volatile()), expected, "a volatile value changing does not break the cache");
    }

    #[test]
    fn a_matching_hash_loads_the_cache_instead_of_the_full_list() {
        let mut px4 = Params::new(1, true);
        px4.use_cache(cached(), volatile());
        assert_eq!(px4.start(), vec![Action::StartInitialTimer, Action::ReadByName { component: 1, name: HASH_CHECK.to_string() }]);
        let crc = cache_crc(&cached(), &volatile());
        let loaded = px4.on_param_value(1, HASH_CHECK, 1, NO_INDEX, ParamValue::U32(crc));
        assert!(loaded.contains(&Action::Ready { missing: false }));
        assert!(loaded.contains(&Action::Set { component: 1, name: HASH_CHECK.to_string(), value: ParamValue::U32(crc) }), "the hash is sent back so PX4 stops streaming");
        assert!(!loaded.iter().any(|a| matches!(a, Action::RequestList { .. })));
        assert_eq!(px4.value(1, "SYS_AUTOSTART"), Some(ParamValue::I32(4001)));
    }

    #[test]
    fn a_refresh_that_reads_everything_back_rewrites_the_cache() {
        let mut px4 = Params::new(1, true);
        px4.start();
        px4.on_param_value(1, "A", 2, 0, ParamValue::I32(1));
        assert!(px4.on_param_value(1, "B", 2, 1, ParamValue::I32(2)).contains(&Action::SaveCache { component: 1 }), "the first complete load is cached");
        px4.refresh_all(1);
        assert!(!px4.on_param_value(1, "A", 2, 0, ParamValue::I32(3)).contains(&Action::SaveCache { component: 1 }));
        assert!(px4.on_param_value(1, "B", 2, 1, ParamValue::I32(4)).contains(&Action::SaveCache { component: 1 }), "ParameterManager writes the cache whenever the waiting reads drain to zero");
        px4.refresh(1, "A");
        assert!(!px4.on_param_value(1, "A", 2, NO_INDEX, ParamValue::I32(5)).contains(&Action::SaveCache { component: 1 }), "a single by-name read is not an index refresh, so it does not rewrite the file");
        let mut ardupilot = Params::new(1, false);
        ardupilot.start();
        ardupilot.on_param_value(1, "A", 1, 0, ParamValue::I32(1));
        ardupilot.refresh_all(1);
        assert!(!ardupilot.on_param_value(1, "A", 1, 0, ParamValue::I32(2)).contains(&Action::SaveCache { component: 1 }), "the cache is PX4 only");
    }

    #[test]
    fn a_stale_cache_or_a_silent_vehicle_falls_back_to_the_full_list() {
        let mut stale = Params::new(1, true);
        stale.use_cache(cached(), volatile());
        stale.start();
        assert!(stale.on_param_value(1, HASH_CHECK, 1, NO_INDEX, ParamValue::U32(1)).contains(&Action::RequestList { component: ALL_COMPONENTS }));
        let mut silent = Params::new(1, true);
        silent.use_cache(cached(), volatile());
        silent.start();
        assert!(silent.on_initial_timeout().contains(&Action::RequestList { component: ALL_COMPONENTS }), "the hash check timing out goes on to PARAM_REQUEST_LIST");
        assert_eq!(silent.on_param_value(1, HASH_CHECK, 1, NO_INDEX, ParamValue::U32(cache_crc(&cached(), &volatile()))), Vec::new(), "a late answer leaves the list request and its retry timer alone");
        let mut stranger = Params::new(1, true);
        stranger.use_cache(cached(), volatile());
        stranger.start();
        assert!(stranger.on_param_value(154, HASH_CHECK, 1, NO_INDEX, ParamValue::U32(cache_crc(&cached(), &volatile()))).contains(&Action::RequestList { component: ALL_COMPONENTS }), "the cache belongs to the autopilot, not whichever component answered");
        let mut fresh = Params::new(1, true);
        assert!(fresh.start().contains(&Action::RequestList { component: ALL_COMPONENTS }), "with no cache there is nothing to check");
        let mut ardupilot = Params::new(1, false);
        ardupilot.use_cache(cached(), volatile());
        assert!(ardupilot.start().contains(&Action::RequestList { component: ALL_COMPONENTS }), "only PX4 answers _HASH_CHECK");
    }

    #[test]
    fn a_parameter_pack_shares_name_prefixes_and_carries_defaults() {
        let mut pack = vec![0x1C, 0x67, 3, 0, 3, 0];
        pack.extend([0x13, (6 << 4) | 0, b'R', b'T', b'L', b'_', b'A', b'L', b'T']);
        pack.extend(1500i32.to_le_bytes());
        pack.extend(1500i32.to_le_bytes());
        pack.extend([0, 0]);
        pack.extend([0x04, (4 << 4) | 4, b'S', b'P', b'E', b'E', b'D']);
        pack.extend(2.5f32.to_le_bytes());
        pack.extend([0x01, 3 << 4, b'A', b'R', b'M', b'1']);
        pack.push(0xFF);
        let entries = parse_pack(&pack).unwrap();
        let summary: Vec<(String, ParamValue, Option<ParamValue>)> = entries.into_iter().map(|e| (e.name, e.value, e.default)).collect();
        assert_eq!(summary, vec![
            ("RTL_ALT".to_string(), ParamValue::I32(1500), Some(ParamValue::I32(1500))),
            ("RTL_SPEED".to_string(), ParamValue::F32(2.5), None),
            ("ARM1".to_string(), ParamValue::I8(-1), None),
        ], "padding is skipped and a name keeps the first common_len characters of the one before");
        assert!(parse_pack(&pack[..pack.len() - 1]).is_err(), "a truncated file is refused");
        let mut params = Params::new(1, false);
        let ready = params.load_pack(1, &parse_pack(&pack).unwrap());
        assert!(params.ready() && ready.contains(&Action::Ready { missing: false }));
        assert_eq!(params.value(1, "RTL_SPEED"), Some(ParamValue::F32(2.5)));
    }

    fn deliver(params: &mut Params, names: &[&str], skip: &[u16]) -> Vec<Action> {
        names
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip.contains(&(*i as u16)))
            .flat_map(|(i, name)| params.on_param_value(1, name, names.len() as u16, i as u16, ParamValue::I32(i as i32)))
            .collect()
    }

    #[test]
    fn a_write_the_vehicle_answers_with_another_value_is_retried_then_failed_and_reread() {
        let mut params = Params::new(1, false);
        deliver(&mut params, &["RTL_ALT", "RTL_SPEED"], &[]);
        params.write(1, "RTL_ALT", ParamValue::I32(3000));
        params.on_param_value(1, "RTL_ALT", 2, 0, ParamValue::I32(0));
        assert!(params.writing(1, "RTL_ALT"), "ParameterManager's ack check needs the echoed value to match what was written");
        let failed: Vec<Action> = (0..=MAX_READ_WRITE_RETRY).flat_map(|_| params.on_waiting_timeout()).collect();
        assert!(failed.contains(&Action::WriteFailed { component: 1, name: "RTL_ALT".into() }));
        assert!(failed.contains(&Action::ReadByName { component: 1, name: "RTL_ALT".into() }), "a failed write refreshes the parameter from the vehicle");
        assert!(acknowledges(ParamValue::F32(0.1), ParamValue::F32(0.100_000_1)), "floats compare fuzzily, as QGC::fuzzyCompare does");
        params.write(1, "RTL_SPEED", ParamValue::I32(5));
        params.on_param_value(1, "RTL_SPEED", 2, 1, ParamValue::I32(5));
        assert!(!params.writing(1, "RTL_SPEED"));
    }

    #[test]
    fn ardupilot_casts_every_value_to_the_float() {
        assert_eq!(ParamValue::decode_cast(6, 1.0), Some(ParamValue::I32(1)));
        assert_eq!(ParamValue::decode_cast(2, -3.0), Some(ParamValue::I8(-3)));
        assert_eq!(ParamValue::decode_cast(9, 0.25), Some(ParamValue::F32(0.25)));
        assert_eq!(ParamValue::I32(1500).encode_cast(), 1500.0);
        assert_eq!(ParamValue::F32(0.25).encode_cast(), 0.25);
        assert_ne!(ParamValue::decode(6, 1.0), Some(ParamValue::I32(1)), "the spec's byte-wise reading of the same frame is a different number");
    }

    #[test]
    fn values_travel_as_bit_patterns_inside_the_float() {
        for value in [ParamValue::I32(-7), ParamValue::U8(200), ParamValue::I16(-300), ParamValue::U32(4_000_000_000), ParamValue::F32(1.5)] {
            assert_eq!(ParamValue::decode(value.param_type(), value.encode()), Some(value));
        }
        assert_eq!(ParamValue::decode(10, 1.0), Some(ParamValue::Unsupported(10)));
        assert_eq!(ParamValue::decode(11, 1.0), None);
        assert!(ParamValue::Unsupported(8).as_f64().is_nan());
        assert_eq!(ParamValue::I8(-1).as_f64(), -1.0);
    }

    #[test]
    fn a_complete_list_makes_the_parameters_ready() {
        let mut params = Params::new(1, false);
        assert_eq!(params.start(), vec![Action::StartInitialTimer, Action::RequestList { component: 0 }]);
        let actions = deliver(&mut params, &["A", "B", "C"], &[]);
        assert_eq!(actions.iter().filter(|a| matches!(a, Action::Added { .. })).count(), 3);
        assert_eq!(actions.last(), Some(&Action::Ready { missing: false }));
        assert!(params.ready());
        assert_eq!(params.value(1, "C"), Some(ParamValue::I32(2)));
        assert!(!actions[..actions.len() - 2].contains(&Action::Ready { missing: false }));
    }

    #[test]
    fn a_write_before_any_parameter_does_not_make_the_load_ready() {
        let mut params = Params::new(1, false);
        params.start();
        params.write(1, "X", ParamValue::I32(1));
        assert_eq!(params.on_waiting_timeout(), vec![Action::StartWaitingTimer]);
        let actions = params.on_waiting_timeout();
        assert!(!actions.iter().any(|a| matches!(a, Action::Ready { .. })));
        assert!(actions.contains(&Action::Set { component: 1, name: "X".into(), value: ParamValue::I32(1) }));
        let mut px4 = Params::new(1, true);
        px4.start();
        assert_eq!(px4.on_param_value(1, "_HASH_CHECK", 3, NO_INDEX, ParamValue::U32(7)), Vec::new());
        let mut fresh = Params::new(1, false);
        assert!(fresh.refresh(5, "Y").contains(&Action::ReadByName { component: 5, name: "Y".into() }));
    }

    #[test]
    fn an_unrequested_value_before_the_list_answer_is_ignored() {
        let mut params = Params::new(1, false);
        params.start();
        assert!(params.on_param_value(1, "STRAY", 3, NO_INDEX, ParamValue::U8(1)).is_empty());
        assert_eq!(params.value(1, "STRAY"), None);
    }

    #[test]
    fn a_missing_index_is_re_requested_then_given_up_on() {
        let mut params = Params::new(1, false);
        params.start();
        let actions = deliver(&mut params, &["A", "B", "C"], &[1]);
        assert!(actions.contains(&Action::StartWaitingTimer) && !actions.iter().any(|a| matches!(a, Action::Ready { .. })));
        let rereads: Vec<Vec<Action>> = (0..5).map(|_| params.on_waiting_timeout()).collect();
        assert!(rereads.iter().all(|a| a.contains(&Action::ReadByIndex { component: 1, index: 1 }) && a.contains(&Action::StartWaitingTimer)));
        let given_up = params.on_waiting_timeout();
        assert!(given_up.contains(&Action::Ready { missing: true }));
        assert!(!given_up.iter().any(|a| matches!(a, Action::ReadByIndex { .. })));
        let late = params.on_param_value(1, "B", 3, 1, ParamValue::I32(1));
        assert_eq!(params.value(1, "B"), Some(ParamValue::I32(1)));
        assert!(!late.iter().any(|a| matches!(a, Action::Ready { .. })));
    }

    #[test]
    fn a_write_is_resent_until_acknowledged_or_failed() {
        let mut params = Params::new(1, false);
        params.start();
        deliver(&mut params, &["A"], &[]);
        let sent = params.write(1, "A", ParamValue::I32(9));
        assert_eq!(sent, vec![Action::Progress(0.5), Action::StartWaitingTimer, Action::Set { component: 1, name: "A".into(), value: ParamValue::I32(9) }]);
        assert_eq!(params.value(1, "A"), Some(ParamValue::I32(9)), "Fact::setRawValue changes the local value at once, before the vehicle acknowledges it");
        let resend = params.on_waiting_timeout();
        assert!(resend.contains(&Action::Set { component: 1, name: "A".into(), value: ParamValue::I32(9) }));
        let ack = params.on_param_value(1, "A", 1, 0, ParamValue::I32(9));
        assert!(!ack.contains(&Action::StartWaitingTimer));
        params.write(1, "A", ParamValue::I32(10));
        let outcomes: Vec<Vec<Action>> = (0..6).map(|_| params.on_waiting_timeout()).collect();
        assert!(outcomes[4].iter().any(|a| matches!(a, Action::Set { .. })));
        assert!(outcomes[5].contains(&Action::WriteFailed { component: 1, name: "A".into() }));
    }

    #[test]
    fn a_named_refresh_is_retried_and_the_initial_list_request_gives_up_after_four_retries() {
        let mut params = Params::new(1, false);
        params.start();
        deliver(&mut params, &["A"], &[]);
        assert!(params.refresh(1, "A").contains(&Action::ReadByName { component: 1, name: "A".into() }));
        let retried: Vec<Vec<Action>> = (0..6).map(|_| params.on_waiting_timeout()).collect();
        assert!(retried[0].contains(&Action::ReadByName { component: 1, name: "A".into() }));
        assert!(retried[5].contains(&Action::ReadFailed { component: 1, name: "A".into() }));
        let mut silent = Params::new(1, true);
        silent.start();
        let retries: Vec<Vec<Action>> = (0..5).map(|_| silent.on_initial_timeout()).collect();
        assert!(retries[..4].iter().all(|a| a.contains(&Action::RequestList { component: 0 })));
        assert_eq!(retries[4], vec![Action::NoResponse]);
        assert!(silent.unanswered(), "requestUnanswered is what the setup page shows once the retries run out");
        silent.refresh_all(ALL_COMPONENTS);
        assert!(!silent.unanswered(), "a new download clears it, as _startParameterDownload does");
    }
}
