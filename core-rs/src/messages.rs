use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
pub struct Message {
    pub index: usize,
    pub time: String,
    pub component: Option<u32>,
    pub severity: String,
    pub level: &'static str,
    pub text: String,
}

// StatusTextHandler prepends each new message, so the string arrives newest first. Every other
// list the core serves runs oldest first, and a head reading last() for the newest is right on two
// of them and wrong on the third with no way to tell. They all run the same way now.
pub fn parse(formatted: &str) -> Vec<Message> {
    formatted
        .split("</font><br/>")
        .filter(|chunk| !chunk.trim().is_empty())
        .filter_map(parse_one)
        .collect::<Vec<Message>>()
        .into_iter()
        .rev()
        .enumerate()
        .map(|(index, message)| Message { index, ..message })
        .collect()
}

pub const LEVELS: &[&str] = &["normal", "warning", "error"];

pub fn level_of(style: &str) -> &'static str {
    match (style.contains("#E"), style.contains("#I")) {
        (true, _) => "error",
        (_, true) => "warning",
        _ => "normal",
    }
}

fn parse_one(chunk: &str) -> Option<Message> {
    let (style, body) = chunk.split_once("\">").unwrap_or(("", chunk));
    let (head, rest) = body.strip_prefix('[')?.split_once(']')?;
    let (time, component_tag) = head.split_once(' ').unwrap_or((head, ""));
    let component = component_tag.strip_prefix("COMP:").and_then(|id| id.parse().ok());
    let (severity, html) = rest.trim_start().split_once(": ").unwrap_or(("", rest.trim_start()));
    Some(Message {
        index: 0,
        time: time.to_string(),
        component,
        severity: severity.to_string(),
        level: level_of(style),
        text: plain_text(html),
    })
}

fn plain_text(html: &str) -> String {
    let with_newlines = html.replace("<br/>", "\n").replace("<br>", "\n");
    html_escape::decode_html_entities(&strip_tags(&with_newlines)).trim().to_string()
}

fn strip_tags(text: &str) -> String {
    text.split('<')
        .enumerate()
        .map(|(index, part)| match index {
            0 => part,
            _ => part.split_once('>').map_or("", |(_, rest)| rest),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = concat!(
        "<font style=\"<#I>\">[12:34:56.789 ] Warning: PreArm: Compass &amp; GPS &lt;inconsistent&gt;</font><br/>",
        "<font style=\"<#N>\">[12:34:55.000 COMP:1] Info: ArduCopter V4.5.7</font><br/>",
    );

    #[test]
    fn splits_messages_and_decodes_entities() {
        let parsed = parse(TWO);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].time, "12:34:56.789");
        assert_eq!(parsed[1].component, None);
        assert_eq!(parsed[1].severity, "Warning");
        assert_eq!(parsed[1].level, "warning", "the level comes from the style token, not the translated word");
        assert_eq!(parsed[1].text, "PreArm: Compass & GPS <inconsistent>");
        assert_eq!((parsed[0].index, parsed[1].index), (0, 1));
        assert_eq!(parsed[0].component, Some(1));
        assert_eq!(parsed[0].level, "normal");
        assert_eq!(parsed[0].text, "ArduCopter V4.5.7");
        assert_eq!(parse("<font style=\"<#E>\">[1:2:3.4 ] Fehler: kaputt</font><br/>")[0].level, "error", "a localized severity word still buckets by the token");
        assert_eq!(parse("<font style=\"<#I>\">[1:2:3.4 ] Notice: heads up</font><br/>")[0].level, "warning", "a notice buckets as a warning, as the Qt handler colours it");
    }

    #[test]
    fn the_newest_message_is_last_as_it_is_in_every_other_list_the_core_serves() {
        let parsed = parse(TWO);
        assert_eq!(parsed.last().unwrap().time, "12:34:56.789", "the handler prepends, so the newest arrives first in the string and has to be moved to the end");
        assert_eq!(parsed.first().unwrap().time, "12:34:55.000");
        assert!(parsed.first().unwrap().time < parsed.last().unwrap().time, "a head taking last() for the newest is right here, on the notices and on the track, rather than right on two of three");
        assert_eq!((parsed[0].index, parsed[1].index), (0, 1), "the index counts from the oldest, so it does not jump around as messages arrive");
    }

    #[test]
    fn keeps_multiline_text_in_one_message() {
        let parsed = parse("<font style=\"c\">[1:2:3.4 ] Error: line one<br/>line two<br/><small><small>details</small></small></font><br/>");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].text, "line one\nline two\ndetails");
    }

    #[test]
    fn empty_input_is_no_messages() {
        assert!(parse("").is_empty());
    }
}
