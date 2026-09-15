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


fn escape_key(key: &str) -> String {
    key.chars()
        .map(|c| match c {
            '/' => "\\".to_string(),
            '\\' => "%5C".to_string(),
            '%' => "%25".to_string(),
            '[' | ']' | '=' => format!("%{:02X}", c as u32),
            c if c.is_ascii_graphic() || c == ' ' => c.to_string(),
            c if (c as u32) < 0x100 => format!("%{:02X}", c as u32),
            c => format!("%U{:04X}", c as u32),
        })
        .collect()
}

fn escaped_body(item: &str) -> String {
    // A \xNN escape consumes hex digits greedily on the way back in, so "bell\x7end" reads as
    // \x7e followed by "nd". Closing and reopening the quotes ends the escape without adding a
    // character: QSettings concatenates adjacent quoted runs, and so does the reader above.
    let chars: Vec<char> = item.chars().collect();
    chars
        .iter()
        .enumerate()
        .map(|(at, c)| {
            let hex_follows = chars.get(at + 1).is_some_and(char::is_ascii_hexdigit);
            match c {
                '\\' => "\\\\".to_string(),
                '"' => "\\\"".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                c if (*c as u32) < 0x20 || *c as u32 == 0x7f => {
                    format!("\\x{:x}{}", *c as u32, if hex_follows { "\"\"" } else { "" })
                }
                c => c.to_string(),
            }
        })
        .collect()
}

fn escape_item(item: &str) -> String {
    format!("\"{}\"", escaped_body(item))
}

fn spell(setting: &Setting) -> String {
    match setting {
        Setting::Invalid => "@Invalid()".to_string(),
        Setting::Variant(raw) => raw.clone(),
        Setting::Bytes(bytes) => format!("@ByteArray({})", escaped_body(&String::from_utf8_lossy(bytes))),
        Setting::Text(text) if text.starts_with('@') => format!("@{}", escaped_body(text)),
        Setting::Text(text) => escape_item(text),
        Setting::List(items) => items.iter().map(|item| escape_item(item)).collect::<Vec<_>>().join(", "),
    }
}

fn split_section(path: &str) -> (&str, &str) {
    match path.split_once('/') {
        Some((section, rest)) => (section, rest),
        None => ("General", path),
    }
}

