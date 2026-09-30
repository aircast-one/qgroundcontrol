use serde_json::{Value, json};

use crate::control::decode;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];

struct Page {
    title: &'static str,
    sections: &'static [(&'static str, &'static str)],
    shows_links: bool,
    shows_about: bool,
    shows_video_sources: bool,
    shows_packet_radio: bool,
}

const PAGES: &[Page] = &[
    Page { title: "General", sections: &[("Application", "appSettings"), ("Units", "unitsSettings"), ("Brand Image", "brandImageSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Fly View", sections: &[("Fly View", "flyViewSettings"), ("Battery Indicator", "batteryIndicatorSettings"), ("Gimbal Controller", "gimbalControllerSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Plan View", sections: &[("Plan View", "planViewSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Video", sections: &[("Video", "videoSettings")], shows_links: false, shows_about: false, shows_video_sources: true, shows_packet_radio: false },
    Page { title: "Maps", sections: &[("Maps", "mapsSettings"), ("Flight Map", "flightMapSettings"), ("Offline Maps", "offlineMapsSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Connections", sections: &[("Auto Connect", "autoConnectSettings")], shows_links: true, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "MAVLink", sections: &[("MAVLink", "mavlinkSettings"), ("APM Stream Rates", "apmMavlinkStreamRateSettings"), ("Actions", "mavlinkActionsSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Flight Modes", sections: &[("Flight Modes", "flightModeSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "ADSB Server", sections: &[("ADSB Server", "adsbVehicleManagerSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Packet Radio", sections: &[("Packet Radio", "packetRadioSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: true },
    Page { title: "Remote ID", sections: &[("Remote ID", "remoteIDSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "RTK GPS", sections: &[("RTK GPS", "rtkSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "Firmware Upgrade", sections: &[("Firmware Upgrade", "firmwareUpgradeSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "3D Viewer", sections: &[("3D Viewer", "viewer3DSettings")], shows_links: false, shows_about: false, shows_video_sources: false, shows_packet_radio: false },
    Page { title: "About", sections: &[], shows_links: false, shows_about: true, shows_video_sources: false, shows_packet_radio: false },
];

// deviceName is drawn by the Packet Radio page's own block as a picker over the adapters the
// radio reports. Left in the generic list it renders a second control writing the same fact - a
// free-text field beside the picker, where a name that is not an adapter gets no feedback at all.
// Same shape as extraVideoSources: when a bespoke block owns a fact, the fact leaves the list.
const HIDDEN: &[&str] = &["firstRunPromptIdsShown", "instrumentQmlFile2", "deviceName"];
const DESKTOP_ONLY: &[(&str, &str)] = &[("rcControls", "on-screen RC controls"), ("extraVideoSources", "additional cameras")];

const CHECKLIST_OFF: &str = "Has no effect while the preflight checklist is off.";

const GATED: &[(&str, &str, &str)] = &[("enforceChecklist", "useChecklist", CHECKLIST_OFF)];

// QGC distinguishes the two, and which binding a page uses is what decides this. FlyViewSettings
// binds the checklist row's `enabled`, so that control is real but inert and says why. The RTK
// page binds `visible` on both groups, because GPSIndicatorPage.qml:113-120 is a Survey-In /
// Specify position radio pair - the two sets are alternatives under a mode selector, not a switch
// with dependents. Four greyed latitude boxes under a Survey-In selection would be a control
// disabled with nothing on screen saying what it is disabled FOR.
const HIDDEN_WHEN: &[(&str, &str, bool)] = &[
    ("surveyInAccuracyLimit", "useFixedBasePosition", true),
    ("surveyInMinObservationDuration", "useFixedBasePosition", true),
    ("fixedBasePositionLatitude", "useFixedBasePosition", false),
    ("fixedBasePositionLongitude", "useFixedBasePosition", false),
    ("fixedBasePositionAltitude", "useFixedBasePosition", false),
    ("fixedBasePositionAccuracy", "useFixedBasePosition", false),
];

const SUBSECTIONS: &[(&str, &[(&str, &[&str])])] = &[
    ("appSettings", &[
        ("Appearance", &["indoorPalette", "appFontPointSize", "overlayGlassFrost", "qLocaleLanguage"]),
        ("Sound", &["audioMuted", "batteryPercentRemainingAnnounce"]),
        ("Preflight checklist", &["useChecklist", "enforceChecklist"]),
        ("Virtual joystick", &["virtualJoystick", "virtualJoystickAutoCenterThrottle", "virtualJoystickLeftHandedMode"]),
        ("Planning defaults", &["defaultMissionItemAltitude", "offlineEditingFirmwareClass", "offlineEditingVehicleClass", "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed", "offlineEditingAscentSpeed", "offlineEditingDescentSpeed"]),
        ("Map providers", &["mapboxToken", "mapboxAccount", "mapboxStyle", "esriToken", "vworldToken", "customURL"]),
        ("AirLink", &["loginAirLink", "passAirLink"]),
        ("Files", &["savePath", "androidSaveToSDCard", "disableAllPersistence"]),
    ]),
    ("videoSettings", &[
        ("Cameras", &["videoSource", "primaryCameraName", "activeVideoSource", "multiViewEnabled"]),
        ("Stream", &["udpUrl", "rtspUrl", "tcpUrl", "whepUrl", "rtspTimeout", "streamEnabled", "disableWhenDisarmed", "lowLatencyMode", "forceVideoDecoder"]),
        ("Display", &["videoFit", "aspectRatio", "gridLines", "showRecControl"]),
        ("Recording", &["videoSavePath", "recordingFormat", "enableStorageLimit", "maxVideoSize"]),
    ]),
    ("flyViewSettings", &[
        ("Guided commands", &["guidedMinimumAltitude", "guidedMaximumAltitude", "maxGoToLocationDistance", "forwardFlightGoToLocationLoiterRad", "goToLocationRequiresConfirmInGuided", "updateHomePosition"]),
        ("Map and compass", &["keepMapCenteredOnVehicle", "showAdditionalIndicatorsCompass", "lockNoseUpCompass", "showObstacleDistanceOverlay"]),
        ("On-screen controls", &["showPhotoVideoControl", "showSimpleCameraControl", "showLogReplayStatusBar"]),
        ("Camera and gimbal channels", &["gimbalTiltChannel", "gimbalPanChannel", "cameraZoomChannel", "cameraLightChannel", "cameraRecordChannel"]),
        ("Sharing control", &["requestControlAllowTakeover", "requestControlTimeout"]),
    ]),
];

fn page_json(page: &Page, with_controls: Option<&dyn Backend>) -> Value {
    json!({
        "title": page.title,
        "showsLinks": page.shows_links,
        "showsAbout": page.shows_about,
        "showsVideoSources": page.shows_video_sources,
        "showsPacketRadio": page.shows_packet_radio,
        "sections": page.sections.iter().map(|(title, group)| section_json(title, group, with_controls)).collect::<Vec<_>>(),
    })
}

fn section_json(title: &str, group: &str, backend: Option<&dyn Backend>) -> Value {
    let path = format!("settings.{group}");
    let Some(backend) = backend else { return json!({ "title": title, "group": group, "path": path }) };
    let facts: Vec<Value> = object(&backend.get(&path)).get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
    let shown: Vec<Value> = facts
        .iter()
        .filter(|f| f.get("visible").and_then(Value::as_bool) != Some(false))
        .filter(|f| {
            let named = f.get("name").and_then(Value::as_str).unwrap_or_default();
            !HIDDEN_WHEN.iter().any(|(hidden, requires, when)| {
                *hidden == named && facts.iter().find(|other| other.get("name").and_then(Value::as_str) == Some(requires)).and_then(|other| other.get("value")).and_then(Value::as_bool) == Some(*when)
            })
        })
        .filter(|f| f.get("name").and_then(Value::as_str).is_some_and(|n| !HIDDEN.contains(&n) && !DESKTOP_ONLY.iter().any(|(d, _)| *d == n)))
        .map(|f| decode(f, &format!("{path}.{}", f.get("name").and_then(Value::as_str).unwrap_or(""))))
        .collect();
    let shown = gated(&shown, &facts);
    let desktop_only: Vec<&str> = facts.iter().filter_map(|f| f.get("name").and_then(Value::as_str)).filter_map(|n| DESKTOP_ONLY.iter().find(|(d, _)| *d == n).map(|(_, label)| *label)).collect();
    let note = match desktop_only.is_empty() {
        true => String::new(),
        false => format!("Set up {} on the desktop - they are stored as JSON that is not editable here.", desktop_only.join(" and ")),
    };
    json!({
        "title": title,
        "group": group,
        "path": path,
        "note": note,
        "subsections": subsections(group, &named_apart(shown)),
    })
}

fn gated(controls: &[Value], facts: &[Value]) -> Vec<Value> {
    let value_of = |name: &str| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)).and_then(|f| f.get("value").cloned());
    controls
        .iter()
        .map(|control| {
            let name = control.get("name").and_then(Value::as_str).unwrap_or("");
            let blocked = GATED
                .iter()
                .find(|(gated, _, _)| *gated == name)
                .filter(|(_, requires, _)| value_of(requires).as_ref().and_then(Value::as_bool) == Some(false))
                .map(|(_, _, reason)| *reason);
            let mut with_gate = control.clone();
            with_gate["enabled"] = json!(blocked.is_none());
            with_gate["disabledReason"] = blocked.map_or(Value::Null, |reason| json!(reason));
            with_gate
        })
        .collect()
}

// Two settings can carry the same shortDesc - the brand image pair differ only in a longDesc
// nobody shows - and a section then draws two identical rows holding different values. The name
// is the only thing that always differs, so a repeated label falls back to it.
fn named_apart(controls: Vec<Value>) -> Vec<Value> {
    let label_of = |c: &Value| c.get("label").and_then(Value::as_str).unwrap_or("").to_string();
    let repeated = |label: &str| controls.iter().filter(|c| label_of(c) == label).count() > 1;
    controls
        .iter()
        .map(|control| match repeated(&label_of(control)) {
            false => control.clone(),
            true => {
                let mut apart = control.clone();
                apart["label"] = json!(crate::label::humanise(control.get("name").and_then(Value::as_str).unwrap_or("")));
                apart
            }
        })
        .collect()
}

fn subsections(group: &str, controls: &[Value]) -> Vec<Value> {
    let name_of = |c: &Value| c.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let Some((_, table)) = SUBSECTIONS.iter().find(|(g, _)| *g == group) else {
        return vec![json!({ "title": "", "controls": controls })];
    };
    let named: Vec<Value> = table
        .iter()
        .map(|(title, names)| json!({ "title": title, "controls": names.iter().filter_map(|n| controls.iter().find(|c| name_of(c) == *n)).cloned().collect::<Vec<_>>() }))
        .filter(|s| !s["controls"].as_array().unwrap().is_empty())
        .collect();
    let claimed: Vec<&str> = table.iter().flat_map(|(_, names)| names.iter().copied()).collect();
    let leftovers: Vec<Value> = controls.iter().filter(|c| !claimed.contains(&name_of(c).as_str())).cloned().collect();
    match leftovers.is_empty() {
        true => named,
        false => named.into_iter().chain(std::iter::once(json!({ "title": "Other", "controls": leftovers }))).collect(),
    }
}

pub fn settings_view(backend: &dyn Backend, args: &[String]) -> Value {
    match args.first() {
        Some(title) => PAGES.iter().find(|p| p.title == title).map(|p| page_json(p, Some(backend))).unwrap_or(json!({ "kind": "null" })),
        None => json!({ "kind": "object", "class": "Settings", "pages": PAGES.iter().map(|p| page_json(p, None)).collect::<Vec<_>>() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.appSettings" => json!({ "kind": "object", "facts": [
                    { "kind": "fact", "name": "audioMuted", "typeIsBool": true, "value": false },
                    { "kind": "fact", "name": "useChecklist", "typeIsBool": true, "value": true },
                    { "kind": "fact", "name": "firstRunPromptIdsShown", "typeIsString": true },
                    { "kind": "fact", "name": "rcControls", "typeIsString": true },
                    { "kind": "fact", "name": "somethingNew", "typeIsString": true },
                ] }),
                _ => json!({ "kind": "object", "facts": [ { "kind": "fact", "name": "verticalDistanceUnits", "enumStrings": ["Feet", "Meters"], "enumIndex": 1 } ] }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn every_name_these_tables_key_on_exists_and_belongs_to_one_group() {
        const GROUPS: &[(&str, &str)] = &[
            ("App", include_str!("../../src/Settings/App.SettingsGroup.json")),
            ("Video", include_str!("../../src/Settings/Video.SettingsGroup.json")),
            ("RTK", include_str!("../../src/Settings/RTK.SettingsGroup.json")),
            ("FlyView", include_str!("../../src/Settings/FlyView.SettingsGroup.json")),
            ("PacketRadio", include_str!("../../src/Settings/PacketRadio.SettingsGroup.json")),
            ("Viewer3D", include_str!("../../src/Settings/Viewer3D.SettingsGroup.json")),
        ];
        let declares = |name: &str| -> Vec<&str> {
            GROUPS.iter().filter(|(_, body)| body.contains(&format!("\"{name}\""))).map(|(group, _)| *group).collect()
        };
        assert_eq!(declares("useFixedBasePosition"), vec!["RTK"], "the parser has to find a name it should, or every assertion below passes by finding nothing");
        assert_eq!(declares("enabled").len(), 2, "enabled is declared in two groups and that is what makes uniqueness worth asserting rather than assumed");

        let keyed: Vec<&str> = HIDDEN
            .iter()
            .copied()
            .chain(DESKTOP_ONLY.iter().map(|(name, _)| *name))
            .chain(GATED.iter().flat_map(|(gated, requires, _)| [*gated, *requires]))
            .chain(HIDDEN_WHEN.iter().flat_map(|(hidden, requires, _)| [*hidden, *requires]))
            .collect();

        keyed.iter().for_each(|name| {
            let groups = declares(name);
            assert!(
                !groups.is_empty(),
                "{name} is keyed on by one of these tables and no settings group declares it. The entry stops matching silently, and every one of these fails OPEN - a stale HIDDEN or DESKTOP_ONLY makes a control reappear on a page it was deliberately kept off, a stale GATED turns the enforce-checklist switch back into a live one, and a stale HIDDEN_WHEN puts both RTK mode groups back on screen at once. The page looks fuller and nothing fails"
            );
            assert_eq!(
                groups.len(),
                1,
                "{name} is declared in {groups:?}. These tables key on a BARE fact name and the filters run per group, so an entry naming a shared name applies to every page that declares it - one line that looks like it names one control silently reaching two"
            );
        });
    }

    #[test]
    fn the_rtk_page_shows_one_mode_at_a_time_rather_than_both_sets_at_once() {
        struct Rtk(Option<bool>);
        impl Backend for Rtk {
            fn get(&self, path: &str) -> String {
                let mode = self.0.map(|on| json!({ "kind": "fact", "name": "useFixedBasePosition", "typeIsBool": true, "value": on }));
                let facts: Vec<Value> = mode
                    .into_iter()
                    .chain(["surveyInAccuracyLimit", "surveyInMinObservationDuration", "fixedBasePositionLatitude", "fixedBasePositionLongitude", "fixedBasePositionAltitude", "fixedBasePositionAccuracy"].iter().map(|n| json!({ "kind": "fact", "name": n, "typeIsString": true })))
                    .collect();
                match path {
                    "settings.rtkSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let shown = |fixed: Option<bool>| -> Vec<String> {
            settings_view(&Rtk(fixed), &["RTK GPS".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .filter_map(|c| c["name"].as_str().map(str::to_string))
                .collect()
        };

        let surveying = shown(Some(false));
        assert!(surveying.contains(&"surveyInAccuracyLimit".to_string()) && surveying.contains(&"surveyInMinObservationDuration".to_string()));
        assert!(
            !surveying.iter().any(|n| n.starts_with("fixedBasePosition")),
            "GPSIndicatorPage.qml binds visible on all six, and the selector above them is a Survey-In / Specify position radio pair - so a head drawing four editable base-position boxes under a Survey-In selection is offering the mode the operator did not choose. Shown: {surveying:?}"
        );

        let specified = shown(Some(true));
        assert_eq!(specified.iter().filter(|n| n.starts_with("fixedBasePosition")).count(), 4);
        assert!(!specified.iter().any(|n| n.starts_with("surveyIn")), "and the mirror: choosing a fixed position puts the survey-in limits away rather than leaving both sets on the page");

        assert_eq!(
            shown(None).len(),
            6,
            "with no useFixedBasePosition in the payload nothing is hidden. A bridge that cannot answer which mode is selected must not empty the page - the same permissive rule as the visible flag, and the direction that fails safe"
        );
    }

    #[test]
    fn a_setting_qgc_hides_on_this_platform_is_not_offered_as_an_editable_row() {
        struct Group(Vec<Value>);
        impl Backend for Group {
            fn get(&self, path: &str) -> String {
                match path {
                    "settings.appSettings" => json!({ "kind": "object", "facts": self.0 }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let names = |facts: Vec<Value>| -> Vec<String> {
            settings_view(&Group(facts), &["General".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .filter_map(|c| c["name"].as_str().map(str::to_string))
                .collect()
        };
        let fact = |name: &str, visible: Option<bool>| {
            let mut f = json!({ "kind": "fact", "name": name, "typeIsString": true });
            visible.into_iter().for_each(|v| { f["visible"] = json!(v); });
            f
        };

        assert_eq!(
            names(vec![fact("savePath", Some(false)), fact("audioMuted", Some(true))]),
            vec!["audioMuted"],
            "AppSettings.cc:134 calls setVisible(false) on savePath under Q_OS_ANDROID, and :102 then hardcodes userHasModifiedSavePath = false so the runtime path overwrites whatever was stored. Serving the row let an operator set a save directory, watch the fact read it back, and find it silently reverted on the next launch - the write succeeded and was undone with nothing said"
        );

        assert_eq!(
            names(vec![fact("audioMuted", None)]),
            vec!["audioMuted"],
            "absent must mean visible. Every fact served by a bridge without this property in its allowlist arrives with no visible key, and a filter reading absence as hidden empties every settings page at once - which is how this whole class of field goes wrong in the safe-looking direction"
        );

        assert_eq!(names(vec![fact("savePath", Some(true))]), vec!["savePath"], "and a fact QGC does show is shown");
    }

    #[test]
    fn enforcing_a_checklist_nobody_is_using_is_offered_as_a_switch_that_does_nothing() {
        struct Checklist(Option<bool>);
        impl Backend for Checklist {
            fn get(&self, path: &str) -> String {
                let facts: Vec<Value> = [
                    Some(json!({ "kind": "fact", "name": "enforceChecklist", "typeIsBool": true, "value": true })),
                    self.0.map(|on| json!({ "kind": "fact", "name": "useChecklist", "typeIsBool": true, "value": on })),
                ]
                .into_iter()
                .flatten()
                .collect();
                match path {
                    "settings.appSettings" => json!({ "kind": "object", "facts": facts }),
                    _ => json!({ "kind": "object", "facts": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let enforce = |using: Option<bool>| {
            settings_view(&Checklist(using), &["General".to_string()])["sections"][0]["subsections"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sub| sub["controls"].as_array().unwrap().clone())
                .find(|c| c["name"] == "enforceChecklist")
                .expect("the control has to be in the section at all, or every assertion below passes by not finding it")
        };

        let on = enforce(Some(true));
        assert_eq!(on["enabled"], true);
        assert_eq!(on["disabledReason"], Value::Null);

        let off = enforce(Some(false));
        assert_eq!(off["enabled"], false, "FlyViewSettings.qml binds this row's enabled to useChecklist.value and GuidedActionsController ANDs the two, so with the checklist off the switch is live, writes a value, and changes nothing anywhere");
        assert_eq!(off["disabledReason"], CHECKLIST_OFF, "the reason states what is true of the setting rather than telling the operator which switch to find - the head owns the call to action and phrases it where the other switch actually is");
        assert_eq!(off["readOnly"], on["readOnly"], "readOnly is the Fact's own property and means the settings file said so; borrowing it for this would tell a head the setting cannot be written when it can");

        let absent = enforce(None);
        assert_eq!(absent["enabled"], true, "a gate whose required setting is not in the payload leaves the control alone, and this case is asserted rather than left to a fixture that happens to omit the key: a rule that disables on absence would grey out every row the moment a group is read with a field list that excludes its gate");
    }

    #[test]
    fn exactly_one_page_carries_the_packet_radio_block() {
        let flagged: Vec<&str> = PAGES.iter().filter(|page| page.shows_packet_radio).map(|page| page.title).collect();
        assert_eq!(flagged, vec!["Packet Radio"], "a head draws its bespoke block from this flag, so a second page carrying it draws the radio twice and none carrying it draws the settings with no status line at all");
        let bespoke: Vec<&str> = PAGES
            .iter()
            .filter(|page| [page.shows_links, page.shows_about, page.shows_video_sources, page.shows_packet_radio].iter().filter(|on| **on).count() > 1)
            .map(|page| page.title)
            .collect();
        assert!(bespoke.is_empty(), "these pages claim more than one bespoke block and a head has one slot for it: {bespoke:?}");
    }

    #[test]
    fn the_page_list_carries_no_controls_and_the_page_carries_decoded_ones() {
        let list = settings_view(&Fake, &[]);
        assert_eq!(list["pages"].as_array().unwrap().len(), 15);
        assert!(list["pages"][0]["sections"][0].get("subsections").is_none());
        let general = settings_view(&Fake, &["General".to_string()]);
        let app = &general["sections"][0];
        assert_eq!(app["path"], "settings.appSettings");
        let subs = app["subsections"].as_array().unwrap();
        assert_eq!(subs[0]["title"], "Sound");
        assert_eq!(subs[0]["controls"][0]["control"], "toggle");
        assert_eq!(subs[1]["title"], "Preflight checklist");
        assert_eq!(subs.last().unwrap()["title"], "Other");
        assert_eq!(subs.last().unwrap()["controls"][0]["name"], "somethingNew");
        assert!(app["note"].as_str().unwrap().contains("on-screen RC controls"));
        let units = &general["sections"][1];
        assert_eq!(units["subsections"][0]["title"], "");
        assert_eq!(units["subsections"][0]["controls"][0]["control"], "choice");
        assert_eq!(settings_view(&Fake, &["Nope".to_string()])["kind"], "null");
    }

    #[test]
    fn two_settings_that_share_a_description_are_told_apart_by_name() {
        let same = |name: &str| json!({ "kind": "object", "name": name, "label": "User-selected brand image" });
        let apart = named_apart(vec![same("userBrandImageIndoor"), same("userBrandImageOutdoor")]);
        assert_eq!(apart[0]["label"], "User Brand Image Indoor");
        assert_eq!(apart[1]["label"], "User Brand Image Outdoor", "two rows reading the same thing and holding different values cannot be told apart at all");

        let single = named_apart(vec![json!({ "name": "audioMuted", "label": "Mute audio output" })]);
        assert_eq!(single[0]["label"], "Mute audio output", "a label nothing else claims is the one the metadata wrote");
    }
}
