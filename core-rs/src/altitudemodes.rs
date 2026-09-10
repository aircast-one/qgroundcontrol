use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.supportsTerrainFrame", "plan.missionController.containsItems", "corePlugin.options.showMissionAbsoluteAltitude"];

pub const MIXED: i64 = 0;
pub const RELATIVE: i64 = 1;
pub const ABSOLUTE: i64 = 2;
pub const CALC_ABOVE_TERRAIN: i64 = 3;
pub const TERRAIN_FRAME: i64 = 4;

const NO_TERRAIN_FRAME: &str = "This vehicle's firmware cannot hold an altitude above terrain.";
const NO_ITEMS_YET: &str = "Add a mission item before choosing how its altitude is measured.";

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
    pub supports_terrain_frame: bool,
    pub has_items: bool,
    pub show_absolute: bool,
}

fn offered(mode: i64, inputs: &Inputs) -> bool {
    let removed = match mode {
        MIXED => !inputs.mission,
        TERRAIN_FRAME => !inputs.supports_terrain_frame,
        ABSOLUTE => !inputs.show_absolute,
        _ => false,
    };
    !removed || mode == inputs.current
}

fn enabled(mode: i64, inputs: &Inputs) -> bool {
    mode == inputs.current || mode == MIXED || !inputs.mission || inputs.has_items
}

fn reason(mode: i64, inputs: &Inputs) -> &'static str {
    match () {
        _ if enabled(mode, inputs) => "",
        _ if mode == TERRAIN_FRAME && !inputs.supports_terrain_frame => NO_TERRAIN_FRAME,
        _ => NO_ITEMS_YET,
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

pub fn altitude_modes_view(backend: &dyn Backend, args: &[String]) -> Value {
    let vehicles = object(&backend.get_fields("vehicles", "activeVehicleAvailable"));
    let vehicle = object(&backend.get_fields("vehicle", "supportsTerrainFrame"));
    let mission = object(&backend.get_fields("plan.missionController", "containsItems"));
    let options = object(&backend.get_fields("corePlugin.options", "showMissionAbsoluteAltitude"));
    let inputs = Inputs {
        mission: args.first().map(|a| a != "item").unwrap_or(true),
        current: args.get(1).and_then(|a| a.trim().parse().ok()).unwrap_or(-1),
        supports_terrain_frame: flag(&vehicles, "activeVehicleAvailable") && flag(&vehicle, "supportsTerrainFrame"),
        has_items: flag(&mission, "containsItems"),
        show_absolute: options.get("showMissionAbsoluteAltitude").and_then(Value::as_bool).unwrap_or(true),
    };
    json!({
        "kind": "object",
        "class": "AltitudeModes",
        "context": if inputs.mission { "mission" } else { "item" },
        "current": inputs.current,
        "supportsTerrainFrame": inputs.supports_terrain_frame,
        "modes": modes(&inputs),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Inputs {
        Inputs { mission: true, current: RELATIVE, supports_terrain_frame: true, has_items: true, show_absolute: true }
    }

    fn raws(inputs: &Inputs) -> Vec<i64> {
        modes(inputs).iter().map(|m| m["raw"].as_i64().unwrap()).collect()
    }

    #[test]
    fn a_vehicle_that_cannot_hold_an_altitude_above_terrain_is_not_offered_it() {
        assert_eq!(raws(&base()), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN, TERRAIN_FRAME, MIXED]);
        let plain = Inputs { supports_terrain_frame: false, ..base() };
        assert_eq!(raws(&plain), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN, MIXED], "offering a mode the firmware cannot fly is an offer without its gate");
        let already_on_it = Inputs { supports_terrain_frame: false, current: TERRAIN_FRAME, ..base() };
        assert!(raws(&already_on_it).contains(&TERRAIN_FRAME), "a mode the plan is already set to stays listed, so an operator can see what they are on and leave it");
    }

    #[test]
    fn an_item_menu_never_offers_mixed_and_a_mission_menu_always_does() {
        assert!(raws(&base()).contains(&MIXED));
        let item = Inputs { mission: false, ..base() };
        assert!(!raws(&item).contains(&MIXED), "mixed means each item chooses, which is not a choice an item can make");
        let item_without_terrain = Inputs { mission: false, supports_terrain_frame: false, ..base() };
        assert_eq!(raws(&item_without_terrain), [RELATIVE, ABSOLUTE, CALC_ABOVE_TERRAIN]);
    }

    #[test]
    fn an_empty_plan_shows_the_modes_but_will_not_let_one_be_chosen() {
        let empty = Inputs { has_items: false, ..base() };
        let listed = modes(&empty);
        let enabled_raws: Vec<i64> = listed.iter().filter(|m| m["enabled"] == true).map(|m| m["raw"].as_i64().unwrap()).collect();
        assert_eq!(enabled_raws, [RELATIVE, MIXED], "only the mode already chosen and mixed stay live until the plan has something to measure");
        assert_eq!(listed[1]["reason"], NO_ITEMS_YET);
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
                "vehicle" => json!({ "kind": "object", "supportsTerrainFrame": self.terrain }).to_string(),
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
        let view = altitude_modes_view(&Fake { terrain: false, items: true }, &["mission".to_string(), "1".to_string()]);
        assert_eq!(view["context"], "mission");
        assert_eq!(view["supportsTerrainFrame"], false);
        assert_eq!(view["modes"].as_array().unwrap().len(), 4);
        assert_eq!(view["current"], RELATIVE);
        let absent = altitude_modes_view(&Fake { terrain: true, items: false }, &[]);
        assert_eq!(absent["current"], -1, "a head that names no current mode is not told one is chosen");
        assert!(absent["modes"].as_array().unwrap().iter().all(|m| m["current"] == false));
        assert_eq!(absent["modes"].as_array().unwrap().iter().filter(|m| m["enabled"] == true).count(), 1, "with no items and no current mode only mixed stays live");
    }
}
