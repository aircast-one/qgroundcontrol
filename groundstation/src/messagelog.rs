use crate::statustext::{Kind, kind, severity_label};

const SEVERITY_CRITICAL: u8 = 2;

#[derive(Debug, Clone, PartialEq)]
struct Logged {
    component: Option<u8>,
    severity: u8,
    time: String,
    text: String,
}

#[derive(Debug, Default)]
pub struct MessageLog {
    active: Option<u8>,
    multi: bool,
    items: Vec<Logged>,
}

pub fn admitted(px4: bool, events_heard: bool, severity: u8, text: &str) -> Option<String> {
    let prearm = text.starts_with("PreArm") || (text.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("preflight")) && severity >= SEVERITY_CRITICAL);
    match (px4 && text.ends_with('\t'), prearm && events_heard) {
        (false, false) => Some(text.strip_prefix('#').unwrap_or(text).to_string()),
        _ => None,
    }
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn clock_now() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

impl MessageLog {
    pub fn record(&mut self, component: u8, severity: u8, text: String, time: String) {
        let active = *self.active.get_or_insert(component);
        self.multi |= component != active;
        self.items.push(Logged { component: self.multi.then_some(component), severity, time, text });
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn formatted(&self) -> String {
        self.items
            .iter()
            .rev()
            .map(|m| {
                let style = match kind(m.severity) {
                    Kind::Error => "<#E>",
                    Kind::Warning => "<#I>",
                    Kind::Normal => "<#N>",
                };
                let component = m.component.map(|c| format!("COMP:{c}")).unwrap_or_default();
                format!("<font style=\"{style}\">[{} {component}] {}: {}</font><br/>", m.time, severity_label(m.severity), escaped(&m.text).replace('\n', "<br/>"))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_drops_px4_event_duplicates_and_prearm_text_a_health_report_supersedes() {
        assert_eq!(admitted(true, false, 6, "Takeoff detected\t"), None, "PX4 sends event-backed messages twice, the text copy ending in a tab");
        assert_eq!(admitted(false, false, 6, "Takeoff detected\t").as_deref(), Some("Takeoff detected\t"));
        assert_eq!(admitted(false, true, 2, "PreArm: RC not calibrated"), None);
        assert_eq!(admitted(true, true, 3, "Preflight Fail: baro"), None);
        assert_eq!(admitted(true, true, 1, "Preflight Fail: baro").as_deref(), Some("Preflight Fail: baro"), "PX4's preflight rule only covers critical and milder, as Vehicle compares severity >= CRITICAL");
        assert_eq!(admitted(false, false, 2, "PreArm: RC not calibrated").as_deref(), Some("PreArm: RC not calibrated"), "without a health report the text is the only place the operator learns it");
        assert_eq!(admitted(false, false, 6, "#Spoken").as_deref(), Some("Spoken"), "a leading # asks to be read aloud and is not part of the message");
    }

    #[test]
    fn the_log_reads_as_qt_formats_it_newest_first_with_a_component_tag_once_two_speak() {
        let mut log = MessageLog::default();
        log.record(1, 6, "ArduCopter V4.5 <SITL> & \"x\"".to_string(), "12:00:00.001".to_string());
        log.record(1, 4, "two\nlines".to_string(), "12:00:01.002".to_string());
        log.record(100, 3, "camera".to_string(), "12:00:02.003".to_string());
        assert_eq!(
            log.formatted(),
            concat!(
                "<font style=\"<#E>\">[12:00:02.003 COMP:100] Error: camera</font><br/>",
                "<font style=\"<#I>\">[12:00:01.002 ] Warning: two<br/>lines</font><br/>",
                "<font style=\"<#N>\">[12:00:00.001 ] Info: ArduCopter V4.5 &lt;SITL&gt; &amp; &quot;x&quot;</font><br/>",
            )
        );
        log.clear();
        assert_eq!(log.formatted(), "");
    }
}