pub fn write(settings: &BTreeMap<String, Setting>) -> String {
    let sections: BTreeMap<&str, Vec<(&str, &Setting)>> =
        settings.iter().fold(BTreeMap::new(), |mut grouped, (path, setting)| {
            let (section, key) = split_section(path);
            grouped.entry(section).or_default().push((key, setting));
            grouped
        });
    sections
        .iter()
        .map(|(section, entries)| {
            let lines: String = entries
                .iter()
                .map(|(key, setting)| format!("{}={}\n", escape_key(key), spell(setting)))
                .collect();
            format!("[{}]\n{lines}", escape_key(section))
        })
        .collect::<Vec<_>>()
        .join("\n")
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

    #[test]
    fn what_the_writer_produces_is_what_the_reader_understands() {
        let once = read(SAMPLE);
        let twice = read(&write(&once));
        assert_eq!(twice, once, "a writer is only trustworthy if the parser it was derived from reads its output back unchanged, and that is the one property neither a fixture nor a rig can fake");

        let spelled = write(&once);
        assert!(spelled.contains("[General]"), "keys with no group belong to General, which QSettings writes without a prefix and reads back the same way");
        assert!(spelled.contains("[LinkConfigurations]"));
        assert!(spelled.contains("Link0\\name="), "a slash inside a key is a backslash in the file - the section is the first segment and every segment after it is part of the key");
        assert!(spelled.contains("@Invalid()"), "an invalid setting is a value QSettings writes, not an absence");
    }

    #[test]
    fn every_shape_the_reader_can_produce_survives_being_written() {
        let awkward: BTreeMap<String, Setting> = [
            ("Units/verticalDistanceUnits".to_string(), Setting::Text("1".into())),
            ("plain".to_string(), Setting::Text("no quotes needed".into())),
            ("comma".to_string(), Setting::Text("47.6, -122.1".into())),
            ("quote".to_string(), Setting::Text("say \"hi\"".into())),
            ("backslash".to_string(), Setting::Text("C:\\logs\\flight.tlog".into())),
            ("newline".to_string(), Setting::Text("line\nnext".into())),
            ("control".to_string(), Setting::Text("bell\u{7}end".into())),
            ("at".to_string(), Setting::Text("@handle".into())),
            ("list".to_string(), Setting::List(vec!["host1:14550".into(), "host two:14551".into()])),
            ("gone".to_string(), Setting::Invalid),
            ("Gro\u{DC}p\u{E9}/k\u{E9}y".to_string(), Setting::Text("unicode in both".into())),
            ("has space/and more".to_string(), Setting::Text("spaced".into())),
        ]
        .into_iter()
        .collect();

        assert_eq!(read(&write(&awkward)), awkward, "every value a head can store has to survive the round trip, or the first operator with a comma in a field loses it");
    }

    #[test]
    fn a_settings_file_qt_actually_wrote_survives_the_round_trip() {
        let written_by_qt = include_str!("../tests/fixtures/qgc-settings.ini");
        let once = read(written_by_qt);
        assert!(once.len() > 100, "the fixture parsed to {} keys, which is too few to be the whole file", once.len());
        assert_eq!(read(&write(&once)), once, "a hand-written sample agrees with whatever the parser happens to do; this one was produced by QSettings and is the only input here that can disagree with it");
        assert_eq!(once["LinkConfigurations/Link0/name"], Setting::Text("Recorder TCP".into()), "a backslash in the file is a slash in the flattened key, and this is the shape link configurations actually arrive in");
        assert_eq!(once["FlyView/rcControls"], Setting::Text("[]".into()), "an empty JSON list is a two-character string and not a list, because QSettings never quoted or comma-separated it");
        assert_eq!(once["Video/rtspUrl"], Setting::Text("rtsp://127.0.0.1:8554/fresh".into()));
    }

    #[test]
    fn a_quote_at_the_end_of_a_byte_array_or_an_at_prefixed_string_is_not_eaten() {
        let bytes = Setting::Bytes(b"ab\"".to_vec());
        assert_eq!(value(&spell(&bytes)), bytes, "@ByteArray and @@ wrap a body rather than a quoted string, so stripping the quotes off a quoted one takes the escaped quote at the end with them and the last byte is lost");

        let at = Setting::Text("@x\"".to_string());
        assert_eq!(value(&spell(&at)), at);

        let plain = Setting::Text("ends with a quote\"".to_string());
        assert_eq!(value(&spell(&plain)), plain, "the same content is safe in a value that IS quoted, which is why this only bites the two prefixed forms");
    }

    #[test]
    fn a_backslash_inside_a_key_is_not_a_path_separator() {
        let awkward: BTreeMap<String, Setting> = [("Sec/back\\slash".to_string(), Setting::Text("1".into()))].into_iter().collect();
        assert_eq!(read(&write(&awkward)), awkward, "a backslash in the file means a subgroup, so a key that contains one has to be percent-encoded or it comes back as two keys joined by a slash");
        assert!(write(&awkward).contains("back%5Cslash"));
    }

    #[test]
    fn the_general_section_has_no_prefix_in_either_direction() {
        let named: BTreeMap<String, Setting> = [("General/x".to_string(), Setting::Text("1".into()))].into_iter().collect();
        assert_eq!(read(&write(&named)).keys().collect::<Vec<_>>(), vec!["x"], "QSettings writes ungrouped keys under [General] and reads them back without the prefix, so General/x and x are the same setting - a head storing one and reading the other gets the value it wrote, and this is the format's rule rather than ours");
    }

    fn golden_map() -> BTreeMap<String, Setting> {
        [
            ("LinkConfigurations/Link0/name".to_string(), Setting::Text("radio 1".into())),
            ("Probe/bell".to_string(), Setting::Text("bell\u{7}end".into())),
            ("Probe/blobEndingInAQuote".to_string(), Setting::Bytes(b"ab\"".to_vec())),
            ("Probe/atEndingInAQuote".to_string(), Setting::Text("@x\"".into())),
            ("Probe/back\\slash".to_string(), Setting::Text("1".into())),
            ("Probe/hosts".to_string(), Setting::List(vec!["a".into(), "b, c".into()])),
            ("Probe/gone".to_string(), Setting::Invalid),
            ("Probe/url".to_string(), Setting::Text("rtsp://h:554/a/b".into())),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn the_writer_produces_the_golden_file_a_real_qsettings_is_tested_against() {
        let golden = include_str!("../tests/fixtures/writer-golden.ini");
        assert_eq!(write(&golden_map()), golden, "this file is the contract between this writer and the C++ test that opens it with a real QSettings; neither side regenerates it, so a change here has to be made deliberately and re-checked against Qt");
        assert_eq!(read(golden), golden_map(), "and it still has to mean the same thing coming back, or the golden pins a spelling that has stopped being correct");
    }
}
