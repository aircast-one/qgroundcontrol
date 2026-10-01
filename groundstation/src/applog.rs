use std::collections::{BTreeSet, VecDeque};
use std::sync::{Mutex, Once};

use chrono::{DateTime, Local};
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};

use crate::router::Backend;

pub const LOG_CLEAR: &str = "appLog.clear";
pub const LOG_SAVE: &str = "appLog.save";
pub const SET_CATEGORY: &str = "appLog.setCategory";
pub const RESET_CATEGORIES: &str = "appLog.resetCategories";
const FILTER_GROUP: &str = "LoggingFilters";
const CATEGORY_SEPARATOR: &str = "::";
const MAX_LOG_ENTRIES: usize = 100_000;
const LEVEL_LABELS: [&str; 5] = ["D", "I", "W", "C", "F"];
const LEVEL_NAMES: [&str; 6] = ["All Levels", "Debug", "Info", "Warning", "Critical", "Fatal"];
const CSV_HEADER: &str = "timestamp,level,category,message,file,line";

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub sequence: u64,
    pub timestamp: DateTime<Local>,
    pub elapsed_ms: u64,
    pub level: usize,
    pub category: String,
    pub message: String,
    pub file: String,
    pub line: u32,
}

#[derive(Default)]
struct Store {
    next: u64,
    entries: VecDeque<Entry>,
}

static STORE: Mutex<Store> = Mutex::new(Store { next: 0, entries: VecDeque::new() });
static INSTALL: Once = Once::new();
static ENABLED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

struct CoreLogger;

impl log::Log for CoreLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let target = record.target();
        if let Ok(mut seen) = SEEN.lock()
            && !seen.contains(target)
        {
            seen.insert(target.to_string());
        }
        let debug = record.level() >= log::Level::Debug;
        if debug && !ENABLED.lock().is_ok_and(|enabled| category_enabled(&enabled, target)) {
            return;
        }
        record_entry(level_of(record.level()), record.target(), &record.args().to_string(), record.file().unwrap_or_default(), record.line().unwrap_or(0));
    }

    fn flush(&self) {}
}

pub fn install() {
    INSTALL.call_once(|| {
        if log::set_logger(&CoreLogger).is_ok() {
            log::set_max_level(log::LevelFilter::Debug);
        }
    });
}

pub fn category_enabled(enabled: &BTreeSet<String>, target: &str) -> bool {
    enabled.iter().any(|category| target.strip_prefix(category.as_str()).is_some_and(|rest| rest.is_empty() || rest.starts_with(CATEGORY_SEPARATOR)))
}

pub fn load_categories() {
    let stored: BTreeSet<String> = crate::settingsstore::entries_under(FILTER_GROUP)
        .into_iter()
        .filter(|(_, value)| matches!(value, crate::settingsini::Setting::Text(text) if text == "true"))
        .filter_map(|(key, _)| key.strip_prefix(&format!("{FILTER_GROUP}/")).map(str::to_string))
        .collect();
    *ENABLED.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = stored;
}

fn filter_key(category: &str) -> String {
    format!("{FILTER_GROUP}/{category}")
}

pub fn set_category(args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let (Some(category), Some(on)) = (args.get(0).and_then(Value::as_str).filter(|c| !c.is_empty()), args.get(1).and_then(Value::as_bool)) else {
        return json!({ "ok": false, "reason": "appLog.setCategory takes a category name and whether its debug output is on" });
    };
    let mut enabled = ENABLED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match on {
        true => {
            enabled.insert(category.to_string());
            crate::settingsstore::written(&filter_key(category), "true");
        }
        false => {
            enabled.remove(category);
            crate::settingsstore::forgotten(&filter_key(category));
        }
    }
    json!({ "ok": true })
}

pub fn reset_categories() -> Value {
    let mut enabled = ENABLED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    enabled.iter().for_each(|category| crate::settingsstore::forgotten(&filter_key(category)));
    enabled.clear();
    json!({ "ok": true })
}

fn with_ancestors(target: &str) -> Vec<String> {
    let parts: Vec<&str> = target.split(CATEGORY_SEPARATOR).collect();
    (1..=parts.len()).map(|depth| parts[..depth].join(CATEGORY_SEPARATOR)).collect()
}

pub fn categories_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let enabled = ENABLED.lock().map(|enabled| enabled.clone()).unwrap_or_default();
    let seen = SEEN.lock().map(|seen| seen.clone()).unwrap_or_default();
    let all: BTreeSet<String> = seen.iter().chain(enabled.iter()).flat_map(|target| with_ancestors(target)).collect();
    json!({
        "kind": "object",
        "class": "LoggingCategories",
        "active": enabled,
        "categories": all.iter().map(|name| json!({
            "name": name,
            "shortName": name.rsplit(CATEGORY_SEPARATOR).next().unwrap_or(name),
            "depth": name.matches(CATEGORY_SEPARATOR).count(),
            "enabled": enabled.contains(name),
        })).collect::<Vec<_>>(),
    })
}

