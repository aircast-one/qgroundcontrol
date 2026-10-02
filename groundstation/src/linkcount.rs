use std::collections::BTreeMap;

const STATUS_EVERY: u64 = 31;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LinkCount {
    pub received: u64,
    pub lost: u64,
    pub running_percent: f32,
    last: BTreeMap<(u8, u8), u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LinkStatus {
    pub sent: u64,
    pub received: u64,
    pub lost: u64,
    pub percent: f32,
}

impl LinkCount {
    pub fn counted(&self, system: u8, component: u8, sequence: u8) -> LinkCount {
        let received = self.received + 1;
        let last = self.last.get(&(system, component)).copied();
        if last == Some(sequence) {
            return LinkCount { received, ..self.clone() };
        }
        let expected = last.map_or(sequence, |l| l.wrapping_add(1));
        let lost = self.lost + u64::from(sequence.wrapping_sub(expected));
        let current = lost as f64 / (received + lost) as f64 * 100.0;
        LinkCount {
            received,
            lost,
            running_percent: ((current as f32) + self.running_percent) * 0.5,
            last: self.last.iter().map(|(k, v)| (*k, *v)).chain(std::iter::once(((system, component), sequence))).collect(),
        }
    }

    pub fn status(&self) -> Option<LinkStatus> {
        (self.received % STATUS_EVERY == 0).then_some(LinkStatus { sent: self.received + self.lost, received: self.received, lost: self.lost, percent: self.running_percent })
    }
}

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.mavlinkSentCount", "vehicle.mavlinkReceivedCount", "vehicle.mavlinkLossCount", "vehicle.mavlinkLossPercent"];

fn active_signing() -> crate::signing::Status {
    let link = crate::hub::lock().active().map(|v| v.link);
    link.map(|link| crate::signing::lock().status(link)).unwrap_or_default()
}

fn signing_rows(status: &crate::signing::Status) -> Vec<(&'static str, String)> {
    let text = match status.state {
        "enabling" => "Configuring\u{2026}",
        "disabling" => "Disabling\u{2026}",
        "on" => "On",
        _ => "Off",
    };
    let enabled = matches!(status.state, "on" | "disabling");
    let shown = [("Signing key", if status.key_name.is_empty() { "None".to_string() } else { status.key_name.clone() }), ("Signing streams", status.stream_count.to_string())];
    std::iter::once(("Signing", text.to_string())).chain(shown.into_iter().filter(|_| enabled)).collect()
}

pub fn link_status_view(backend: &dyn crate::router::Backend, _args: &[String]) -> serde_json::Value {
    use crate::read::{flag, object};
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let fields = object(&backend.get_fields("vehicle", "mavlinkSentCount,mavlinkReceivedCount,mavlinkLossCount,mavlinkLossPercent"));
    let whole = |key: &str| fields.get(key).and_then(serde_json::Value::as_f64).map_or("0".to_string(), |v| format!("{}", v as u64));
    let percent = fields.get("mavlinkLossPercent").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    serde_json::json!({
        "kind": "object",
        "class": "LinkStatus",
        "connected": connected,
        "rows": match connected {
            true => [("Total messages sent (computed)", whole("mavlinkSentCount")), ("Total messages received", whole("mavlinkReceivedCount")), ("Total message loss", whole("mavlinkLossCount")), ("Loss rate", format!("{percent:.0}%"))]
                .into_iter()
                .chain(signing_rows(&active_signing()))
                .collect::<Vec<_>>(),
            false => Vec::new(),
        }.into_iter().map(|(label, value)| serde_json::json!({ "label": label, "value": value })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_reads_as_mavlink_link_status_spells_it() {
        let status = |state: &'static str, key: &str| crate::signing::Status { state, key_name: key.to_string(), stream_count: 2 };
        assert_eq!(signing_rows(&status("off", "")), vec![("Signing", "Off".to_string())]);
        assert_eq!(signing_rows(&status("enabling", "field")), vec![("Signing", "Configuring\u{2026}".to_string())]);
        assert_eq!(signing_rows(&status("on", "field")), vec![("Signing", "On".to_string()), ("Signing key", "field".to_string()), ("Signing streams", "2".to_string())]);
        assert_eq!(signing_rows(&status("disabling", "")).len(), 3, "SigningController counts Disabling as enabled until the vehicle confirms");
    }

    #[test]
    fn gaps_in_the_sequence_count_as_lost_and_a_repeat_does_not() {
        let start = LinkCount::default().counted(1, 1, 250);
        let after = [251u8, 253, 253, 2].iter().fold(start, |count, seq| count.counted(1, 1, *seq));
        assert_eq!((after.received, after.lost), (5, 1 + 4), "252 then 254, 255, 0, 1 were never heard");
        assert_eq!(LinkCount::default().counted(1, 1, 9).counted(1, 2, 40).lost, 0, "each component keeps its own sequence");
    }

    #[test]
    fn status_is_reported_every_thirty_one_messages() {
        let count = (0..31u8).fold(LinkCount::default(), |c, s| c.counted(1, 1, s));
        assert_eq!(count.status().map(|s| (s.sent, s.received, s.lost)), Some((31, 31, 0)));
        assert_eq!(count.counted(1, 1, 31).status(), None);
    }
}
