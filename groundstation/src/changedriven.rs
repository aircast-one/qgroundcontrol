use std::cell::{Cell, RefCell};

use serde_json::Value;

use crate::router::Backend;

pub const VIEWS: &[&str] = &["view.battery", "view.control", "view.settings"];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Source {
    Settings,
    Batteries,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Settings(u64),
    Batteries(Option<u8>, Vec<(u8, crate::batteryfacts::BatteryFacts)>),
}

fn source(path: &str) -> Option<Source> {
    match path {
        "units" => Some(Source::Settings),
        _ if path.starts_with("settings.") || path.starts_with("units.") => Some(Source::Settings),
        _ if path.starts_with("vehicle.batteries.") => Some(Source::Batteries),
        _ => None,
    }
}

fn now(source: Source, settings: u64) -> Input {
    match source {
        Source::Settings => Input::Settings(settings),
        Source::Batteries => {
            let (vehicle, packs) = crate::vehiclefacade::battery_inputs();
            Input::Batteries(vehicle, packs)
        }
    }
}

pub fn gated(path: &str) -> bool {
    !crate::qthost::present() && crate::vehiclefacade::switched_on() && crate::settingsstore::enabled() && VIEWS.contains(&crate::view::split(path).0)
}

pub fn unchanged(inputs: &[Input]) -> bool {
    let settings = crate::settingsstore::revision();
    inputs.iter().all(|input| {
        let source = match input {
            Input::Settings(_) => Source::Settings,
            Input::Batteries(..) => Source::Batteries,
        };
        now(source, settings) == *input
    })
}

pub struct Recorder<'a> {
    inner: &'a dyn Backend,
    read: RefCell<Vec<String>>,
    wrote: Cell<bool>,
    settings: u64,
}

impl<'a> Recorder<'a> {
    pub fn new(inner: &'a dyn Backend) -> Self {
        Self { inner, read: RefCell::default(), wrote: Cell::new(false), settings: crate::settingsstore::revision() }
    }

    pub fn inputs(&self) -> Option<Vec<Input>> {
        (!self.wrote.get()).then_some(())?;
        let sources: Vec<Source> = self.read.borrow().iter().map(|path| source(path)).collect::<Option<_>>()?;
        Some(sources.iter().fold(Vec::new(), |seen: Vec<Source>, s| if seen.contains(s) { seen } else { [seen, vec![*s]].concat() }).into_iter().map(|s| now(s, self.settings)).collect())
    }

    fn reading(&self, path: &str) {
        self.read.borrow_mut().push(path.to_string());
    }
}

impl Backend for Recorder<'_> {
    fn get(&self, path: &str) -> String {
        self.reading(path);
        self.inner.get(path)
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        self.reading(path);
        self.inner.get_fields(path, fields)
    }
    fn value(&self, path: &str) -> Value {
        self.reading(path);
        self.inner.value(path)
    }
    fn value_fields(&self, path: &str, fields: &str) -> Value {
        self.reading(path);
        self.inner.value_fields(path, fields)
    }
    fn set(&self, path: &str, value: &str) -> String {
        self.wrote.set(true);
        self.inner.set(path, value)
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        self.wrote.set(true);
        self.inner.invoke(path, args)
    }
    fn watch(&self, paths: &[String]) {
        self.inner.watch(paths);
    }
    fn core_guided(&self, action: &Value) -> Option<Result<(), String>> {
        self.wrote.set(true);
        self.inner.core_guided(action)
    }
    fn remember_setting(&self, key: &str, value: &Value) {
        self.wrote.set(true);
        self.inner.remember_setting(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Answers;
    impl Backend for Answers {
        fn get(&self, _p: &str) -> String {
            json!({ "kind": "value", "value": 1 }).to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_render_that_read_only_settings_is_stamped_with_the_settings_revision() {
        let recorder = Recorder::new(&Answers);
        recorder.value("settings.batteryIndicatorSettings.threshold1.rawValue");
        recorder.value_fields("units", "horizontalDistanceUnits");
        assert_eq!(recorder.inputs(), Some(vec![Input::Settings(recorder.settings)]), "two settings reads are one input");
    }

    #[test]
    fn a_render_that_read_anything_unmapped_or_wrote_is_never_reused() {
        let unmapped = Recorder::new(&Answers);
        unmapped.value("settings.appSettings.audioMuted");
        unmapped.value("vehicle.roll");
        assert_eq!(unmapped.inputs(), None, "the roll changes without any revision the router can see");
        let writer = Recorder::new(&Answers);
        writer.value("settings.appSettings.audioMuted");
        writer.invoke("vehicle.guidedModeLand", "[]");
        assert_eq!(writer.inputs(), None, "a render with a side effect runs every pass");
    }

    #[test]
    fn a_settings_write_makes_a_settings_stamp_stale() {
        let stamp = [Input::Settings(crate::settingsstore::revision())];
        assert!(unchanged(&stamp));
        crate::settingsstore::written("ChangeDriven/probe", "1");
        assert!(!unchanged(&stamp), "the write bumps the revision after it lands");
    }
}
