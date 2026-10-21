use std::path::PathBuf;

fn argument(given: &[String], name: &str) -> Option<String> {
    given
        .iter()
        .position(|a| a == name)
        .and_then(|at| given.get(at + 1).cloned())
}

fn main() {
    let given: Vec<String> = std::env::args().collect();
    let usage = "flight_replay --tlog FLIGHT.tlog --detections FLIGHT.detections.jsonl [--truth TRACK.csv|TRACK.gpx | --truth-system SYSID] [--vehicle SYSID] [--out REPORT.json]";
    let (Some(tlog), Some(seen)) = (argument(&given, "--tlog"), argument(&given, "--detections"))
    else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let read = |path: &str| std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let tlog_bytes = read(&tlog);
    let detections =
        groundstation::flightreplay::detections(&String::from_utf8_lossy(&read(&seen)));
    let truth = match (
        argument(&given, "--truth"),
        argument(&given, "--truth-system"),
    ) {
        (Some(path), _) if path.ends_with(".gpx") => {
            groundstation::flightreplay::truth_gpx(&String::from_utf8_lossy(&read(&path)))
        }
        (Some(path), _) => {
            groundstation::flightreplay::truth_csv(&String::from_utf8_lossy(&read(&path)))
        }
        (None, Some(system)) => groundstation::flightreplay::truth_from_tlog(
            &tlog_bytes,
            system
                .parse()
                .expect("--truth-system is a MAVLink system id"),
        ),
        (None, None) => Vec::new(),
    };
    let vehicle = argument(&given, "--vehicle").and_then(|id| id.parse().ok());
    let report = groundstation::flightreplay::replay(&tlog_bytes, &detections, &truth, vehicle);
    let text = serde_json::to_string_pretty(&report).expect("a report serialises");
    match argument(&given, "--out").map(PathBuf::from) {
        Some(out) => {
            std::fs::write(&out, text).unwrap_or_else(|e| panic!("{}: {e}", out.display()))
        }
        None => println!(
            "{}",
            serde_json::json!({
                "frames": report["frames"],
                "detections": report["detections"],
                "wouldRelease": report["wouldRelease"],
                "trackErrorAgainstTruth": report["trackErrorAgainstTruth"],
            })
        ),
    }
}
