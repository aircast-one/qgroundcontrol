use roxmltree::Document;

pub const PREFIX: &str = "ParameterFactMetaData";
pub const PX4_FIRMWARE: u8 = 12;
pub const WANTED_MAJOR: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: i64,
    pub minor: i64,
}

pub const UNKNOWN: Version = Version { major: -1, minor: -1 };

pub fn version_of(xml: &str) -> Version {
    let Ok(document) = Document::parse(xml) else { return UNKNOWN };
    let number = |tag: &str| document.descendants().find(|n| n.has_tag_name(tag)).and_then(|n| n.text()).and_then(|t| t.trim().parse().ok()).unwrap_or(-1);
    Version { major: number("parameter_version_major"), minor: number("parameter_version_minor") }
}

pub fn cache_file_name(firmware: u8, major: i64) -> String {
    format!("{PREFIX}.{firmware}.{major}.xml")
}

pub fn major_from_name(firmware: u8, name: &str) -> Option<i64> {
    name.strip_prefix(&format!("{PREFIX}.{firmware}."))?.strip_suffix(".xml")?.parse().ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Cache { name: String, version: Version },
    Internal { version: Version },
}

fn cache_hit(firmware: u8, entries: &[CacheEntry]) -> Option<&CacheEntry> {
    let exact = entries.iter().find(|e| e.name == cache_file_name(firmware, WANTED_MAJOR)).filter(|e| e.version.major == WANTED_MAJOR);
    exact.or_else(|| {
        entries
            .iter()
            .filter_map(|e| major_from_name(firmware, &e.name).map(|m| (m, e)))
            .filter(|(m, _)| *m < WANTED_MAJOR)
            .max_by_key(|(m, _)| *m)
            .map(|(_, e)| e)
            .filter(|e| major_from_name(firmware, &e.name) == Some(e.version.major))
    })
}

pub fn choose(firmware: u8, entries: &[CacheEntry], internal: Version) -> Choice {
    let usable = cache_hit(firmware, entries).filter(|hit| match (internal.major == WANTED_MAJOR, hit.version.major == WANTED_MAJOR) {
        (true, true) => hit.version.minor > internal.minor,
        (true, false) => false,
        (false, wanted) => wanted,
    });
    match usable {
        Some(hit) => Choice::Cache { name: hit.name.clone(), version: hit.version },
        None => Choice::Internal { version: internal },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheAction {
    Store { as_name: String, replacing: Option<String> },
    Keep,
}

pub fn on_received(firmware: u8, received: Version, current: &Choice) -> CacheAction {
    let major = WANTED_MAJOR;
    let as_name = cache_file_name(firmware, major);
    match current {
        Choice::Cache { name, version } if version.major == major => {
            if received.minor >= version.minor { CacheAction::Store { as_name, replacing: Some(name.clone()) } } else { CacheAction::Keep }
        }
        Choice::Internal { version } if version.major == major && received.minor < version.minor => CacheAction::Keep,
        _ => CacheAction::Store { as_name, replacing: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, major: i64, minor: i64) -> CacheEntry {
        CacheEntry { name: name.to_string(), version: Version { major, minor } }
    }

    #[test]
    fn the_version_stamp_is_read_from_the_xml_or_reported_unknown() {
        assert_eq!(version_of("<parameters><version>3</version><parameter_version_major>1</parameter_version_major><parameter_version_minor>15</parameter_version_minor></parameters>"), Version { major: 1, minor: 15 });
        assert_eq!(version_of("<parameters><version>3</version></parameters>"), UNKNOWN);
        assert_eq!(version_of("not xml"), UNKNOWN);
        assert_eq!(version_of(crate::px4meta::BUNDLED).major, 1);
        assert_eq!(cache_file_name(PX4_FIRMWARE, 1), "ParameterFactMetaData.12.1.xml");
        assert_eq!(major_from_name(PX4_FIRMWARE, "ParameterFactMetaData.12.1.xml"), Some(1));
        assert_eq!(major_from_name(PX4_FIRMWARE, "ParameterFactMetaData.3.1.xml"), None);
    }

    #[test]
    fn a_newer_cached_minor_beats_the_bundled_file_and_nothing_else_does() {
        let bundled = Version { major: 1, minor: 15 };
        let newer = [entry("ParameterFactMetaData.12.1.xml", 1, 16)];
        assert_eq!(choose(PX4_FIRMWARE, &newer, bundled), Choice::Cache { name: "ParameterFactMetaData.12.1.xml".into(), version: Version { major: 1, minor: 16 } });
        let older = [entry("ParameterFactMetaData.12.1.xml", 1, 14)];
        assert_eq!(choose(PX4_FIRMWARE, &older, bundled), Choice::Internal { version: bundled });
        let corrupt = [entry("ParameterFactMetaData.12.1.xml", 2, 0)];
        assert_eq!(choose(PX4_FIRMWARE, &corrupt, bundled), Choice::Internal { version: bundled });
        assert_eq!(choose(PX4_FIRMWARE, &[], bundled), Choice::Internal { version: bundled });
        let unknown_internal = UNKNOWN;
        assert!(matches!(choose(PX4_FIRMWARE, &older, unknown_internal), Choice::Cache { .. }));
    }

    #[test]
    fn a_received_file_is_stored_unless_the_cache_already_has_a_newer_minor() {
        let received = Version { major: 1, minor: 16 };
        let none = Choice::Internal { version: Version { major: 1, minor: 15 } };
        assert_eq!(on_received(PX4_FIRMWARE, received, &none), CacheAction::Store { as_name: "ParameterFactMetaData.12.1.xml".into(), replacing: None });
        assert_eq!(on_received(PX4_FIRMWARE, Version { major: 1, minor: 14 }, &none), CacheAction::Keep);
        let same = Choice::Cache { name: "ParameterFactMetaData.12.1.xml".into(), version: Version { major: 1, minor: 16 } };
        assert_eq!(on_received(PX4_FIRMWARE, received, &same), CacheAction::Store { as_name: "ParameterFactMetaData.12.1.xml".into(), replacing: Some("ParameterFactMetaData.12.1.xml".into()) });
        let newer = Choice::Cache { name: "ParameterFactMetaData.12.1.xml".into(), version: Version { major: 1, minor: 17 } };
        assert_eq!(on_received(PX4_FIRMWARE, received, &newer), CacheAction::Keep);
    }
}
