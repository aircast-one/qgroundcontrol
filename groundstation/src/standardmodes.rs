pub const PROPERTY_ADVANCED: u32 = 1;
pub const PROPERTY_NOT_USER_SELECTABLE: u32 = 2;
pub const MSG_AVAILABLE_MODES: u32 = 435;

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableMode {
    pub custom_mode: u32,
    pub properties: u32,
    pub number_modes: u8,
    pub mode_index: u8,
    pub standard_mode: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlightMode {
    pub name: String,
    pub standard_mode: u8,
    pub custom_mode: u32,
    pub can_be_set: bool,
    pub advanced: bool,
    pub fixed_wing: bool,
    pub multi_rotor: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    RequestMode(u8),
    Completed(Vec<FlightMode>),
    Failed,
}

fn standard_name(standard_mode: u8) -> Option<(&'static str, bool)> {
    match standard_mode {
        1 => Some(("Position", false)),
        2 => Some(("Orbit", true)),
        3 => Some(("Cruise", false)),
        4 => Some(("Altitude", false)),
        5 => Some(("Safe Recovery", true)),
        6 => Some(("Mission", false)),
        7 => Some(("Land", true)),
        8 => Some(("Takeoff", true)),
        _ => None,
    }
}

pub fn unique_names(modes: Vec<FlightMode>) -> Vec<FlightMode> {
    let mut named = modes;
    for i in 0..named.len() {
        let name = named[i].name.clone();
        let mut duplicates = 0;
        for later in named.iter_mut().skip(i + 1) {
            if later.name == name {
                duplicates += 1;
                later.name = format!("{name} ({duplicates})");
            }
        }
    }
    named
}

#[derive(Debug, Default)]
pub struct StandardModes {
    modes: Vec<FlightMode>,
    request_active: bool,
    want_reset: bool,
    last_seq: Option<u8>,
}

impl StandardModes {
    pub fn request(&mut self) -> Vec<Out> {
        if self.request_active {
            self.want_reset = true;
            return Vec::new();
        }
        self.modes.clear();
        self.request_mode(1)
    }

    fn request_mode(&mut self, index: u8) -> Vec<Out> {
        self.request_active = true;
        vec![Out::RequestMode(index)]
    }

    pub fn on_monitor(&mut self, seq: u8) -> Vec<Out> {
        if self.last_seq == Some(seq) {
            return Vec::new();
        }
        self.last_seq = Some(seq);
        self.request()
    }

    pub fn on_message(&mut self, accepted: bool, message: Option<&AvailableMode>) -> Vec<Out> {
        self.request_active = false;
        if self.want_reset {
            self.want_reset = false;
            return self.request();
        }
        let Some(mode) = message.filter(|_| accepted) else { return vec![Out::Failed] };
        let (name, forced_off) = standard_name(mode.standard_mode).map(|(n, off)| (n.to_string(), off)).unwrap_or((mode.name.clone(), false));
        let cannot_be_set = mode.properties & PROPERTY_NOT_USER_SELECTABLE != 0 || forced_off;
        self.modes.push(FlightMode {
            name,
            standard_mode: mode.standard_mode,
            custom_mode: mode.custom_mode,
            can_be_set: !cannot_be_set,
            advanced: mode.properties & PROPERTY_ADVANCED != 0,
            fixed_wing: true,
            multi_rotor: true,
        });
        if mode.mode_index >= mode.number_modes {
            let modes = unique_names(std::mem::take(&mut self.modes));
            self.modes = modes.clone();
            vec![Out::Completed(modes)]
        } else {
            self.request_mode(mode.mode_index + 1)
        }
    }

    pub fn modes(&self) -> &[FlightMode] {
        &self.modes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(index: u8, total: u8, standard: u8, name: &str, properties: u32, custom: u32) -> AvailableMode {
        AvailableMode { custom_mode: custom, properties, number_modes: total, mode_index: index, standard_mode: standard, name: name.into() }
    }

    #[test]
    fn modes_are_requested_one_by_one_named_by_standard_and_made_unique() {
        let mut modes = StandardModes::default();
        assert_eq!(modes.request(), vec![Out::RequestMode(1)]);
        assert_eq!(modes.on_message(true, Some(&mode(1, 4, 1, "POSCTL", 0, 10))), vec![Out::RequestMode(2)]);
        assert_eq!(modes.on_message(true, Some(&mode(2, 4, 8, "TAKEOFF", 0, 11))), vec![Out::RequestMode(3)]);
        assert_eq!(modes.on_message(true, Some(&mode(3, 4, 0, "Acro", PROPERTY_ADVANCED, 12))), vec![Out::RequestMode(4)]);
        let done = modes.on_message(true, Some(&mode(4, 4, 0, "Acro", PROPERTY_NOT_USER_SELECTABLE, 13)));
        let Out::Completed(list) = &done[0] else { panic!("not completed") };
        assert_eq!(list.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["Position", "Takeoff", "Acro", "Acro (1)"]);
        assert_eq!(list.iter().map(|m| m.can_be_set).collect::<Vec<_>>(), vec![true, false, true, false]);
        assert_eq!(list.iter().map(|m| m.advanced).collect::<Vec<_>>(), vec![false, false, true, false]);
        assert_eq!(list[1].custom_mode, 11);
    }

    #[test]
    fn a_monitor_change_restarts_the_request_and_a_reset_during_a_request_waits_for_the_answer() {
        let mut modes = StandardModes::default();
        assert_eq!(modes.on_monitor(3), vec![Out::RequestMode(1)]);
        assert!(modes.on_monitor(3).is_empty());
        assert!(modes.on_monitor(4).is_empty(), "a reset while a request is active is deferred");
        assert_eq!(modes.on_message(true, Some(&mode(1, 1, 6, "AUTO", 0, 4))), vec![Out::RequestMode(1)], "the deferred reset starts over");
        assert_eq!(modes.on_message(false, None), vec![Out::Failed]);
        assert!(modes.modes().is_empty());
    }
}
