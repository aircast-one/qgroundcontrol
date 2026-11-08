use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicle.aircastLink.quality",
    "vehicle.aircastLink.radioType",
    "vehicle.aircastLink.status",
    "vehicle.aircastLink.videoBitrate",
    "vehicle.aircastLink.uplink",
    "vehicle.aircastLink.uplinkSwitches",
];

const QUALITY_UNKNOWN: i64 = 255;
const UPLINK_CELLULAR: i64 = 3;
const UPLINKS: [(&str, &str); 5] = [
    ("none", "No uplink"),
    ("ethernet", "Ethernet"),
    ("wifi", "Wi-Fi"),
    ("cellular", "LTE"),
    ("starlink", "Starlink"),
];

fn uplink(code: i64) -> Option<(&'static str, &'static str)> {
    usize::try_from(code)
        .ok()
        .and_then(|i| UPLINKS.get(i))
        .copied()
}

fn uplink_label(code: Option<i64>) -> String {
    code.and_then(uplink)
        .map_or("Unknown".to_string(), |(_, label)| label.to_string())
}

fn clock(at_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(at_ms).map_or_else(String::new, |at| {
        at.with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string()
    })
}

fn switches(backend: &dyn Backend, group: &Value) -> Value {
    json!(
        history(backend, group, "uplinkSwitches")
            .as_array()
            .map(|switches| {
                switches
                    .iter()
                    .map(|s| {
                        let at = s.get("atMs").and_then(Value::as_i64).unwrap_or_default();
                        json!({
                            "atMs": at,
                            "clock": clock(at),
                            "from": uplink_label(s.get("from").and_then(Value::as_i64)),
                            "to": uplink_label(s.get("to").and_then(Value::as_i64)),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    )
}

pub fn cell_text(uplink_code: Option<i64>, quality: Option<i64>) -> String {
    let quality_text = quality.map_or("--".to_string(), |q| format!("{q}%"));
    match uplink_code.and_then(|code| uplink(code).map(|(_, label)| (code, label))) {
        None => quality_text,
        Some((UPLINK_CELLULAR, label)) => format!("{label} {quality_text}"),
        Some((_, label)) => label.to_string(),
    }
}

fn fact<'a>(group: &'a Value, name: &str) -> Option<&'a Value> {
    group.get("facts")?.as_array()?.iter().find(|fact| {
        fact.get("property")
            .or(fact.get("name"))
            .and_then(Value::as_str)
            == Some(name)
    })
}

fn raw(group: &Value, name: &str) -> Option<i64> {
    fact(group, name)
        .and_then(|f| f.get("rawValue").or(f.get("value")))
        .and_then(Value::as_f64)
        .map(|v| v as i64)
}

fn label(group: &Value, name: &str) -> String {
    fact(group, name)
        .and_then(|f| f.get("enumOrValueString").or(f.get("enumStringValue")))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn bitrate_text(kbps: i64) -> String {
    match kbps >= 1000 {
        true => format!("{:.1} Mbit/s", kbps as f64 / 1000.0),
        false => format!("{kbps} kbit/s"),
    }
}

fn history(backend: &dyn Backend, group: &Value, name: &str) -> Value {
    group
        .get(name)
        .cloned()
        .filter(Value::is_array)
        .unwrap_or_else(|| {
            backend
                .value(&format!("vehicle.aircastLink.{name}"))
                .get("value")
                .cloned()
                .unwrap_or(json!([]))
        })
}

pub fn aircast_link_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let group = backend.value("vehicle.aircastLink");
    let quality = raw(&group, "quality").filter(|q| *q != QUALITY_UNKNOWN);
    let bitrate = raw(&group, "videoBitrate").unwrap_or(0);
    let uplink_code = raw(&group, "uplink");
    let cellular = raw(&group, "quality").is_some();
    json!({
        "kind": "object",
        "class": "AircastLink",
        "shown": group.get("telemetryAvailable").and_then(Value::as_bool).unwrap_or(false),
        "quality": quality,
        "qualityText": quality.map_or("--".to_string(), |q| format!("{q}%")),
        "signalText": quality.map_or("unknown".to_string(), |q| format!("{q} %")),
        "network": label(&group, "radioType"),
        "modem": label(&group, "status"),
        "cellular": cellular,
        "bitrateText": if cellular { bitrate_text(bitrate) } else { String::new() },
        "qualityHistory": history(backend, &group, "qualityHistory"),
        "bitrateHistory": history(backend, &group, "bitrateHistory"),
        "uplink": uplink_code.and_then(uplink).map(|(kind, _)| kind),
        "uplinkText": uplink_code.and_then(uplink).map_or("", |(_, label)| label),
        "cellText": cell_text(uplink_code, quality),
        "switches": switches(backend, &group),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitrate_reads_in_kbit_until_a_megabit() {
        assert_eq!(bitrate_text(999), "999 kbit/s");
        assert_eq!(bitrate_text(2450), "2.5 Mbit/s");
    }

    #[test]
    fn cellular_status_fills_the_link_facts_and_its_history() {
        use mavlink::dialects::ardupilotmega::{CELLULAR_STATUS_DATA, MavMessage};
        let header = mavlink::MavHeader {
            system_id: 1,
            component_id: 1,
            sequence: 0,
        };
        let message = MavMessage::CELLULAR_STATUS(CELLULAR_STATUS_DATA {
            quality: 73,
            ..Default::default()
        });
        let mut raw = Vec::new();
        mavlink::write_v2_msg(&mut raw, header, &message).unwrap();
        let payload_length = usize::from(raw[1]);
        let extended: Vec<u8> = raw[..10 + payload_length]
            .iter()
            .copied()
            .chain(std::iter::repeat_n(0, 15 - payload_length))
            .chain(250u32.to_le_bytes())
            .collect();
        assert_eq!(crate::vehiclefact::cellular_rx_rate(&extended), 250);
        assert_eq!(
            crate::vehiclefact::cellular_rx_rate(&raw),
            0,
            "a frame without the extension reads as no rate"
        );
        let MavMessage::CELLULAR_STATUS(cellular) = &message else {
            unreachable!()
        };
        let mut facts = crate::vehiclefact::AircastLinkFacts::default();
        facts.apply(cellular, 250);
        facts.apply(
            &CELLULAR_STATUS_DATA {
                quality: 255,
                ..*cellular
            },
            u32::MAX,
        );
        assert_eq!(
            (
                facts.bitrate_kbps,
                facts.quality_history.iter().copied().collect::<Vec<_>>(),
                facts.bitrate_history.iter().copied().collect::<Vec<_>>()
            ),
            (0, vec![73, -1], vec![2048, 0])
        );
        let group = facts.group();
        assert_eq!(
            (
                raw_of(&group, "quality"),
                group["telemetryAvailable"].clone(),
                group["qualityHistory"].clone()
            ),
            (Some(255), json!(true), json!([73, -1]))
        );
    }

    fn raw_of(group: &Value, name: &str) -> Option<i64> {
        raw(group, name)
    }

    struct Link(crate::vehiclefact::AircastLinkFacts);

    impl Backend for Link {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.aircastLink" => self.0.group().to_string(),
                _ => "null".to_string(),
            }
        }
        fn get_fields(&self, _: &str, _: &str) -> String {
            "null".to_string()
        }
        fn set(&self, _: &str, _: &str) -> String {
            "null".to_string()
        }
        fn invoke(&self, _: &str, _: &str) -> String {
            "null".to_string()
        }
        fn watch(&self, _: &[String]) {}
    }

    #[test]
    fn the_uplink_is_remembered_with_every_change_newest_first() {
        let mut facts = crate::vehiclefact::AircastLinkFacts::default();
        facts.note_uplink(4, 1_000);
        facts.note_uplink(4, 2_000);
        facts.note_uplink(3, 3_000);
        facts.apply(&Default::default(), 0);
        facts.note_uplink(4, 4_000);
        assert_eq!(facts.uplink, Some(4));
        assert_eq!(
            facts
                .uplink_switches
                .iter()
                .map(|s| (s.at_ms, s.from, s.to))
                .collect::<Vec<_>>(),
            vec![(4_000, 3, 4), (3_000, 4, 3)],
            "a repeat is not a switch, the first value is not a switch, and a cellular report keeps the history"
        );
        (0..30).for_each(|i| facts.note_uplink(i % 2 + 2, 10_000 + i64::from(i)));
        assert_eq!(facts.uplink_switches.len(), 5);
    }

    #[test]
    fn the_cell_names_the_link_and_keeps_the_signal_only_for_lte() {
        assert_eq!(cell_text(Some(4), Some(80)), "Starlink");
        assert_eq!(cell_text(Some(2), None), "Wi-Fi");
        assert_eq!(cell_text(Some(3), Some(87)), "LTE 87%");
        assert_eq!(cell_text(Some(0), None), "No uplink");
        assert_eq!(
            cell_text(None, Some(87)),
            "87%",
            "an aircastd that never says keeps today's cell"
        );
    }

    #[test]
    fn the_view_shows_an_aircraft_on_wi_fi_without_a_modem() {
        let mut facts = crate::vehiclefact::AircastLinkFacts::default();
        facts.note_uplink(2, 1_000);
        facts.note_uplink(3, 1_794_149_670_000);
        let view = aircast_link_view(&Link(facts), &[]);
        assert_eq!(
            (
                view["shown"].clone(),
                view["uplink"].clone(),
                view["uplinkText"].clone(),
                view["cellText"].clone()
            ),
            (
                json!(true),
                json!("cellular"),
                json!("LTE"),
                json!("LTE --")
            )
        );
        assert_eq!(
            (view["cellular"].clone(), view["bitrateText"].clone()),
            (json!(false), json!("")),
            "no modem report yet: no signal, no bitrate to show"
        );
        assert_eq!(view["switches"][0]["from"], json!("Wi-Fi"));
        assert_eq!(view["switches"][0]["to"], json!("LTE"));
        assert_eq!(view["switches"][0]["atMs"], json!(1_794_149_670_000_i64));
        assert_eq!(view["switches"][0]["clock"].as_str().map(str::len), Some(8));

        let old = aircast_link_view(&Link(Default::default()), &[]);
        assert_eq!(
            (old["uplink"].clone(), old["switches"].clone()),
            (Value::Null, json!([]))
        );
    }
}
