use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.escs.count"];

const HEALTH_CHECKED_MOTORS: u32 = 4;

fn fact<'a>(group: &'a Value, name: &str) -> Option<&'a Value> {
    group.get("facts").and_then(Value::as_array).and_then(|facts| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)))
}

fn raw(group: &Value, name: &str) -> i64 {
    fact(group, name).and_then(|f| f.get("rawValue").or(f.get("value"))).and_then(Value::as_f64).map_or(0, |v| v as i64)
}

fn shown(group: &Value, name: &str) -> String {
    fact(group, name)
        .map(|f| {
            let value = f.get("valueString").and_then(Value::as_str).unwrap_or_default();
            let units = f.get("units").and_then(Value::as_str).unwrap_or_default();
            [value, units].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ")
        })
        .unwrap_or_default()
}

pub fn summary(escs: &[Value]) -> Value {
    let Some(first) = escs.first() else { return json!({ "kind": "object", "class": "Escs", "shown": false }) };
    let motor_count = raw(first, "count");
    let online_mask = raw(first, "info");
    let online_count = if motor_count == 0 { 0 } else { i64::from(online_mask.count_ones()) };
    let online = |index: usize| online_mask & (1 << index) != 0;
    let motor_healthy = |index: usize| online(index) && escs.get(index).is_some_and(|esc| raw(esc, "failureFlags") == 0);
    let failing_checked = (0..HEALTH_CHECKED_MOTORS as usize).any(|index| online(index) && escs.get(index).is_some_and(|esc| raw(esc, "failureFlags") > 0));
    let healthy = online_count == motor_count && !failing_checked;
    let healthy_motors = (0..escs.len()).filter(|i| motor_healthy(*i)).count();
    json!({
        "kind": "object",
        "class": "Escs",
        "shown": true,
        "onlineCount": online_count,
        "healthy": healthy,
        "healthText": if healthy { "OK" } else { "ERR" },
        "healthyMotorsText": format!("{healthy_motors}/{}", escs.len()),
        "totalErrors": escs.iter().map(|esc| raw(esc, "errorCount")).sum::<i64>(),
        "motors": escs.iter().enumerate().map(|(index, esc)| {
            let fine = motor_healthy(index);
            json!({
                "title": format!("Motor {} {}", raw(esc, "id") + 1, if fine { "" } else { "- OFFLINE" }).trim_end().to_string(),
                "healthy": fine,
                "rpm": shown(esc, "rpm"),
                "temperature": shown(esc, "temperature"),
                "voltage": shown(esc, "voltage"),
                "current": shown(esc, "current"),
                "errors": shown(esc, "errorCount"),
            })
        }).collect::<Vec<_>>(),
    })
}

pub fn esc_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let listed = object(&backend.get("vehicle.escs"));
    summary(&listed.get("elements").and_then(Value::as_array).cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esc(id: i64, count: i64, info: i64, failures: i64, errors: i64) -> Value {
        let f = |name: &str, value: i64, units: &str| json!({ "name": name, "rawValue": value, "valueString": value.to_string(), "units": units });
        json!({ "facts": [f("id", id, ""), f("count", count, ""), f("info", info, ""), f("failureFlags", failures, ""), f("errorCount", errors, ""), f("rpm", 4200, "rpm"), f("voltage", 16, "V")] })
    }

    #[test]
    fn health_reads_as_the_toolbar() {
        let four = [esc(0, 4, 0b1111, 0, 1), esc(1, 4, 0b1111, 0, 0), esc(2, 4, 0b1111, 0, 2), esc(3, 4, 0b1111, 0, 0)];
        let fine = summary(&four);
        assert_eq!((fine["onlineCount"].as_i64(), fine["healthText"].as_str()), (Some(4), Some("OK")));
        assert_eq!(fine["totalErrors"], 3);
        assert_eq!(fine["healthyMotorsText"], "4/4");
        assert_eq!(fine["motors"][0]["title"], "Motor 1");
        assert_eq!(fine["motors"][0]["rpm"], "4200 rpm");
        let one_down = [esc(0, 4, 0b0111, 0, 0), esc(1, 4, 0b0111, 0, 0), esc(2, 4, 0b0111, 0, 0), esc(3, 4, 0b0111, 0, 0)];
        let down = summary(&one_down);
        assert_eq!(down["healthText"], "ERR", "a motor missing from the online mask is an error");
        assert_eq!(down["motors"][3]["title"], "Motor 4 - OFFLINE");
        let failing = [esc(0, 1, 0b1, 4, 0)];
        assert_eq!(summary(&failing)["healthText"], "ERR", "a failure flag on an online motor is an error");
        assert_eq!(summary(&[])["shown"], false);
    }
}