fn level_of(level: log::Level) -> usize {
    match level {
        log::Level::Error => 3,
        log::Level::Warn => 2,
        log::Level::Info => 1,
        log::Level::Debug | log::Level::Trace => 0,
    }
}

pub fn record_entry(level: usize, category: &str, message: &str, file: &str, line: u32) {
    let Ok(mut store) = STORE.lock() else { return };
    let sequence = store.next;
    store.next += 1;
    store.entries.push_back(Entry { sequence, timestamp: Local::now(), elapsed_ms: STARTED.elapsed().as_millis() as u64, level, category: category.to_string(), message: message.to_string(), file: file.to_string(), line });
    if store.entries.len() > MAX_LOG_ENTRIES {
        store.entries.pop_front();
    }
}

fn snapshot() -> Vec<Entry> {
    STORE.lock().map(|store| store.entries.iter().cloned().collect()).unwrap_or_default()
}

pub fn clear(_backend: &dyn Backend) -> Value {
    if let Ok(mut store) = STORE.lock() {
        store.entries.clear();
    }
    json!({ "ok": true })
}

fn timestamp(entry: &Entry) -> String {
    entry.timestamp.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()
}

fn formatted(entry: &Entry) -> String {
    format!("{} [{}] {}: {}", timestamp(entry), LEVEL_LABELS[entry.level], entry.category, entry.message)
}

fn source(entry: &Entry) -> String {
    match (entry.file.is_empty(), entry.line) {
        (true, _) => String::new(),
        (false, 0) => entry.file.clone(),
        (false, line) => format!("{}:{line}", entry.file),
    }
}

fn escape_csv(field: &str) -> String {
    if field.contains([',', '"', '\n']) { format!("\"{}\"", field.replace('"', "\"\"")) } else { field.to_string() }
}

pub fn as_text(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| match source(entry) {
            s if s.is_empty() => format!("{}\n", formatted(entry)),
            s => format!("{} ({s})\n", formatted(entry)),
        })
        .collect()
}

pub fn as_csv(entries: &[Entry]) -> String {
    std::iter::once(format!("{CSV_HEADER}\n"))
        .chain(entries.iter().map(|entry| {
            let line = if entry.file.is_empty() { String::new() } else { entry.line.to_string() };
            format!("{},{},{},{},{},{line}\n", escape_csv(&timestamp(entry)), LEVEL_LABELS[entry.level], escape_csv(&entry.category), escape_csv(&entry.message), escape_csv(&entry.file))
        }))
        .collect()
}

pub fn save(_backend: &dyn Backend, file_name: &str) -> Value {
    let entries = snapshot();
    let text = if file_name.to_lowercase().ends_with(".csv") { as_csv(&entries) } else { as_text(&entries) };
    json!({ "ok": true, "result": text })
}

fn decoded(arg: Option<&String>) -> String {
    arg.map(|raw| url::form_urlencoded::parse(format!("q={raw}").as_bytes()).map(|(_, v)| v.into_owned()).next().unwrap_or_default()).unwrap_or_default()
}

pub struct Filter {
    pub level: usize,
    pub category: String,
    pub text: String,
    pub regex: bool,
}

enum Matcher {
    Everything,
    Substring(String),
    Pattern(Regex),
    Nothing,
}

impl Filter {
    fn matcher(&self) -> Matcher {
        match (self.text.is_empty(), self.regex) {
            (true, _) => Matcher::Everything,
            (false, false) => Matcher::Substring(self.text.to_lowercase()),
            (false, true) => RegexBuilder::new(&self.text).build().map_or(Matcher::Nothing, Matcher::Pattern),
        }
    }
}

fn accepts(filter: &Filter, category: &str, matcher: &Matcher, entry: &Entry) -> bool {
    entry.level >= filter.level
        && (category.is_empty() || entry.category.to_lowercase().contains(category))
        && match matcher {
            Matcher::Everything => true,
            Matcher::Substring(text) => entry.message.to_lowercase().contains(text.as_str()),
            Matcher::Pattern(pattern) => pattern.is_match(&entry.message),
            Matcher::Nothing => false,
        }
}

pub fn passes(filter: &Filter, entry: &Entry) -> bool {
    accepts(filter, &filter.category.to_lowercase(), &filter.matcher(), entry)
}

static STARTED: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);
const ELAPSED_SETTING: &str = "settings.appSettings.showAppLogTimestampAsElapsedTime";

pub fn shown_time(entry: &Entry, elapsed: bool) -> String {
    match elapsed {
        true => format!("{:.3}", entry.elapsed_ms as f64 / 1000.0),
        false => entry.timestamp.format("%H:%M:%S%.3f").to_string(),
    }
}

fn entry_json(entry: &Entry) -> Value {
    json!({
        "sequence": entry.sequence,
        "level": entry.level,
        "message": format!("{} {}", LEVEL_LABELS[entry.level], entry.message),
        "category": entry.category,
        "timestamp": shown_time(entry, crate::settingsstore::raw_setting(ELAPSED_SETTING).and_then(|v| v.as_bool()).unwrap_or(false)),
        "source": source(entry),
    })
}

