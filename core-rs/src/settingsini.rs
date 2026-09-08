use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Text(String),
    List(Vec<String>),
    Bytes(Vec<u8>),
    Invalid,
    Variant(String),
}

fn key_code(text: &str, at: usize, len: usize) -> Option<u32> {
    text.get(at..at + len).and_then(|h| u32::from_str_radix(h, 16).ok())
}

pub fn unescape_key(key: &str) -> String {
    let mut out = String::new();
    let mut at = 0usize;
    while at < key.len() {
        let rest = &key[at..];
        if rest.starts_with("%U") {
            if let Some(code) = key_code(key, at + 2, 4).and_then(char::from_u32) {
                out.push(code);
                at += 6;
                continue;
            }
        }
        if rest.starts_with('%') {
            if let Some(code) = key_code(key, at + 1, 2).and_then(char::from_u32) {
                out.push(code);
                at += 3;
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(if c == '\\' { '/' } else { c });
        at += c.len_utf8();
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Normal,
    Quoted,
    Escape,
    Hex,
    Octal,
}

fn unescaped_list(text: &str) -> (Vec<String>, bool) {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quoted_item = false;
    let mut is_list = false;
    let mut state = State::Normal;
    let mut before_escape = State::Normal;
    let mut code: u32 = 0;
    let finish = |current: &mut String, quoted: &mut bool, items: &mut Vec<String>| {
        let item = if *quoted { std::mem::take(current) } else { std::mem::take(current).trim().to_string() };
        items.push(item);
        *quoted = false;
    };
    for c in text.chars() {
        loop {
            match state {
                State::Normal => match c {
                    '"' => {
                        state = State::Quoted;
                        quoted_item = true;
                        current = if current.trim().is_empty() { String::new() } else { current.trim_end().to_string() };
                    }
                    ',' => {
                        is_list = true;
                        finish(&mut current, &mut quoted_item, &mut items);
                    }
                    '\\' => {
                        before_escape = State::Normal;
                        state = State::Escape;
                    }
                    _ => current.push(c),
                },
                State::Quoted => match c {
                    '"' => state = State::Normal,
                    '\\' => {
                        before_escape = State::Quoted;
                        state = State::Escape;
                    }
                    _ => current.push(c),
                },
                State::Escape => {
                    state = before_escape;
                    match c {
                        'b' => current.push('\u{8}'),
                        'f' => current.push('\u{c}'),
                        'n' => current.push('\n'),
                        'r' => current.push('\r'),
                        't' => current.push('\t'),
                        'v' => current.push('\u{b}'),
                        'a' => current.push('\u{7}'),
                        'x' => {
                            code = 0;
                            state = State::Hex;
                        }
                        '0'..='7' => {
                            code = c as u32 - '0' as u32;
                            state = State::Octal;
                        }
                        '\n' => {}
                        other => current.push(other),
                    }
                }
                State::Hex => match c.to_digit(16) {
                    Some(d) => code = (code << 4) | d,
                    None => {
                        current.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        state = before_escape;
                        continue;
                    }
                },
                State::Octal => match c.to_digit(8) {
                    Some(d) => code = code * 8 + d,
                    None => {
                        current.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        state = before_escape;
                        continue;
                    }
                },
            }
            break;
        }
    }
    if matches!(state, State::Hex | State::Octal) {
        current.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
    }
    finish(&mut current, &mut quoted_item, &mut items);
    (items, is_list)
}

fn special(text: &str) -> Setting {
    match text {
        "@Invalid()" => Setting::Invalid,
        t if t.starts_with("@ByteArray(") && t.ends_with(')') => Setting::Bytes(t["@ByteArray(".len()..t.len() - 1].as_bytes().to_vec()),
        t if t.starts_with("@String(") && t.ends_with(')') => Setting::Text(t["@String(".len()..t.len() - 1].to_string()),
        t if t.starts_with("@@") => Setting::Text(t[1..].to_string()),
        t if t.starts_with('@') && t.ends_with(')') => Setting::Variant(t.to_string()),
        t => Setting::Text(t.to_string()),
    }
}

pub fn value(raw: &str) -> Setting {
    let (items, is_list) = unescaped_list(raw.trim());
    match (is_list, items.as_slice()) {
        (false, [single]) => special(single),
        _ => Setting::List(items),
    }
}

pub fn read(text: &str) -> BTreeMap<String, Setting> {
    let (map, _) = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';') && !l.starts_with('#')).fold((BTreeMap::new(), String::new()), |(mut map, section), line| {
        if line.starts_with('[') && line.ends_with(']') {
            let name = unescape_key(&line[1..line.len() - 1]);
            return (map, if name == "General" { String::new() } else { name });
        }
        let Some((key, raw)) = line.split_once('=') else { return (map, section) };
        let key = unescape_key(key.trim());
        let full = if section.is_empty() { key } else { format!("{section}/{key}") };
        map.insert(full, value(raw));
        (map, section)
    });
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[General]\nofflineEditingCruiseSpeed=15\nlastKnownHome=\"47.6, -122.1\"\nsavedFile=@Invalid()\n\n[Units]\nverticalDistanceUnits=1\n\n[Video]\nrtspUrl=\"rtsp://host:8554/live\"\n\n[Branding]\nuserBrandImageIndoor=\"/Users/me/Desktop/My%20Logo.png\"\n\n[LinkConfigurations]\nLink0\\name=\"UDP Link\"\nLink0\\hostList=host1:14550, \"host two:14551\"\ncount=1\nblob=@ByteArray(abc)\nvariant=@Variant(\\0\\0\\0\\x7f)\nescaped=\"line\\nnext \\\"q\\\" \\x41\"\nwinpath=C:\\\\logs\\\\flight.tlog\nat=@@handle\nsingle=1, 2\n";

    #[test]
    fn groups_and_keys_flatten_to_the_qsettings_paths() {
        let settings = read(SAMPLE);
        assert_eq!(settings["offlineEditingCruiseSpeed"], Setting::Text("15".into()));
        assert_eq!(settings["Units/verticalDistanceUnits"], Setting::Text("1".into()));
        assert_eq!(settings["Video/rtspUrl"], Setting::Text("rtsp://host:8554/live".into()));
        assert_eq!(settings["Branding/userBrandImageIndoor"], Setting::Text("/Users/me/Desktop/My%20Logo.png".into()));
        assert_eq!(settings["LinkConfigurations/Link0/name"], Setting::Text("UDP Link".into()));
        assert_eq!(settings["savedFile"], Setting::Invalid);
        assert_eq!(unescape_key("My%20Key\\sub"), "My Key/sub");
        assert_eq!(unescape_key("Gro%DCp%U00E9"), "GroÜpé");
    }

    #[test]
    fn escapes_apply_everywhere_and_lists_split_only_outside_quotes() {
        let settings = read(SAMPLE);
        assert_eq!(settings["lastKnownHome"], Setting::Text("47.6, -122.1".into()));
        assert_eq!(settings["LinkConfigurations/Link0/hostList"], Setting::List(vec!["host1:14550".into(), "host two:14551".into()]));
        assert_eq!(settings["LinkConfigurations/blob"], Setting::Bytes(b"abc".to_vec()));
        assert!(matches!(settings["LinkConfigurations/variant"], Setting::Variant(_)));
        assert_eq!(settings["LinkConfigurations/escaped"], Setting::Text("line\nnext \"q\" A".into()));
        assert_eq!(settings["LinkConfigurations/winpath"], Setting::Text("C:\\logs\\flight.tlog".into()));
        assert_eq!(settings["LinkConfigurations/at"], Setting::Text("@handle".into()));
        assert_eq!(settings["LinkConfigurations/single"], Setting::List(vec!["1".into(), "2".into()]));
        assert_eq!(value("say \\\"hi\\\""), Setting::Text("say \"hi\"".into()));
        assert_eq!(value("\\101\\x42-c"), Setting::Text("AB-c".into()));
        assert_eq!(value("@ByteArray(\\x1\\xd9)"), Setting::Bytes(vec![0x01, 0xC3, 0x99]));
    }
}
