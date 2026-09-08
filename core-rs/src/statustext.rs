use std::collections::BTreeMap;

pub const TEXT_FIELD_LEN: usize = 50;
const MISSING_CHUNK: &str = " ... ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Normal,
    Warning,
    Error,
}

pub fn kind(severity: u8) -> Kind {
    match severity {
        0..=3 => Kind::Error,
        4 | 5 => Kind::Warning,
        _ => Kind::Normal,
    }
}

pub fn severity_label(severity: u8) -> &'static str {
    match severity {
        0 => "EMERGENCY",
        1 => "ALERT",
        2 => "Critical",
        3 => "Error",
        4 => "Warning",
        5 => "Notice",
        6 => "Info",
        7 => "Debug",
        _ => "",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusText {
    pub component: u8,
    pub severity: u8,
    pub text: String,
}

#[derive(Debug, Default)]
struct Pending {
    id: u16,
    severity: u8,
    chunks: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Handler {
    pub messages: Vec<StatusText>,
    pending: BTreeMap<u8, Pending>,
    active_component: Option<u8>,
    pub multi_component: bool,
}

pub fn message_text(raw: &[u8]) -> (String, bool) {
    let bytes: Vec<u8> = raw.iter().take(TEXT_FIELD_LEN).take_while(|b| **b != 0).copied().collect();
    let includes_terminator = bytes.len() < TEXT_FIELD_LEN;
    (String::from_utf8_lossy(&bytes).into_owned(), includes_terminator)
}

impl Handler {
    pub fn receive(&mut self, component: u8, severity: u8, id: u16, chunk_seq: u8, raw_text: &[u8]) -> Option<StatusText> {
        let (text, includes_terminator) = message_text(raw_text);
        let stale = self.pending.get(&component).map(|p| p.id != id).unwrap_or(false);
        let flushed = stale.then(|| self.complete(component, true)).flatten();
        match id {
            0 => {
                self.pending.insert(component, Pending { id: 0, severity, chunks: vec![text] });
            }
            _ => match self.pending.get_mut(&component) {
                Some(pending) => {
                    let missing = (chunk_seq as usize).saturating_sub(pending.chunks.len());
                    pending.chunks.extend(std::iter::repeat_n(String::new(), missing));
                    pending.chunks.push(text);
                }
                None => {
                    self.pending.insert(component, Pending { id, severity, chunks: vec![text] });
                }
            },
        }
        let completed = (id == 0 || includes_terminator).then(|| self.complete(component, false)).flatten();
        debug_assert!(flushed.is_none() || completed.is_some() || id != 0);
        completed.or(flushed)
    }

    pub fn expire_pending(&mut self) -> Vec<StatusText> {
        let components: Vec<u8> = self.pending.keys().copied().collect();
        components.into_iter().filter_map(|c| self.complete(c, true)).collect()
    }

    fn complete(&mut self, component: u8, missing_tail: bool) -> Option<StatusText> {
        let pending = self.pending.remove(&component)?;
        let chunks = pending.chunks.into_iter().chain(missing_tail.then(String::new));
        let text: String = chunks.map(|c| if c.is_empty() { MISSING_CHUNK.to_string() } else { c }).collect();
        Some(self.record(component, pending.severity, text))
    }

    pub fn record(&mut self, component: u8, severity: u8, text: String) -> StatusText {
        let active = *self.active_component.get_or_insert(component);
        self.multi_component |= component != active;
        let message = StatusText { component, severity, text };
        self.messages.push(message.clone());
        message
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        self.messages.iter().fold((0, 0, 0), |(n, w, e), m| match kind(m.severity) {
            Kind::Normal => (n + 1, w, e),
            Kind::Warning => (n, w + 1, e),
            Kind::Error => (n, w, e + 1),
        })
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn formatted(&self, clock: &str, message: &StatusText) -> String {
        let style = match kind(message.severity) {
            Kind::Error => "<#E>",
            Kind::Warning => "<#I>",
            Kind::Normal => "<#N>",
        };
        let component = if self.multi_component { format!("COMP:{}", message.component) } else { String::new() };
        let html = message.text.replace('\n', "<br/>");
        format!("<font style=\"{style}\">[{clock} {component}] {}: {html}</font><br/>", severity_label(message.severity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded(text: &str) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.resize(TEXT_FIELD_LEN, 0);
        bytes
    }

    #[test]
    fn plain_messages_count_by_severity_like_status_text_handler_test() {
        let mut handler = Handler::default();
        assert!(handler.receive(1, 6, 0, 0, &padded("StatusTextHandlerTestInfo")).is_some());
        assert_eq!(handler.counts(), (1, 0, 0));
        handler.receive(1, 4, 0, 0, &padded("StatusTextHandlerTestWarning"));
        assert_eq!(handler.counts(), (1, 1, 0));
        assert_eq!(handler.messages.len(), 2);
        assert_eq!(handler.messages[1].text, "StatusTextHandlerTestWarning");
        handler.clear();
        assert_eq!(handler.counts(), (0, 0, 0));
        assert!(!handler.multi_component);
    }

    #[test]
    fn chunks_reassemble_in_order_and_a_missing_chunk_is_marked() {
        let mut handler = Handler::default();
        let full: String = "x".repeat(TEXT_FIELD_LEN);
        assert!(handler.receive(1, 6, 7, 0, full.as_bytes()).is_none());
        let done = handler.receive(1, 6, 7, 1, &padded("tail")).unwrap();
        assert_eq!(done.text, format!("{full}tail"));
        assert!(handler.receive(1, 6, 8, 0, full.as_bytes()).is_none());
        let gapped = handler.receive(1, 6, 8, 2, &padded("end")).unwrap();
        assert_eq!(gapped.text, format!("{full} ... end"));
    }

    #[test]
    fn a_new_id_flushes_a_stale_sequence_and_expiry_marks_the_missing_tail() {
        let mut handler = Handler::default();
        let full: String = "y".repeat(TEXT_FIELD_LEN);
        assert!(handler.receive(1, 4, 3, 0, full.as_bytes()).is_none());
        let flushed = handler.receive(1, 6, 0, 0, &padded("next")).unwrap();
        assert_eq!(handler.messages.len(), 2);
        assert_eq!(handler.messages[0].text, format!("{full} ... "));
        assert_eq!(flushed.text, "next");
        assert!(handler.receive(2, 6, 5, 0, full.as_bytes()).is_none());
        let expired = handler.expire_pending();
        assert_eq!(expired.len(), 1);
        assert!(expired[0].text.ends_with(" ... "));
        assert!(handler.multi_component);
    }

    #[test]
    fn the_formatted_line_matches_the_qt_format() {
        let mut handler = Handler::default();
        let message = handler.record(1, 4, "PreArm: Compass\nnot healthy".into());
        assert_eq!(handler.formatted("12:34:56.789", &message), "<font style=\"<#I>\">[12:34:56.789 ] Warning: PreArm: Compass<br/>not healthy</font><br/>");
        handler.record(2, 3, "boom".into());
        let error = handler.messages[1].clone();
        assert_eq!(handler.formatted("1", &error), "<font style=\"<#E>\">[1 COMP:2] Error: boom</font><br/>");
        let (text, terminated) = message_text(&padded("short"));
        assert_eq!((text.as_str(), terminated), ("short", true));
    }
}