pub fn log_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let filter = Filter {
        level: args.first().and_then(|a| a.parse().ok()).unwrap_or(0),
        category: decoded(args.get(1)),
        text: decoded(args.get(2)),
        regex: args.get(3).is_some_and(|a| a == "1"),
    };
    let after: Option<u64> = args.get(4).and_then(|a| a.parse().ok());
    let matcher = filter.matcher();
    let category = filter.category.to_lowercase();
    let store = STORE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let categories: BTreeSet<&str> = store.entries.iter().map(|entry| entry.category.as_str()).collect();
    json!({
        "kind": "object",
        "class": "AppLog",
        "levels": LEVEL_NAMES,
        "categories": std::iter::once("All Categories").chain(categories).collect::<Vec<_>>(),
        "headers": ["Message", "Category", "Timestamp", "Source"],
        "empty": "No log entries",
        "regexValid": !matches!(matcher, Matcher::Nothing),
        "first": store.entries.front().map(|entry| entry.sequence),
        "entries": store
            .entries
            .iter()
            .skip_while(|entry| after.is_some_and(|after| entry.sequence <= after))
            .filter(|entry| accepts(&filter, &category, &matcher, entry))
            .map(entry_json)
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_time_is_the_clock_or_the_seconds_since_start() {
        let at = Entry { sequence: 0, timestamp: Local.with_ymd_and_hms(2026, 10, 1, 3, 20, 11).unwrap(), elapsed_ms: 12_345, level: 1, category: String::new(), message: String::new(), file: String::new(), line: 0 };
        assert_eq!(shown_time(&at, false), "03:20:11.000");
        assert_eq!(shown_time(&at, true), "12.345");
    }
    use chrono::TimeZone;

    struct Nothing;

    impl Backend for Nothing {
        fn get(&self, _path: &str) -> String {
            String::new()
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            String::new()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn a_category_turns_on_its_own_debug_output_and_everything_under_it() {
        let enabled: BTreeSet<String> = ["groundstation::hub".to_string(), "ureq".to_string()].into();
        assert!(category_enabled(&enabled, "groundstation::hub"));
        assert!(category_enabled(&enabled, "groundstation::hub::links"));
        assert!(category_enabled(&enabled, "ureq::unversioned"));
        assert!(!category_enabled(&enabled, "groundstation::hubble"), "a name that only starts the same is another category");
        assert!(!category_enabled(&enabled, "groundstation"));
        assert_eq!(with_ancestors("groundstation::hub::links"), ["groundstation", "groundstation::hub", "groundstation::hub::links"]);
        assert_eq!(set_category(r#"["groundstation::hub"]"#)["ok"], false);
    }

    fn entry(level: usize, category: &str, message: &str, file: &str, line: u32) -> Entry {
        Entry { sequence: 0, timestamp: Local.with_ymd_and_hms(2026, 10, 1, 3, 20, 11).unwrap(), elapsed_ms: 12_345, level, category: category.into(), message: message.into(), file: file.into(), line }
    }

    #[test]
    fn saves_in_the_log_formatter_layouts() {
        let entries = [entry(2, "hub", "link lost", "src/hub.rs", 12), entry(1, "hub", "a, \"b\"", "", 0)];
        assert_eq!(as_text(&entries), "2026-10-01T03:20:11.000 [W] hub: link lost (src/hub.rs:12)\n2026-10-01T03:20:11.000 [I] hub: a, \"b\"\n");
        assert_eq!(as_csv(&entries), "timestamp,level,category,message,file,line\n2026-10-01T03:20:11.000,W,hub,link lost,src/hub.rs,12\n2026-10-01T03:20:11.000,I,hub,\"a, \"\"b\"\"\",,\n");
    }

    #[test]
    fn filters_like_the_log_model() {
        let warn = entry(2, "groundstation::hub", "Link 3 lost", "", 0);
        let filter = |level, category: &str, text: &str, regex| Filter { level, category: category.into(), text: text.into(), regex };
        assert!(passes(&filter(0, "", "", false), &warn));
        assert!(!passes(&filter(3, "", "", false), &warn), "below the level");
        assert!(passes(&filter(0, "HUB", "", false), &warn), "category matches as a case-insensitive substring");
        assert!(passes(&filter(0, "", "link", false), &warn), "text matches case-insensitively");
        assert!(passes(&filter(0, "", r"Link \d+", true), &warn));
        assert!(!passes(&filter(0, "", "(", true), &warn), "an invalid pattern matches nothing");
    }

    #[test]
    fn serves_new_entries_after_a_sequence() {
        install();
        log::warn!(target: "applog-test", "first, with comma");
        log::info!(target: "applog-test", "second");
        let view = log_view(&Nothing, &["0".into(), "applog-test".into(), "comma".into(), "0".into()]);
        let seen = view["entries"].as_array().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0]["message"], "W first, with comma");
        let after = seen[0]["sequence"].as_u64().unwrap().to_string();
        let later = log_view(&Nothing, &["0".into(), "applog-test".into(), String::new(), "0".into(), after]);
        assert_eq!(later["entries"].as_array().unwrap().iter().map(|e| e["message"].as_str().unwrap()).collect::<Vec<_>>(), ["I second"]);
    }
}
