use std::collections::BTreeMap;

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
    initial_complete: bool,
    waiting_for_default: bool,
    ready: bool,
    missing: bool,
    total_count: usize,
}

impl Params {
    pub fn new(default_component: u8, px4: bool) -> Self {
        Params { default_component, px4, ..Default::default() }
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn value(&self, component: u8, name: &str) -> Option<ParamValue> {
        self.facts.get(&component)?.get(name).copied()
    }

    pub fn names(&self, component: u8) -> Vec<String> {
        self.facts.get(&component).map(|m| m.keys().cloned().collect()).unwrap_or_default()
    }

    pub fn start(&mut self) -> Vec<Action> {
        self.refresh_all(ALL_COMPONENTS)
    }

    pub fn refresh_all(&mut self, component: u8) -> Vec<Action> {
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
        if index == NO_INDEX && name != HASH_CHECK && self.initial_timer_active {
            return Vec::new();
        }
        self.initial_timer_active = false;
        if self.px4 && name == HASH_CHECK {
            return vec![Action::StopInitialTimer];
        }
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
        let waiting = self.waiting_index.get_mut(&component).unwrap();
        if waiting.remove(&index).is_some() {
            self.batch_queue.retain(|i| *i != index);
            actions.extend(self.fill_batch_queue(false));
        }
        self.waiting_read.entry(component).or_default().remove(name);
        self.waiting_write.entry(component).or_default().remove(name);
        self.pending_write.entry(component).or_default().remove(name);
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
        actions.extend(self.check_initial_load_complete());
        actions
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
                        actions.push(Action::WriteFailed { component: *component, name });
                    }
                }
            }
        }
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
        self.initial_retry += 1;
        if self.initial_retry <= MAX_INITIAL_REQUEST_LIST_RETRY {
            let mut actions = self.refresh_all(ALL_COMPONENTS);
            actions.push(Action::StartInitialTimer);
            actions
        } else {
            self.initial_timer_active = false;
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
        vec![Action::Ready { missing: self.missing }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deliver(params: &mut Params, names: &[&str], skip: &[u16]) -> Vec<Action> {
        names
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip.contains(&(*i as u16)))
            .flat_map(|(i, name)| params.on_param_value(1, name, names.len() as u16, i as u16, ParamValue::I32(i as i32)))
            .collect()
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
        assert_eq!(px4.on_param_value(1, "_HASH_CHECK", 3, NO_INDEX, ParamValue::U32(7)), vec![Action::StopInitialTimer]);
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
        assert_eq!(params.value(1, "A"), Some(ParamValue::I32(0)));
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
    }
}
