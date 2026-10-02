use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.supports.terrainFrame", "plan.missionController.containsItems", crate::coreplan::CHANGED, "corePlugin.options.showMissionAbsoluteAltitude"];

pub const MIXED: i64 = 0;
pub const RELATIVE: i64 = 1;
pub const ABSOLUTE: i64 = 2;
pub const CALC_ABOVE_TERRAIN: i64 = 3;
pub const TERRAIN_FRAME: i64 = 4;

const NO_TERRAIN_FRAME: &str = "This vehicle's firmware cannot hold an altitude above terrain.";
const ITEMS_ADDED: &str = "The plan's altitude mode is chosen before mission items are added; once they exist only Mixed is left.";
const NOT_A_PLAN: &str = "Mixed applies to a whole plan, where each item sets its own.";
const ABSOLUTE_HIDDEN: &str = "This build does not offer altitudes above mean sea level for mission items.";

const MODES: &[(i64, &str, &str)] = &[
    (RELATIVE, "Relative To Launch", "Above the launch position."),
    (ABSOLUTE, "AMSL", "Above mean sea level."),
    (CALC_ABOVE_TERRAIN, "Calculated Above Terrain", "Above terrain, converted to AMSL before upload."),
    (TERRAIN_FRAME, "Terrain Frame", "Above terrain, held by the vehicle in flight."),
    (MIXED, "Mixed Modes", "Each item sets its own."),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub mission: bool,
    pub current: i64,
    pub holds_altitude_above_terrain: bool,
    pub has_items: bool,
    pub show_absolute: bool,
}

fn offered(mode: i64, inputs: &Inputs) -> bool {
    let removed = match mode {
        MIXED => !inputs.mission,
        TERRAIN_FRAME => !inputs.holds_altitude_above_terrain,
        ABSOLUTE => !inputs.show_absolute,
        _ => false,
    };
    !removed || mode == inputs.current
}

fn enabled(mode: i64, inputs: &Inputs) -> bool {
    mode == inputs.current || mode == MIXED || !inputs.mission || !inputs.has_items
}

fn absence(mode: i64) -> &'static str {
    match mode {
        MIXED => NOT_A_PLAN,
        TERRAIN_FRAME => NO_TERRAIN_FRAME,
        ABSOLUTE => ABSOLUTE_HIDDEN,
        _ => "",
    }
}

pub fn omitted(inputs: &Inputs) -> Vec<Value> {
    MODES
        .iter()
        .filter(|(mode, _, _)| !offered(*mode, inputs))
        .map(|(mode, title, _)| json!({ "raw": mode, "title": title, "reason": absence(*mode) }))
        .collect()
}

fn reason(mode: i64, inputs: &Inputs) -> &'static str {
    match () {
        _ if enabled(mode, inputs) => "",
        _ if mode == TERRAIN_FRAME && !inputs.holds_altitude_above_terrain => NO_TERRAIN_FRAME,
        _ => ITEMS_ADDED,
    }
}

pub fn modes(inputs: &Inputs) -> Vec<Value> {
    MODES
        .iter()
        .filter(|(mode, _, _)| offered(*mode, inputs))
        .map(|(mode, title, help)| {
            json!({
                "raw": mode,
                "title": title,
                "help": help,
                "current": *mode == inputs.current,
                "enabled": enabled(*mode, inputs),
                "reason": reason(*mode, inputs),
            })
        })
        .collect()
}

pub(crate) fn read_inputs(backend: &dyn Backend, mission_context: bool, current: i64) -> Inputs {
    let vehicles = object(&backend.get_fields("vehicles", "activeVehicleAvailable"));
    let vehicle = object(&backend.get_fields("vehicle.supports", "terrainFrame"));
    let mission = object(&backend.get_fields("plan.missionController", "containsItems"));
    let options = object(&backend.get_fields("corePlugin.options", "showMissionAbsoluteAltitude"));
    Inputs {
        mission: mission_context,
        current,
        holds_altitude_above_terrain: match flag(&vehicles, "activeVehicleAvailable") {
            true => flag(&vehicle, "terrainFrame"),
            false => crate::coreplan::plan_types().is_none_or(|(firmware, _)| crate::plandoc::firmware(firmware) != crate::cmdinfo::Firmware::Px4),
        },
        has_items: flag(&mission, "containsItems"),
        show_absolute: options.get("showMissionAbsoluteAltitude").and_then(Value::as_bool).unwrap_or(true),
    }
}

pub fn altitude_modes_view(backend: &dyn Backend, args: &[String]) -> Value {
    let inputs = read_inputs(backend, args.first().map(|a| a != "item").unwrap_or(true), args.get(1).and_then(|a| a.trim().parse().ok()).unwrap_or(-1));
    json!({
        "kind": "object",
        "class": "AltitudeModes",
        "context": if inputs.mission { "mission" } else { "item" },
        "current": inputs.current,
        "holdsAltitudeAboveTerrain": inputs.holds_altitude_above_terrain,
        "modes": modes(&inputs),
        "omitted": omitted(&inputs),
    })
}

