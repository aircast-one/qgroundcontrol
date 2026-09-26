use serde_json::{Value, json};

use crate::read::value_number;
use crate::router::Backend;

// vehicle.terrain is TerrainFactGroup: the autopilot's TERRAIN_REPORT counts of terrain blocks it
// holds and still wants. The macOS head read the raw group and derived progress and wording from
// the two numbers; the arithmetic and the sentence are the core's now.
pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.terrain.blocksLoaded", "vehicle.terrain.blocksPending"];

fn blocks(value: Option<f64>) -> u64 {
    value.filter(|v| v.is_finite() && *v > 0.0).map_or(0, |v| v as u64)
}

pub fn text(loaded: u64, pending: u64) -> String {
    match (loaded, pending) {
        (0, 0) => String::new(),
        (l, 0) => format!("Terrain loaded, {l} block{}", if l == 1 { "" } else { "s" }),
        (l, p) => format!("Loading terrain {l} of {}", l + p),
    }
}

pub fn terrain_download_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let loaded = blocks(value_number(&backend.get("vehicle.terrain.blocksLoaded")));
    let pending = blocks(value_number(&backend.get("vehicle.terrain.blocksPending")));
    let total = loaded + pending;
    let fraction = match total {
        0 => 0.0,
        t => loaded as f64 / t as f64,
    };
    json!({
        "kind": "object",
        "class": "TerrainDownload",
        "loaded": loaded,
        "pending": pending,
        "total": total,
        "busy": pending > 0,
        "started": total > 0,
        "fraction": fraction,
        "percentText": format!("{}%", (fraction * 100.0).round() as i64),
        "text": text(loaded, pending),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Terrain(Option<(f64, f64)>);

    impl Backend for Terrain {
        fn get(&self, p: &str) -> String {
            let fact = |v: f64| json!({ "kind": "fact", "value": v });
            match (p, self.0) {
                ("vehicle.terrain.blocksLoaded", Some((l, _))) => fact(l),
                ("vehicle.terrain.blocksPending", Some((_, p))) => fact(p),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn terrain_progress_reads_the_way_the_head_wrote_it() {
        let busy = terrain_download_view(&Terrain(Some((3.0, 1.0))), &[]);
        assert_eq!((&busy["busy"], &busy["started"], &busy["total"], &busy["percentText"]), (&json!(true), &json!(true), &json!(4), &json!("75%")));
        assert_eq!(busy["text"], "Loading terrain 3 of 4");
        assert_eq!(terrain_download_view(&Terrain(Some((1.0, 0.0))), &[])["text"], "Terrain loaded, 1 block");
        assert_eq!(text(12, 0), "Terrain loaded, 12 blocks");
        let none = terrain_download_view(&Terrain(None), &[]);
        assert_eq!((&none["started"], &none["text"], &none["percentText"]), (&json!(false), &json!(""), &json!("0%")), "no vehicle reads as a download that never started, not as a failure");
        assert_eq!(terrain_download_view(&Terrain(Some((f64::NAN, -2.0))), &[])["total"], 0);
    }
}
