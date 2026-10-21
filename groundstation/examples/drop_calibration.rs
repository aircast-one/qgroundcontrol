fn argument(given: &[String], name: &str) -> Option<String> {
    given
        .iter()
        .position(|a| a == name)
        .and_then(|at| given.get(at + 1).cloned())
}

fn main() {
    let given: Vec<String> = std::env::args().collect();
    let usage = "drop_calibration --tlog FLIGHT.tlog --drops DROPS.json --mass-g GRAMS [--vehicle SYSID]\nDROPS.json: [{\"separationUs\": 1700000000123456, \"landed\": {\"latitude\": 47.1, \"longitude\": 8.5}}, ...]";
    let (Some(tlog), Some(drops), Some(mass)) = (
        argument(&given, "--tlog"),
        argument(&given, "--drops"),
        argument(&given, "--mass-g").and_then(|g| g.parse::<f64>().ok()),
    ) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let read = |path: &str| std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let tlog_bytes = read(&tlog);
    let vehicle = argument(&given, "--vehicle")
        .and_then(|id| id.parse().ok())
        .unwrap_or(1);
    let listed: Vec<serde_json::Value> =
        serde_json::from_slice(&read(&drops)).expect("DROPS.json is a JSON array");
    let observed: Vec<groundstation::dropcal::Observed> = listed
        .iter()
        .filter_map(|drop| {
            let landed = groundstation::dropsim::Point {
                lat: drop["landed"]["latitude"].as_f64()?,
                lon: drop["landed"]["longitude"].as_f64()?,
            };
            groundstation::dropcal::observed(
                &tlog_bytes,
                vehicle,
                drop["separationUs"].as_u64()?,
                landed,
            )
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&groundstation::dropcal::calibration(
            &observed,
            mass / 1000.0
        ))
        .expect("serialises")
    );
}
