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
    read_through: usize,
}

pub fn admitted(px4: bool, checks_supported: bool, severity: u8, text: &str) -> Option<String> {
    let prearm = text.starts_with("PreArm") || (text.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("preflight")) && severity >= SEVERITY_CRITICAL);
    match (px4 && text.ends_with('\t'), prearm && checks_supported) {
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
        self.record_html(component, severity, escaped(&text), time);
    }

    pub fn record_html(&mut self, component: u8, severity: u8, text: String, time: String) {
        let active = *self.active.get_or_insert(component);
        self.multi |= component != active;
        self.items.push(Logged { component: self.multi.then_some(component), severity, time, text });
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.read_through = 0;
    }

    pub fn unread(&self) -> usize {
        self.items.len() - self.read_through
    }

    pub fn reset_all(&mut self) {
        self.read_through = self.items.len();
    }

    pub fn unread_type(&self) -> &'static str {
        let unread = &self.items[self.read_through..];
        match () {
            _ if unread.iter().any(|m| kind(m.severity) == Kind::Error) => "error",
            _ if unread.iter().any(|m| kind(m.severity) == Kind::Warning) => "warning",
            _ if !unread.is_empty() => "normal",
            _ => "none",
        }
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
                format!("<font style=\"{style}\">[{} {component}] {}: {}</font><br/>", m.time, severity_label(m.severity), m.text.replace('\n', "<br/>"))
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
    fn unread_counts_and_the_worst_unread_kind_reset_when_read_as_status_text_handler_does() {
        let mut log = MessageLog::default();
        assert_eq!((log.unread(), log.unread_type()), (0, "none"));
        log.record(1, 6, "a".into(), "t".into());
        log.record(1, 3, "b".into(), "t".into());
        assert_eq!((log.unread(), log.unread_type()), (2, "error"));
        log.reset_all();
        assert_eq!((log.unread(), log.unread_type(), log.count()), (0, "none", 2), "reading keeps the messages and clears only what is new");
        log.record(1, 4, "c".into(), "t".into());
        assert_eq!((log.unread(), log.unread_type()), (1, "warning"));
        log.clear();
        assert_eq!(log.unread(), 0);
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
