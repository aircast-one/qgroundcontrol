use serde_json::Value;

const CMD_NAV_WAYPOINT: i64 = 16;
const CMD_DO_JUMP: i64 = 177;
const FRAME_GLOBAL: i64 = 0;
const SAVED_TRANSECTS: &[&str] = &["survey", "CorridorScan"];

#[derive(Debug, Clone, PartialEq)]
pub struct UploadItem {
    pub seq: usize,
    pub frame: i64,
    pub command: i64,
    pub params: [f64; 7],
    pub auto_continue: bool,
}

struct Placed {
    item: UploadItem,
    jump_id: Option<i64>,
}

pub fn flatten(plan: &Value, send_home: bool) -> Result<Vec<UploadItem>, String> {
    let mission = plan.get("mission").ok_or("The plan has no mission.")?;
    let home = mission
        .get("plannedHomePosition")
        .and_then(Value::as_array)
        .filter(|h| h.len() >= 3)
        .ok_or("The plan has no planned home position.")?;
    let home_item = UploadItem {
        seq: 0,
        frame: FRAME_GLOBAL,
        command: CMD_NAV_WAYPOINT,
        params: [0.0, 0.0, 0.0, 0.0, number(&home[0]), number(&home[1]), number(&home[2])],
        auto_continue: true,
    };
    let saved = mission.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let expanded = saved.iter().map(expand).collect::<Result<Vec<_>, _>>()?;
    let placed: Vec<Placed> = std::iter::once(Placed { item: home_item, jump_id: None })
        .chain(expanded.into_iter().flatten())
        .enumerate()
        .map(|(seq, placed)| Placed { item: UploadItem { seq, ..placed.item }, ..placed })
        .collect();
    let resolved = placed
        .iter()
        .map(|p| match p.item.command {
            CMD_DO_JUMP => jump_target(&placed, p.item.params[0]).map(|target| UploadItem { params: with_first(p.item.params, target as f64), ..p.item.clone() }),
            _ => Ok(p.item.clone()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match send_home {
        true => resolved,
        false => resolved.into_iter().skip(1).enumerate().map(|(seq, item)| UploadItem { seq, ..item }).collect(),
    })
}

fn jump_target(placed: &[Placed], jump_id: f64) -> Result<usize, String> {
    placed
        .iter()
        .find(|p| p.jump_id == Some(jump_id as i64))
        .map(|p| p.item.seq)
        .ok_or_else(|| format!("Could not find doJumpId: {}", jump_id as i64))
}

fn with_first(params: [f64; 7], first: f64) -> [f64; 7] {
    [first, params[1], params[2], params[3], params[4], params[5], params[6]]
}

fn expand(item: &Value) -> Result<Vec<Placed>, String> {
    match item.get("type").and_then(Value::as_str) {
        Some("SimpleItem") => simple(item).map(|placed| vec![placed]),
        Some("ComplexItem") => {
            let kind = item.get("complexItemType").and_then(Value::as_str).unwrap_or("");
            match SAVED_TRANSECTS.contains(&kind) {
                true => item
                    .get("TransectStyleComplexItem")
                    .and_then(|t| t.get("Items"))
                    .and_then(Value::as_array)
                    .ok_or_else(|| format!("The {kind} item has no saved mission items."))?
                    .iter()
                    .map(|inner| simple(inner).map(|placed| Placed { jump_id: None, ..placed }))
                    .collect(),
                false => Err(format!("The core cannot build the mission items of a {kind} item yet.")),
            }
        }
        other => Err(format!("Unknown item type: {}", other.unwrap_or("none"))),
    }
}

fn simple(item: &Value) -> Result<Placed, String> {
    let params = item.get("params").and_then(Value::as_array).ok_or("A mission item has no params.")?;
    let coordinate = item.get("coordinate").and_then(Value::as_array);
    let spelled: Vec<f64> = match (params.len(), coordinate) {
        (7, _) => params.iter().map(number).collect(),
        (4, Some(c)) if c.len() >= 3 => params.iter().chain(c.iter().take(3)).map(number).collect(),
        _ => return Err("A mission item needs seven params, or four and a coordinate.".to_string()),
    };
    let field = |key: &str| item.get(key).and_then(Value::as_i64).ok_or_else(|| format!("A mission item has no {key}."));
    Ok(Placed {
        item: UploadItem {
            seq: 0,
            frame: field("frame")?,
            command: field("command")?,
            params: [spelled[0], spelled[1], spelled[2], spelled[3], spelled[4], spelled[5], spelled[6]],
            auto_continue: item.get("autoContinue").and_then(Value::as_bool).unwrap_or(true),
        },
        jump_id: item.get("doJumpId").and_then(Value::as_i64),
    })
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn section_test() -> Value {
        serde_json::from_str(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap()
    }

    #[test]
    fn ardupilot_is_sent_the_planned_home_as_item_zero() {
        let items = flatten(&section_test(), true).unwrap();
        assert_eq!(items.len(), 6);
        assert_eq!((items[0].command, items[0].frame, items[0].params[4], items[0].params[6]), (16, 0, 47.633389756176875, 20.0));
        assert_eq!(items.iter().map(|i| i.seq).collect::<Vec<_>>(), (0..6).collect::<Vec<_>>());
        assert_eq!((items[1].command, items[1].frame, items[1].params[6]), (22, 3, 20.0));
        assert!(items[1].params[3].is_nan(), "a null param is NaN on the wire, as Qt sends it");
        assert_eq!((items[4].command, items[4].params[4], items[4].params[6]), (205, 0.0, 2.0));
    }

    #[test]
    fn a_vehicle_that_keeps_home_to_itself_is_not_sent_one() {
        let items = flatten(&section_test(), false).unwrap();
        assert_eq!(items.len(), 5);
        assert_eq!((items[0].seq, items[0].command), (0, 22));
    }

    #[test]
    fn a_jump_targets_the_sequence_its_saved_id_now_occupies() {
        let plan = json!({ "mission": { "firmwareType": 3, "plannedHomePosition": [1.0, 2.0, 3.0], "items": [
            { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 1, "params": [0, 0, 0, 0, 1.0, 2.0, 10.0] },
            { "type": "ComplexItem", "complexItemType": "survey", "TransectStyleComplexItem": { "Items": [
                { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 2, "params": [0, 0, 0, 0, 1.1, 2.1, 10.0] },
                { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 3, "params": [0, 0, 0, 0, 1.2, 2.2, 10.0] },
            ] } },
            { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 9, "params": [0, 0, 0, 0, 1.3, 2.3, 10.0] },
            { "type": "SimpleItem", "command": 177, "frame": 2, "doJumpId": 10, "params": [9, 3, 0, 0, 0, 0, 0] },
        ] } });
        let items = flatten(&plan, true).unwrap();
        assert_eq!(items.len(), 6, "home, one waypoint, two survey items, a waypoint and the jump");
        assert_eq!((items[5].command, items[5].params[0], items[5].params[1]), (177, 4.0, 3.0));
    }

    #[test]
    fn a_jump_into_a_survey_is_refused_as_qt_refuses_it() {
        let plan = json!({ "mission": { "firmwareType": 3, "plannedHomePosition": [1.0, 2.0, 3.0], "items": [
            { "type": "ComplexItem", "complexItemType": "survey", "TransectStyleComplexItem": { "Items": [
                { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 2, "params": [0, 0, 0, 0, 1.1, 2.1, 10.0] },
            ] } },
            { "type": "SimpleItem", "command": 177, "frame": 2, "doJumpId": 3, "params": [2, 1, 0, 0, 0, 0, 0] },
        ] } });
        assert_eq!(flatten(&plan, true), Err("Could not find doJumpId: 2".to_string()));
    }

    #[test]
    fn items_the_core_cannot_generate_are_refused_rather_than_skipped() {
        let plan = json!({ "mission": { "plannedHomePosition": [1.0, 2.0, 3.0], "items": [
            { "type": "ComplexItem", "complexItemType": "StructureScan" },
        ] } });
        assert!(flatten(&plan, true).unwrap_err().contains("StructureScan"));
    }
}

#[cfg(test)]
mod qt_oracle {
    use super::*;

    const FRAME_MISSION: i64 = 2;

    fn same_on_the_wire(core: &UploadItem, qt: &Value) -> Result<(), String> {
        let sent = qt["params"].as_array().unwrap();
        let param = |i: usize| sent[i].as_f64().unwrap_or(f64::NAN);
        let float = |mine: f64, theirs: f64| (mine.is_nan() && theirs.is_nan()) || (mine as f32) == (theirs as f32);
        let scaled = |mine: f64| match core.frame {
            FRAME_MISSION => mine as i32,
            _ => (mine * 1e7) as i32,
        };
        let header = (qt["seq"].as_u64().unwrap() as usize, qt["frame"].as_i64().unwrap(), qt["command"].as_i64().unwrap(), qt["autoContinue"].as_bool().unwrap());
        let floats = [0, 1, 2, 3, 6].iter().all(|&i| float(core.params[i], param(i)));
        let ints = scaled(core.params[4]) as f64 == param(4) && scaled(core.params[5]) as f64 == param(5);
        match (header == (core.seq, core.frame, core.command, core.auto_continue), floats && ints) {
            (true, true) => Ok(()),
            _ => Err(format!("core {core:?}\n  qt {qt}")),
        }
    }

    fn agrees(plan: &str, sent: &str) {
        agrees_with_home_altitude(plan, sent, None);
    }

    fn agrees_with_home_altitude(plan: &str, sent: &str, home_altitude: Option<f64>) {
        let flat = flatten(&serde_json::from_str(plan).unwrap(), true).unwrap();
        let items: Vec<UploadItem> = flat
            .into_iter()
            .map(|item| match (item.seq, home_altitude) {
                (0, Some(altitude)) => UploadItem { params: [item.params[0], item.params[1], item.params[2], item.params[3], item.params[4], item.params[5], altitude], ..item },
                _ => item,
            })
            .collect();
        let sent: Vec<Value> = serde_json::from_str(sent).unwrap();
        assert_eq!(items.len(), sent.len());
        let differing: Vec<String> = items.iter().zip(&sent).filter_map(|(core, qt)| same_on_the_wire(core, qt).err()).collect();
        assert!(differing.is_empty(), "{}", differing.join("\n"));
    }

    #[test]
    fn a_plan_with_sections_uploads_the_items_qt_sent_to_ardupilot_sitl() {
        let sent = include_str!("../tests/fixtures/sectiontest-sent-by-qt.json");
        let terrain_under_home = 0.0;
        agrees_with_home_altitude(include_str!("../../test/MissionManager/SectionTest.plan"), sent, Some(terrain_under_home));
        let file_home = flatten(&serde_json::from_str(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap(), true).unwrap()[0].params[6];
        assert_eq!(file_home, 20.0, "Qt's editor replaces the file's home altitude with the terrain height under it (MissionSettingsItem::_setHomeAltFromTerrain), so the model that owns the plan must apply that rule; flattening passes through whatever the document holds");
    }

    #[test]
    fn a_survey_uploads_the_items_qt_sent_to_ardupilot_sitl() {
        agrees(include_str!("../tests/fixtures/survey-upload.plan"), include_str!("../tests/fixtures/survey-sent-by-qt.json"));
    }
}
