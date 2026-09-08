use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
pub struct Message {
    pub time: String,
    pub component: Option<u32>,
    pub severity: String,
    pub text: String,
}

pub fn parse(formatted: &str) -> Vec<Message> {
    formatted
        .split("</font><br/>")
        .filter(|chunk| !chunk.trim().is_empty())
        .filter_map(parse_one)
        .collect()
}

fn parse_one(chunk: &str) -> Option<Message> {
    let body = chunk.split_once("\">").map_or(chunk, |(_, rest)| rest);
    let (head, rest) = body.strip_prefix('[')?.split_once(']')?;
    let (time, component_tag) = head.split_once(' ').unwrap_or((head, ""));
    let component = component_tag.strip_prefix("COMP:").and_then(|id| id.parse().ok());
    let (severity, html) = rest.trim_start().split_once(": ").unwrap_or(("", rest.trim_start()));
    Some(Message {
        time: time.to_string(),
        component,
        severity: severity.to_string(),
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
        "<font style=\"color:orange\">[12:34:56.789 ] Warning: PreArm: Compass &amp; GPS &lt;inconsistent&gt;</font><br/>",
        "<font style=\"color:white\">[12:34:55.000 COMP:1] Info: ArduCopter V4.5.7</font><br/>",
    );

    #[test]
    fn splits_messages_and_decodes_entities() {
        let parsed = parse(TWO);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].time, "12:34:56.789");
        assert_eq!(parsed[0].component, None);
        assert_eq!(parsed[0].severity, "Warning");
        assert_eq!(parsed[0].text, "PreArm: Compass & GPS <inconsistent>");
        assert_eq!(parsed[1].component, Some(1));
        assert_eq!(parsed[1].text, "ArduCopter V4.5.7");
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
