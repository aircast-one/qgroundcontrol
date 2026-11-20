use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
pub struct Message {
    pub index: usize,
    pub time: String,
    pub component: Option<u32>,
    pub severity: String,
    pub level: &'static str,
    pub text: String,
    #[serde(rename = "toldElsewhere")]
    pub told_elsewhere: bool,
    pub resolved: bool,
}

// StatusTextHandler prepends each new message, so the string arrives newest first. Every other
// list the core serves runs oldest first, and a head reading last() for the newest is right on two
// of them and wrong on the third with no way to tell. They all run the same way now.
pub fn parse(formatted: &str) -> Vec<Message> {
    let oldest_first: Vec<Message> = formatted
        .split("</font><br/>")
        .filter(|chunk| !chunk.trim().is_empty())
        .filter_map(parse_one)
        .collect::<Vec<Message>>()
        .into_iter()
        .rev()
        .collect();
    let cleared: Vec<Option<String>> = oldest_first
        .iter()
        .map(|message| cleared_subject(&message.text))
        .collect();
    oldest_first
        .into_iter()
        .enumerate()
        .map(|(index, message)| {
            let resolved = cleared[index + 1..].iter().flatten().any(|subject| {
                cleared[index].is_none() && message.text.to_lowercase().starts_with(subject)
            });
            Message {
                index,
                level: match cleared[index] {
                    Some(_) => "normal",
                    None => message.level,
                },
                resolved,
                ..message
            }
        })
        .collect()
}

const CLEARING_WORDS: &[&str] = &["cleared", "regained", "resolved"];

pub fn cleared_subject(text: &str) -> Option<String> {
    let lower = text.trim().trim_end_matches('.').to_lowercase();
    CLEARING_WORDS
        .iter()
        .find_map(|word| lower.strip_suffix(word))
        .or_else(|| {
            lower
                .strip_suffix(" off")
                .filter(|rest| rest.ends_with("failsafe"))
        })
        .map(|rest| rest.trim_end_matches([' ', ':']).to_string())
        .filter(|subject| subject.len() >= 3)
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
    let component = component_tag
        .strip_prefix("COMP:")
        .and_then(|id| id.parse().ok());
    let (severity, html) = rest
        .trim_start()
        .split_once(": ")
        .unwrap_or(("", rest.trim_start()));
    let text = plain_text(html);
    Some(Message {
        index: 0,
        time: time.to_string(),
        component,
        severity: severity.to_string(),
        level: level_of(style),
        told_elsewhere: crate::refusal::told_elsewhere(&text),
        resolved: false,
        text,
    })
}

fn plain_text(html: &str) -> String {
    let with_newlines = html.replace("<br/>", "\n").replace("<br>", "\n");
    html_escape::decode_html_entities(&strip_tags(&with_newlines))
        .trim()
        .to_string()
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
        assert_eq!(
            parsed[1].level, "warning",
            "the level comes from the style token, not the translated word"
        );
        assert_eq!(parsed[1].text, "PreArm: Compass & GPS <inconsistent>");
        assert_eq!((parsed[0].index, parsed[1].index), (0, 1));
        assert_eq!(parsed[0].component, Some(1));
        assert_eq!(parsed[0].level, "normal");
        assert_eq!(parsed[0].text, "ArduCopter V4.5.7");
        assert_eq!(
            (parsed[1].told_elsewhere, parsed[0].told_elsewhere),
            (true, false),
            "a PreArm line is in the readiness list, so the heads keep it out of the banner"
        );
        assert!(
            parse("<font style=\"<#I>\">[1:2:3.4 ] Warning: No such mode 26</font><br/>")[0]
                .told_elsewhere,
            "the core already worded the refused mode change as a banner"
        );
        assert_eq!(
            parse("<font style=\"<#E>\">[1:2:3.4 ] Fehler: kaputt</font><br/>")[0].level,
            "error",
            "a localized severity word still buckets by the token"
        );
        assert_eq!(
            parse("<font style=\"<#I>\">[1:2:3.4 ] Notice: heads up</font><br/>")[0].level,
            "warning",
            "a notice buckets as a warning, as the Qt handler colours it"
        );
    }

    #[test]
    fn the_newest_message_is_last_as_it_is_in_every_other_list_the_core_serves() {
        let parsed = parse(TWO);
        assert_eq!(
            parsed.last().unwrap().time,
            "12:34:56.789",
            "the handler prepends, so the newest arrives first in the string and has to be moved to the end"
        );
        assert_eq!(parsed.first().unwrap().time, "12:34:55.000");
        assert!(
            parsed.first().unwrap().time < parsed.last().unwrap().time,
            "a head taking last() for the newest is right here, on the notices and on the track, rather than right on two of three"
        );
        assert_eq!(
            (parsed[0].index, parsed[1].index),
            (0, 1),
            "the index counts from the oldest, so it does not jump around as messages arrive"
        );
    }

    #[test]
    fn a_failsafe_cleared_later_is_resolved_and_the_clearing_line_is_no_error() {
        let parsed = parse(concat!(
            "<font style=\"<#E>\">[10:00:09.000 ] Critical: EKF Failsafe Cleared</font><br/>",
            "<font style=\"<#I>\">[10:00:08.000 ] Warning: Throttle failsafe off</font><br/>",
            "<font style=\"<#E>\">[10:00:07.000 ] Critical: Battery 1 is low 10.50V</font><br/>",
            "<font style=\"<#I>\">[10:00:06.000 ] Warning: Throttle failsafe on</font><br/>",
            "<font style=\"<#E>\">[10:00:05.000 ] Critical: EKF Failsafe: changed to LAND Mode</font><br/>",
        ));
        let read: Vec<(&str, &str, bool)> = parsed
            .iter()
            .map(|m| (m.text.as_str(), m.level, m.resolved))
            .collect();
        assert_eq!(
            read,
            vec![
                ("EKF Failsafe: changed to LAND Mode", "error", true),
                ("Throttle failsafe on", "warning", true),
                ("Battery 1 is low 10.50V", "error", false),
                ("Throttle failsafe off", "normal", false),
                ("EKF Failsafe Cleared", "normal", false),
            ],
            "a cleared failsafe is over, so neither it nor the line saying so stays up as an alarm"
        );
        assert_eq!(
            cleared_subject("Data link regained"),
            Some("data link".to_string())
        );
        assert_eq!(
            cleared_subject("Motors off"),
            None,
            "only a failsafe is cleared by off"
        );
    }

    #[test]
    fn keeps_multiline_text_in_one_message() {
        let parsed = parse(
            "<font style=\"c\">[1:2:3.4 ] Error: line one<br/>line two<br/><small><small>details</small></small></font><br/>",
        );
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].text, "line one\nline two\ndetails");
    }

    #[test]
    fn empty_input_is_no_messages() {
        assert!(parse("").is_empty());
    }
}
