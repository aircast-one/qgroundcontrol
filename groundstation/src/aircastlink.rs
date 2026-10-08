use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.aircastLink.quality", "vehicle.aircastLink.radioType", "vehicle.aircastLink.status", "vehicle.aircastLink.videoBitrate"];

const QUALITY_UNKNOWN: i64 = 255;

fn fact<'a>(group: &'a Value, name: &str) -> Option<&'a Value> {
    group.get("facts")?.as_array()?.iter().find(|fact| fact.get("property").or(fact.get("name")).and_then(Value::as_str) == Some(name))
}

fn raw(group: &Value, name: &str) -> Option<i64> {
    fact(group, name).and_then(|f| f.get("rawValue").or(f.get("value"))).and_then(Value::as_f64).map(|v| v as i64)
}

fn label(group: &Value, name: &str) -> String {
    fact(group, name).and_then(|f| f.get("enumOrValueString").or(f.get("enumStringValue"))).and_then(Value::as_str).unwrap_or_default().to_string()
}

pub fn bitrate_text(kbps: i64) -> String {
    match kbps >= 1000 {
        true => format!("{:.1} Mbit/s", kbps as f64 / 1000.0),
        false => format!("{kbps} kbit/s"),
    }
}

fn history(backend: &dyn Backend, group: &Value, name: &str) -> Value {
    group.get(name).cloned().filter(Value::is_array).unwrap_or_else(|| backend.value(&format!("vehicle.aircastLink.{name}")).get("value").cloned().unwrap_or(json!([])))
}

pub fn aircast_link_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let group = backend.value("vehicle.aircastLink");
    let quality = raw(&group, "quality").filter(|q| *q != QUALITY_UNKNOWN);
    let bitrate = raw(&group, "videoBitrate").unwrap_or(0);
    json!({
        "kind": "object",
        "class": "AircastLink",
        "shown": group.get("telemetryAvailable").and_then(Value::as_bool).unwrap_or(false),
        "quality": quality,
        "qualityText": quality.map_or("--".to_string(), |q| format!("{q}%")),
        "signalText": quality.map_or("unknown".to_string(), |q| format!("{q} %")),
        "network": label(&group, "radioType"),
        "modem": label(&group, "status"),
        "bitrateText": bitrate_text(bitrate),
        "qualityHistory": history(backend, &group, "qualityHistory"),
        "bitrateHistory": history(backend, &group, "bitrateHistory"),
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
        let header = mavlink::MavHeader { system_id: 1, component_id: 1, sequence: 0 };
        let message = MavMessage::CELLULAR_STATUS(CELLULAR_STATUS_DATA { quality: 73, ..Default::default() });
        let mut raw = Vec::new();
        mavlink::write_v2_msg(&mut raw, header, &message).unwrap();
        let payload_length = usize::from(raw[1]);
        let extended: Vec<u8> = raw[..10 + payload_length].iter().copied().chain(std::iter::repeat_n(0, 15 - payload_length)).chain(250u32.to_le_bytes()).collect();
        assert_eq!(crate::vehiclefact::cellular_rx_rate(&extended), 250);
        assert_eq!(crate::vehiclefact::cellular_rx_rate(&raw), 0, "a frame without the extension reads as no rate");
        let MavMessage::CELLULAR_STATUS(cellular) = &message else { unreachable!() };
        let mut facts = crate::vehiclefact::AircastLinkFacts::default();
        facts.apply(cellular, 250);
        facts.apply(&CELLULAR_STATUS_DATA { quality: 255, ..*cellular }, u32::MAX);
        assert_eq!((facts.bitrate_kbps, facts.quality_history.iter().copied().collect::<Vec<_>>(), facts.bitrate_history.iter().copied().collect::<Vec<_>>()), (0, vec![73, -1], vec![2048, 0]));
        let group = facts.group();
        assert_eq!((raw_of(&group, "quality"), group["telemetryAvailable"].clone(), group["qualityHistory"].clone()), (Some(255), json!(true), json!([73, -1])));
    }

    fn raw_of(group: &Value, name: &str) -> Option<i64> {
        raw(group, name)
    }
}
