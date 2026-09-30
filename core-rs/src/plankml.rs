use serde_json::Value;

use crate::cmdinfo::{Command, Firmware, VehicleClass};
use crate::planitems::UploadItem;
use crate::units::Conversion;

const KML_NAMESPACE: &str = "http://www.opengis.net/kml/2.2";
const KML_SCHEMA: &str = "https://schemas.opengis.net/kml/2.2.0/ogckml22.xsd";
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
const BALLOON_STYLE: &str = "BalloonStyle";
const MISSION_LINE_STYLE: &str = "MissionLineStyle";
const SURVEY_POLYGON_STYLE: &str = "SurveyPolygonStyle";
const CMD_NAV_WAYPOINT: i64 = 16;
const FRAME_GLOBAL: i64 = 0;

type Coord = (f64, f64, f64);

pub struct Planned {
    pub firmware: Firmware,
    pub class: VehicleClass,
    pub application: String,
    pub vertical: Option<Conversion>,
}

enum Node {
    Element { name: String, attributes: Vec<(&'static str, String)>, children: Vec<Node> },
    Text(String),
    Data(String),
}

fn element(name: &str, children: Vec<Node>) -> Node {
    Node::Element { name: name.to_string(), attributes: Vec::new(), children }
}

fn styled(name: &str, id: &str, children: Vec<Node>) -> Node {
    Node::Element { name: name.to_string(), attributes: vec![("id", id.to_string())], children }
}

fn text(name: &str, value: impl Into<String>) -> Node {
    element(name, vec![Node::Text(value.into())])
}

fn escaped(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn written(node: &Node, depth: usize) -> String {
    match node {
        Node::Text(value) => escaped(value),
        Node::Data(value) => format!("<![CDATA[{value}]]>"),
        Node::Element { name, attributes, children } => {
            let pad = " ".repeat(depth);
            let open = format!("{pad}<{name}{}", attributes.iter().map(|(key, value)| format!(" {key}=\"{}\"", escaped(value).replace('"', "&quot;"))).collect::<String>());
            match (children.is_empty(), children.iter().any(|c| matches!(c, Node::Element { .. }))) {
                (true, _) => format!("{open}/>\n"),
                (false, false) => format!("{open}>{}</{name}>\n", children.iter().map(|c| written(c, 0)).collect::<String>()),
                (false, true) => format!("{open}>\n{}{pad}</{name}>\n", children.iter().map(|c| written(c, depth + 1)).collect::<String>()),
            }
        }
    }
}

fn coord_string((latitude, longitude, altitude): Coord) -> String {
    format!("{longitude:.7},{latitude:.7},{:.2}", if altitude.is_nan() { 0.0 } else { altitude })
}

fn coord_lines(coords: &[Coord]) -> String {
    coords.iter().map(|c| format!("{}\n", coord_string(*c))).collect()
}

pub fn color(rrggbb: &str, opacity: f64) -> String {
    let rgb = rrggbb.trim_start_matches('#');
    let channel = |at: usize| rgb.get(at..at + 2).and_then(|h| u8::from_str_radix(h, 16).ok()).unwrap_or(0);
    format!("{:02x}{:02x}{:02x}{:02x}", (255.0 * opacity) as i64, channel(4), channel(2), channel(0))
}

fn styles() -> Vec<Node> {
    let line = |rrggbb: &str, width: i64, opacity: f64| element("LineStyle", vec![text("color", color(rrggbb, opacity)), text("width", width.to_string())]);
    vec![
        styled("Style", BALLOON_STYLE, vec![element("BalloonStyle", vec![text("text", "$[description]")])]),
        styled("Style", MISSION_LINE_STYLE, vec![line("#0a84ff", 4, 1.0)]),
        styled("Style", SURVEY_POLYGON_STYLE, vec![element("PolyStyle", vec![text("color", color("#008000", 0.5))]), line("#008000", 1, 0.5)]),
    ]
}

fn waypoint_mark(item: &UploadItem, info: &Command, at: Coord, home: Coord, vertical: Option<Conversion>) -> Node {
    let (shown, unit): (fn(f64) -> f64, &str) = vertical.map_or((|v| v, "m"), |c| (c.shown, c.name));
    let label = if item.command == CMD_NAV_WAYPOINT { "" } else { info.friendly_name.as_str() };
    let description = format!(
        "Index: {}\n{}\nAlt AMSL: {:.2} {unit}\nAlt Rel: {:.2} {unit}\nLat: {:.7}\nLon: {:.7}\n",
        item.seq,
        info.friendly_name,
        shown(at.2),
        shown(at.2 - home.2),
        at.0,
        at.1
    );
    element(
        "Placemark",
        vec![
            text("name", format!("{} {label}", item.seq)),
            text("styleUrl", format!("#{BALLOON_STYLE}")),
            element("description", vec![Node::Data(description)]),
            element("Point", vec![text("altitudeMode", "absolute"), text("coordinates", coord_string(at)), text("extrude", "1")]),
        ],
    )
}

fn flight_path(items: &[UploadItem], planned: &Planned) -> Vec<Node> {
    let Some(first) = items.first() else { return Vec::new() };
    let home = (first.params[4], first.params[5], first.params[6]);
    let tree = crate::cmdinfo::tree(planned.firmware, planned.class);
    let traced: Vec<(Vec<Coord>, Option<Node>)> = items
        .iter()
        .filter_map(|item| {
            let info = tree.get(&item.command)?;
            let adjust = if item.frame == FRAME_GLOBAL { 0.0 } else { home.2 };
            let climb = (info.is_takeoff && planned.class != VehicleClass::FixedWing).then_some((home.0, home.1, item.params[6] + adjust));
            let at = (item.params[4], item.params[5], item.params[6] + adjust);
            let through = (info.specifies_coordinate && !info.standalone_coordinate).then_some(at);
            let mark = info.specifies_coordinate.then(|| waypoint_mark(item, info, at, home, planned.vertical));
            Some((climb.into_iter().chain(through).collect(), mark))
        })
        .collect();
    let path: Vec<Coord> = traced.iter().flat_map(|(coords, _)| coords.clone()).collect();
    let marks: Vec<Node> = traced.into_iter().filter_map(|(_, mark)| mark).collect();
    let look_at = element(
        "LookAt",
        vec![
            text("latitude", format!("{:.7}", home.0)),
            text("longitude", format!("{:.7}", home.1)),
            text("altitude", format!("{:.2}", if home.2.is_nan() { 0.0 } else { home.2 })),
            text("heading", "-100"),
            text("tilt", "45"),
            text("range", "2500"),
        ],
    );
    vec![
        element("Folder", [vec![text("name", "Items")], marks].into_iter().flatten().collect()),
        element(
            "Placemark",
            vec![
                text("styleUrl", format!("#{MISSION_LINE_STYLE}")),
                text("name", "Flight Path"),
                text("visibility", "1"),
                look_at,
                element("LineString", vec![text("extrude", "1"), text("tessellate", "1"), text("altitudeMode", "absolute"), text("coordinates", coord_lines(&path))]),
            ],
        ),
    ]
}

fn survey_area(item: &Value) -> Vec<Coord> {
    match item.get("complexItemType").and_then(Value::as_str) {
        Some("survey") => crate::surveydoc::polygon(item).into_iter().map(|(latitude, longitude)| (latitude, longitude, f64::NAN)).collect(),
        Some("CorridorScan") => crate::corridorscan::corridor_polygon(item),
        _ => Vec::new(),
    }
}

fn survey_mark(area: &[Coord]) -> Node {
    let ring: Vec<Coord> = area.iter().chain(area.first()).copied().collect();
    element(
        "Placemark",
        vec![
            text("name", "Survey Area"),
            text("visibility", "1"),
            element("Polygon", vec![text("altitudeMode", "clampToGround"), element("outerBoundaryIs", vec![element("LinearRing", vec![text("coordinates", coord_lines(&ring))])])]),
            text("styleUrl", format!("#{SURVEY_POLYGON_STYLE}")),
        ],
    )
}

pub fn document(plan: &Value, planned: &Planned) -> Result<String, String> {
    let items = crate::planitems::flatten(plan)?;
    let areas: Vec<Node> = plan["mission"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(survey_area)
        .filter(|area| !area.is_empty())
        .map(|area| survey_mark(&area))
        .collect();
    let body = [vec![text("name", format!("{} Plan KML", planned.application)), text("open", "1")], styles(), flight_path(&items, planned), areas].into_iter().flatten().collect::<Vec<Node>>();
    let root = Node::Element {
        name: "kml".to_string(),
        attributes: vec![("xmlns", KML_NAMESPACE.to_string()), ("xmlns:xsi", XSI_NAMESPACE.to_string()), ("xsi:schemaLocation", format!("{KML_NAMESPACE} {KML_SCHEMA}"))],
        children: vec![element("Document", body)],
    };
    Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", written(&root, 0)))
}

pub fn with_extension(file: &str) -> String {
    let named = std::path::Path::new(file).file_name().and_then(|n| n.to_str()).unwrap_or("");
    if named.contains('.') { file.to_string() } else { format!("{file}.kml") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn colours_are_written_alpha_blue_green_red_as_kml_reads_them() {
        assert_eq!(color("#0a84ff", 1.0), "ffff840a");
        assert_eq!(color("#008000", 0.5), "7f008000");
        assert_eq!(with_extension("/tmp/a"), "/tmp/a.kml");
        assert_eq!(with_extension("/tmp/a.xml"), "/tmp/a.xml");
    }

    #[test]
    fn a_copter_climbs_from_home_and_every_positioned_item_is_marked() {
        let plan = json!({ "mission": {
            "plannedHomePosition": [47.0, 8.0, 400.0], "firmwareType": 3, "vehicleType": 2,
            "items": [
                { "type": "SimpleItem", "command": 22, "frame": 3, "params": [0, 0, 0, null, 0, 0, 30], "autoContinue": true, "doJumpId": 1 },
                { "type": "SimpleItem", "command": 16, "frame": 3, "params": [0, 0, 0, null, 47.001, 8.001, 50], "autoContinue": true, "doJumpId": 2 },
            ]
        } });
        let planned = Planned { firmware: Firmware::ArduPilot, class: VehicleClass::MultiRotor, application: "QGC".into(), vertical: None };
        let kml = document(&plan, &planned).unwrap();
        assert!(kml.contains("<name>QGC Plan KML</name>"));
        assert!(kml.contains("<coordinates>8.0000000,47.0000000,400.00\n8.0000000,47.0000000,430.00\n8.0010000,47.0010000,450.00\n</coordinates>"), "{kml}");
        assert!(kml.contains("<name>0 </name>") && kml.contains("<name>2 </name>") && !kml.contains("<name>1 Takeoff</name>"), "the takeoff has no coordinate of its own, so no mark");
        assert!(kml.contains("Alt Rel: 50.00 m"));
    }
}
