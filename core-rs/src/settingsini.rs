use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Text(String),
    List(Vec<String>),
    Bytes(Vec<u8>),
    Invalid,
    Variant(String),
}

fn hex(byte: &[u8]) -> Option<u8> {
    std::str::from_utf8(byte).ok().and_then(|s| u8::from_str_radix(s, 16).ok())
}

fn unescape_key(key: &str) -> String {
    let bytes = key.as_bytes();
    let decoded = (0..bytes.len()).fold((Vec::new(), 0usize), |(mut out, skip), i| {
        if skip > 0 {
            return (out, skip - 1);
        }
        match bytes[i] {
            b'%' if i + 2 < bytes.len() + 0 && hex(&bytes[i + 1..i + 3]).is_some() => {
                out.push(hex(&bytes[i + 1..i + 3]).unwrap());
                (out, 2)
            }
            b'\\' => {
                out.push(b'/');
                (out, 0)
            }
            b => {
                out.push(b);
                (out, 0)
            }
        }
    });
    String::from_utf8_lossy(&decoded.0).into_owned()
}

fn unquote(text: &str) -> String {
    let inner = &text[1..text.len().saturating_sub(1)];
    let chars: Vec<char> = inner.chars().collect();
    let (out, _) = chars.iter().enumerate().fold((String::new(), 0usize), |(mut out, skip), (i, c)| {
        if skip > 0 {
            return (out, skip - 1);
        }
        if *c != '\\' || i + 1 >= chars.len() {
            out.push(*c);
            return (out, 0);
        }
        match chars[i + 1] {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            'x' => {
                let digits: String = chars[i + 2..].iter().take_while(|d| d.is_ascii_hexdigit()).take(4).collect();
                let code = u32::from_str_radix(&digits, 16).ok().and_then(char::from_u32).unwrap_or('\u{FFFD}');
                out.push(code);
                return (out, 1 + digits.len());
            }
            other => out.push(other),
        }
        (out, 1)
    });
    out
}

fn split_list(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let (mut items, current, _, _) = chars.iter().fold((Vec::new(), String::new(), false, false), |(mut items, mut current, quoted, escaped), c| match (c, quoted, escaped) {
        (_, _, true) => {
            current.push('\\');
            current.push(*c);
            (items, current, quoted, false)
        }
        ('\\', true, false) => (items, current, quoted, true),
        ('"', _, false) => {
            current.push('"');
            (items, current, !quoted, false)
        }
        (',', false, false) => {
            items.push(current);
            (items, String::new(), false, false)
        }
        _ => {
            current.push(*c);
            (items, current, quoted, false)
        }
    });
    items.push(current);
    items.into_iter().map(|item| value_text(item.trim())).collect()
}

fn value_text(text: &str) -> String {
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') { unquote(text) } else { text.to_string() }
}

pub fn value(raw: &str) -> Setting {
    let text = raw.trim();
    match text {
        "@Invalid()" => Setting::Invalid,
        t if t.starts_with("@ByteArray(") && t.ends_with(')') => Setting::Bytes(t["@ByteArray(".len()..t.len() - 1].as_bytes().to_vec()),
        t if t.starts_with('@') && t.ends_with(')') => Setting::Variant(t.to_string()),
        t => match split_list(t).as_slice() {
            [single] => Setting::Text(single.clone()),
            items => Setting::List(items.to_vec()),
        },
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

    const SAMPLE: &str = "[General]\nofflineEditingCruiseSpeed=15\nlastKnownHome=\"47.6, -122.1\"\nsavedFile=@Invalid()\n\n[Units]\nverticalDistanceUnits=1\n\n[Video]\nrtspUrl=\"rtsp://host:8554/live\"\n\n[Branding]\nuserBrandImageIndoor=\"/Users/me/Desktop/My%20Logo.png\"\n\n[LinkConfigurations]\nLink0\\name=\"UDP Link\"\nLink0\\hostList=host1:14550, \"host two:14551\"\ncount=1\nblob=@ByteArray(abc)\nvariant=@Variant(\\0\\0\\0\\x7f)\nescaped=\"line\\nnext \\\"q\\\" \\x41\"\n";

    #[test]
    fn groups_and_keys_flatten_to_the_qsettings_paths() {
        let settings = read(SAMPLE);
        assert_eq!(settings["offlineEditingCruiseSpeed"], Setting::Text("15".into()));
        assert_eq!(settings["Units/verticalDistanceUnits"], Setting::Text("1".into()));
        assert_eq!(settings["Video/rtspUrl"], Setting::Text("rtsp://host:8554/live".into()));
        assert_eq!(settings["Branding/userBrandImageIndoor"], Setting::Text("/Users/me/Desktop/My%20Logo.png".into()));
        assert_eq!(settings["LinkConfigurations/Link0/name"], Setting::Text("UDP Link".into()));
        assert_eq!(settings["savedFile"], Setting::Invalid);
    }

    #[test]
    fn lists_quotes_and_special_values_decode() {
        let settings = read(SAMPLE);
        assert_eq!(settings["lastKnownHome"], Setting::Text("47.6, -122.1".into()));
        assert_eq!(settings["LinkConfigurations/Link0/hostList"], Setting::List(vec!["host1:14550".into(), "host two:14551".into()]));
        assert_eq!(settings["LinkConfigurations/blob"], Setting::Bytes(b"abc".to_vec()));
        assert!(matches!(settings["LinkConfigurations/variant"], Setting::Variant(_)));
        assert_eq!(settings["LinkConfigurations/escaped"], Setting::Text("line\nnext \"q\" A".into()));
        assert_eq!(unescape_key("My%20Key\\sub"), "My Key/sub");
    }
}