pub const FRAME_RELATIVE: i64 = 1;
pub const FRAME_ABSOLUTE: i64 = 2;
pub const FRAME_CALC_ABOVE_TERRAIN: i64 = 3;
pub const FRAME_TERRAIN: i64 = 4;

pub fn frame_short_description(frame: i64) -> &'static str {
    match frame {
        FRAME_RELATIVE => "Relative (Rel)",
        FRAME_ABSOLUTE => "Absolute (AMSL)",
        FRAME_CALC_ABOVE_TERRAIN => "Above Terrain Calced (AGLC)",
        FRAME_TERRAIN => "Above Terrain (AGL)",
        _ => "",
    }
}

pub fn transect_distance_modes(manual_camera: bool, terrain_frame: bool) -> Vec<Value> {
    [FRAME_RELATIVE, FRAME_ABSOLUTE, FRAME_CALC_ABOVE_TERRAIN, FRAME_TERRAIN]
        .into_iter()
        .filter(|frame| (*frame != FRAME_ABSOLUTE || manual_camera) && (*frame != FRAME_TERRAIN || terrain_frame))
        .map(|frame| json!({ "raw": frame, "title": frame_short_description(frame) }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transect_frame_menu_drops_what_transect_style_terrain_follow_removes() {
        let raws = |manual, terrain| transect_distance_modes(manual, terrain).iter().map(|m| m["raw"].as_i64().unwrap()).collect::<Vec<_>>();
        assert_eq!(raws(true, true), [1, 2, 3, 4]);
        assert_eq!(raws(false, true), [1, 3, 4], "Absolute only for a manual camera");
        assert_eq!(raws(true, false), [1, 2, 3], "Terrain only where the firmware supports the terrain frame");
        assert_eq!(transect_distance_modes(true, true)[2]["title"], "Above Terrain Calced (AGLC)");
    }

    fn base() -> Inputs {
        Inputs { mission: true, current: RELATIVE, holds_altitude_above_terrain: true, has_items: true, show_absolute: true }
    }

    fn raws(inputs: &Inputs) -> Vec<i64> {
        modes(inputs).iter().map(|m| m["raw"].as_i64().unwrap()).collect()
    }

    #[test]
    fn a_mode_the_list_does_not_offer_says_why_it_is_not_there() {
        let vtol = Inputs { mission: true, current: RELATIVE, holds_altitude_above_terrain: true, has_items: true, show_absolute: true };
        assert!(omitted(&vtol).is_empty(), "with everything supported nothing is left out, so an empty list is a real answer and not the only answer this can give");

        let plain = Inputs { holds_altitude_above_terrain: false, show_absolute: false, ..vtol };
        let left_out = omitted(&plain);
        let named: Vec<i64> = left_out.iter().map(|m| m["raw"].as_i64().unwrap()).collect();
        assert_eq!(named, vec![ABSOLUTE, TERRAIN_FRAME], "a mode removed from the picker is indistinguishable from a mode that never existed, unless the list that dropped it says so");
        assert!(left_out.iter().all(|m| !m["reason"].as_str().unwrap().is_empty()), "and an entry with no reason is the same silence one level in");
        assert_eq!(left_out[1]["reason"], NO_TERRAIN_FRAME);

        let single = Inputs { mission: false, ..vtol };
        let alone = omitted(&single);
        assert_eq!(alone.iter().map(|m| m["raw"].as_i64().unwrap()).collect::<Vec<_>>(), vec![MIXED], "one item cannot be mixed with itself");
        assert_eq!(alone[0]["reason"], NOT_A_PLAN, "every omission carries its own reason, and asserting that over one subset leaves the others free to say nothing");

        let holding = Inputs { mission: false, current: MIXED, ..vtol };
        assert!(omitted(&holding).is_empty(), "except when it is the mode already in force, which the picker must keep so the operator can see what they are changing from");
    }

    #[test]
    fn a_vehicle_that_cannot_hold_an_altitude_above_terrain_is_not_offered_it() {
        assert_eq!(raws(&base()), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN, TERRAIN_FRAME, MIXED]);
        let plain = Inputs { holds_altitude_above_terrain: false, ..base() };
        assert_eq!(raws(&plain), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN, MIXED], "offering a mode the firmware cannot fly is an offer without its gate");
        let already_on_it = Inputs { holds_altitude_above_terrain: false, current: TERRAIN_FRAME, ..base() };
        assert!(raws(&already_on_it).contains(&TERRAIN_FRAME), "a mode the plan is already set to stays listed, so an operator can see what they are on and leave it");
    }

    #[test]
    fn an_item_menu_never_offers_mixed_and_a_mission_menu_always_does() {
        assert!(raws(&base()).contains(&MIXED));
        let item = Inputs { mission: false, ..base() };
        assert!(!raws(&item).contains(&MIXED), "mixed means each item chooses, which is not a choice an item can make");
        let item_without_terrain = Inputs { mission: false, holds_altitude_above_terrain: false, ..base() };
        assert_eq!(raws(&item_without_terrain), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN]);
    }

    #[test]
    fn the_plan_mode_is_chosen_before_items_exist_and_only_mixed_opens_after() {
        let empty = Inputs { has_items: false, ..base() };
        assert!(modes(&empty).iter().all(|m| m["enabled"] == true), "MissionSettingsEditor disables nothing while _noMissionItemsAdded");
        let listed = modes(&Inputs { has_items: true, ..base() });
        let enabled_raws: Vec<i64> = listed.iter().filter(|m| m["enabled"] == true).map(|m| m["raw"].as_i64().unwrap()).collect();
        assert_eq!(enabled_raws, [RELATIVE, MIXED], "with items only the mode in use and mixed stay live");
        assert_eq!(listed[1]["reason"], ITEMS_ADDED);
        let item = Inputs { mission: false, has_items: false, ..base() };
        assert!(modes(&item).iter().all(|m| m["enabled"] == true), "an item editor is only open because an item exists");
    }

    #[test]
    fn absolute_is_hidden_where_the_build_hides_it_unless_it_is_the_one_in_use() {
        let hidden = Inputs { show_absolute: false, ..base() };
        assert!(!raws(&hidden).contains(&ABSOLUTE));
        let in_use = Inputs { show_absolute: false, current: ABSOLUTE, ..base() };
        assert!(raws(&in_use).contains(&ABSOLUTE));
    }

    struct Fake {
        terrain: bool,
        items: bool,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.get_fields(path, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }).to_string(),
                "vehicle.supports" => json!({ "kind": "object", "terrainFrame": self.terrain }).to_string(),
                "plan.missionController" => json!({ "kind": "object", "containsItems": self.items }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn the_view_reads_the_vehicle_and_the_plan() {
        let view = altitude_modes_view(&Fake { terrain: false, items: false }, &["mission".to_string(), "1".to_string()]);
        assert_eq!(view["context"], "mission");
        assert_eq!(view["holdsAltitudeAboveTerrain"], false, "this answers for the terrain frame alone, raw 4, and Calculated Above Terrain at raw 3 stays selectable while it is false - a head reading the old name as a verdict on terrain altitudes would have hidden a mode that works");
        assert!(view["modes"].as_array().unwrap().iter().any(|m| m["raw"] == 3 && m["enabled"] == true), "the mode this flag does not speak for");
        assert_eq!(view["modes"].as_array().unwrap().len(), 4);
        assert_eq!(view["current"], RELATIVE);
        let absent = altitude_modes_view(&Fake { terrain: true, items: false }, &[]);
        assert_eq!(absent["current"], -1, "a head that names no current mode is not told one is chosen");
        assert!(absent["modes"].as_array().unwrap().iter().all(|m| m["current"] == false));
        assert!(absent["modes"].as_array().unwrap().iter().all(|m| m["enabled"] == true), "with no items every mode can still be chosen");
    }
    #[test]
    fn the_altitude_modes_are_the_ordinals_qgc_declares() {
        let header = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/QmlControls/QGroundControlQmlGlobal.h")).unwrap_or_default();
        let body = header
            .split_once("enum AltitudeFrame")
            .and_then(|(_, rest)| rest.split_once('{'))
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(body, _)| body.to_string())
            .unwrap_or_default();
        assert!(body.contains("AltitudeFrameRelative"), "this guard reads QGroundControlQmlGlobal.h, where the modes are declared; a rename there would leave every assertion below comparing nothing");

        let uncommented = body.lines().map(|line| line.split("//").next().unwrap_or("")).collect::<Vec<_>>().join(" ");
        let declared: Vec<(String, i64)> = uncommented
            .split(',')
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .scan(0i64, |next, entry| {
                let (name, value) = match entry.split_once('=') {
                    Some((name, given)) => (name.trim().to_string(), given.trim().parse().unwrap_or(*next)),
                    None => (entry.clone(), *next),
                };
                *next = value + 1;
                Some((name, value))
            })
            .collect();
        let at = |name: &str| declared.iter().find(|(n, _)| n == name).map(|(_, v)| *v);

        assert_eq!(
            [at("AltitudeFrameMixed"), at("AltitudeFrameRelative"), at("AltitudeFrameAbsolute"), at("AltitudeFrameCalcAboveTerrain"), at("AltitudeFrameTerrain")],
            [Some(MIXED), Some(RELATIVE), Some(ABSOLUTE), Some(CALC_ABOVE_TERRAIN), Some(TERRAIN_FRAME)],
            "these decide what a waypoint's altitude is measured FROM, so a shifted ordinal flies the aircraft at a height nobody asked for"
        );

        assert_eq!(
            [crate::planfile::ALTITUDE_MODE_MIXED, crate::planfile::ALTITUDE_MODE_RELATIVE, crate::planfile::ALTITUDE_MODE_ABSOLUTE, crate::planfile::ALTITUDE_MODE_TERRAIN_FRAME],
            [MIXED, RELATIVE, ABSOLUTE, TERRAIN_FRAME],
            "planfile keeps its own copy for reading and writing .plan files, and a plan written with one numbering and read with another is silently wrong"
        );
    }

}
