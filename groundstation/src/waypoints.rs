use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const COLUMNS: usize = 12;
const TAKEOFF_COMMANDS: &[i64] = &[22, 24, 84];

#[derive(Debug, PartialEq)]
pub struct Row {
    pub sequence: i64,
    pub current: bool,
    pub frame: i64,
    pub command: i64,
    pub params: [f64; 4],
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub auto_continue: bool,
}

#[derive(Debug, PartialEq)]
pub struct Waypoints {
    pub version: i64,
    pub home: Option<Row>,
    pub items: Vec<Row>,
}

fn row(line: &str) -> Result<Row, String> {
    let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
    if fields.len() != COLUMNS {
        return Err(format!("expected {COLUMNS} columns, found {}", fields.len()));
    }
    let number = |i: usize| fields[i].parse::<f64>().map_err(|_| format!("column {} is not a number: {}", i + 1, fields[i]));
    let integer = |i: usize| fields[i].parse::<i64>().map_err(|_| format!("column {} is not an integer: {}", i + 1, fields[i]));
    Ok(Row {
        sequence: integer(0)?,
        current: integer(1)? != 0,
        frame: integer(2)?,
        command: integer(3)?,
        params: [number(4)?, number(5)?, number(6)?, number(7)?],
        latitude: number(8)?,
        longitude: number(9)?,
        altitude: number(10)?,
        auto_continue: integer(11)? != 0,
    })
}

pub fn parse(text: &str) -> Result<Waypoints, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<&str> = lines.next().unwrap_or("").split(' ').collect();
    let version = match header.as_slice() {
        ["QGC", "WPL", "110"] => 110,
        ["QGC", "WPL", "120"] => 120,
        _ => return Err("not a QGC WPL 110 or 120 file".to_string()),
    };
    let rows: Vec<Row> = lines.enumerate().map(|(i, l)| row(l).map_err(|e| format!("line {}: {e}", i + 2))).collect::<Result<_, _>>()?;
    let (home, items) = match version {
        110 if !rows.is_empty() => {
            let mut rows = rows;
            let home = rows.remove(0);
            (Some(home), rows)
        }
        _ => (None, rows),
    };
    Ok(Waypoints { version, home, items })
}

fn line(r: &Row) -> String {
    [
        r.sequence.to_string(),
        (r.current as u8).to_string(),
        r.frame.to_string(),
        r.command.to_string(),
        r.params[0].to_string(),
        r.params[1].to_string(),
        r.params[2].to_string(),
        r.params[3].to_string(),
        r.latitude.to_string(),
        r.longitude.to_string(),
        r.altitude.to_string(),
        (r.auto_continue as u8).to_string(),
    ]
    .join("\t")
}

pub fn write(file: &Waypoints) -> String {
    let rows = file.home.iter().chain(&file.items).map(line);
    std::iter::once(format!("QGC WPL {}", file.version)).chain(rows).map(|l| l + "\n").collect()
}

fn row_json(r: &Row) -> Value {
    json!({
        "sequence": r.sequence,
        "current": r.current,
        "frame": r.frame,
        "command": r.command,
        "params": r.params,
        "coordinate": { "latitude": r.latitude, "longitude": r.longitude, "altitude": r.altitude },
        "autoContinue": r.auto_continue,
        "takeoff": TAKEOFF_COMMANDS.contains(&r.command),
    })
}

pub fn waypoints_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return crate::read::refused("this needs the path of a waypoints file to read, and none was given") };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "object", "class": "WaypointsFile", "path": path, "readable": false, "error": e.to_string() }),
    };
    match parse(&text) {
        Err(error) => json!({ "kind": "object", "class": "WaypointsFile", "path": path, "readable": true, "valid": false, "error": error }),
        Ok(file) => json!({
            "kind": "object",
            "class": "WaypointsFile",
            "path": path,
            "readable": true,
            "valid": true,
            "error": "",
            "version": file.version,
            "homeInFile": file.home.is_some(),
            "home": file.home.as_ref().map(|h| json!({ "latitude": h.latitude, "longitude": h.longitude, "altitude": h.altitude })),
            "itemCount": file.items.len(),
            "takeoffCount": file.items.iter().filter(|r| TAKEOFF_COMMANDS.contains(&r.command)).count(),
            "items": file.items.iter().map(row_json).collect::<Vec<_>>(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_110_file_puts_home_in_the_first_row() {
        let text = "QGC WPL 110\n0\t1\t0\t16\t0\t0\t0\t0\t47.66\t-122.10\t5.2\t1\n1\t0\t3\t22\t0\t0\t0\t0\t47.661\t-122.103\t100\t1\n2\t0\t3\t16\t0\t0\t0\t0\t47.662\t-122.104\t100\t1\n";
        let file = parse(text).unwrap();
        assert_eq!(file.version, 110);
        assert_eq!(file.home.as_ref().unwrap().altitude, 5.2);
        assert_eq!(file.items.len(), 2);
        assert_eq!(file.items[0].command, 22);
        assert!(file.items[0].current == false);
    }

    #[test]
    fn a_120_file_has_no_home_row_and_bad_input_is_refused_with_a_line_number() {
        let file = parse("QGC WPL 120\n0\t0\t3\t16\t0\t0\t0\t0\t1\t2\t3\t1\n").unwrap();
        assert!(file.home.is_none());
        assert_eq!(file.items.len(), 1);
        assert_eq!(parse("QGC WPL 130\n").unwrap_err(), "not a QGC WPL 110 or 120 file");
        assert!(parse("QGC WPL 110\n0\t1\t0\n").unwrap_err().starts_with("line 2: expected 12 columns"));
        assert!(parse("QGC WPL 110\n0\t1\t0\t16\t0\t0\t0\t0\tx\t-122.10\t5.2\t1\n").unwrap_err().contains("column 9"));
    }

    #[test]
    fn the_repository_fixture_parses() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/MissionManager/MissionPlanner.waypoints")).unwrap();
        let file = parse(&text).unwrap();
        assert_eq!(file.version, 110);
        assert!(file.home.is_some());
        assert!(file.items.len() > 3);
    }

    #[test]
    fn a_written_file_reads_back_equal() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/MissionManager/MissionPlanner.waypoints")).unwrap();
        let file = parse(&text).unwrap();
        let written = write(&file);
        assert!(written.starts_with("QGC WPL 110\n"));
        assert_eq!(written.lines().count(), 1 + file.home.iter().count() + file.items.len());
        assert_eq!(parse(&written).unwrap(), file);
        let nan_row = Row { sequence: 1, current: false, frame: 3, command: 16, params: [0.0, 0.0, 0.0, f64::NAN, 0.0, 0.0, 0.0][..4].try_into().unwrap(), latitude: 1.5, longitude: -2.25, altitude: 30.0, auto_continue: true };
        let round = parse(&write(&Waypoints { version: 120, home: None, items: vec![nan_row] })).unwrap();
        assert!(round.items[0].params[3].is_nan() && round.items[0].latitude == 1.5 && round.version == 120);
    }
}
