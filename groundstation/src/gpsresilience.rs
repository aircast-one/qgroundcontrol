use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

const INTEGRITY_FACTS: [&str; 3] = ["jammingState", "spoofingState", "authenticationState"];
const GROUPS: [(&str, &str); 3] = [("vehicle.gpsAggregate", "GPS Resilience Status"), ("vehicle.gps", "GPS 1 Details"), ("vehicle.gps2", "GPS 2 Details")];
const UNKNOWN: i64 = 255;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.gpsAggregate.jammingState",
    "vehicle.gpsAggregate.spoofingState",
    "vehicle.gpsAggregate.authenticationState",
    "vehicle.gps.jammingState",
    "vehicle.gps.spoofingState",
    "vehicle.gps.authenticationState",
    "vehicle.gps2.jammingState",
    "vehicle.gps2.spoofingState",
    "vehicle.gps2.authenticationState",
];

#[derive(Clone)]
struct State {
    value: Option<i64>,
    text: String,
}

fn read(backend: &dyn Backend, group: &str, state: &str) -> State {
    let fact = object(&backend.get(&format!("{group}.{state}")));
    let is_fact = fact.get("kind").and_then(Value::as_str) == Some("fact");
    State {
        value: is_fact.then(|| fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64)).flatten().map(|v| v as i64),
        text: fact.get("enumOrValueString").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("n/a").to_string(),
    }
}

fn reported(state: &State) -> bool {
    state.value.is_some_and(|v| v > 0 && v < UNKNOWN)
}

pub fn authentication_colour(value: Option<i64>) -> &'static str {
    match value {
        Some(1) => "warning",
        Some(2) => "error",
        Some(3) => "good",
        _ => "neutral",
    }
}

pub fn interference_colour(value: Option<i64>) -> &'static str {
    match value {
        Some(1) => "good",
        Some(2) => "alert",
        Some(3) => "error",
        _ => "neutral",
    }
}

const DETAIL_LABELS: [&str; 3] = ["Jamming", "Spoofing", "Authentication"];

fn section(title: &str, states: &[State; 3]) -> Value {
    let rows: Vec<Value> = DETAIL_LABELS.iter().zip(states.iter()).filter(|(_, s)| reported(s)).map(|(label, s)| json!({ "label": label, "text": s.text })).collect();
    json!({ "title": title, "rows": rows })
}

pub fn resilience(groups: &[[State; 3]; 3]) -> Value {
    let aggregate = &groups[0];
    let [jamming, spoofing, authentication] = aggregate;
    let interference = [jamming.value, spoofing.value].into_iter().flatten().max();
    let interference_shown = interference.is_some_and(|v| v > 0 && v < UNKNOWN);
    json!({
        "kind": "object",
        "class": "GpsResilience",
        "shown": aggregate.iter().any(reported),
        "authentication": { "shown": reported(authentication), "colour": authentication_colour(authentication.value) },
        "interference": { "shown": interference_shown, "colour": interference_colour(interference) },
        "sections": GROUPS.iter().zip(groups.iter()).enumerate().map(|(i, ((_, title), states))| {
            let mut shown = section(title, states);
            if i == 0 {
                shown["rows"] = json!([
                    { "label": "GPS Jamming", "text": jamming.text },
                    { "label": "GPS Spoofing", "text": spoofing.text },
                    { "label": "GPS Authentication", "text": authentication.text },
                ]);
            }
            shown
        }).filter(|s| s["rows"].as_array().is_some_and(|r| !r.is_empty())).collect::<Vec<_>>(),
    })
}

pub fn resilience_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let group = |name: &str| INTEGRITY_FACTS.map(|state| read(backend, name, state));
    resilience(&GROUPS.map(|(name, _)| group(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(jam: Option<i64>, spoof: Option<i64>, auth: Option<i64>) -> [State; 3] {
        [jam, spoof, auth].map(|value| State { value, text: value.map_or("n/a".to_string(), |v| format!("state {v}")) })
    }

    #[test]
    fn it_shows_only_what_the_receiver_reports() {
        let unknown = [states(Some(0), Some(255), None), states(None, None, None), states(None, None, None)];
        assert_eq!(resilience(&unknown)["shown"], false, "0 is unknown and 255 not reported, as the toolbar treats them");
        let spoofed = [states(Some(1), Some(3), Some(3)), states(Some(1), Some(3), Some(3)), states(None, None, None)];
        let shown = resilience(&spoofed);
        assert_eq!(shown["interference"]["colour"], "error", "the worse of jamming and spoofing sets the colour");
        assert_eq!(shown["authentication"]["colour"], "good");
        assert_eq!(shown["sections"].as_array().unwrap().len(), 2, "the second receiver's section is left out when it reports nothing");
        assert_eq!(interference_colour(Some(2)), "alert", "mitigated is orange");
        assert_eq!(authentication_colour(Some(1)), "warning", "initializing is yellow");
    }
}
