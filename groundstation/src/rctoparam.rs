use serde_json::{Value, json};

use crate::mavout::Outbound;

pub const SET_RC_TO_PARAM: &str = "parameters.setRcToParam";
pub const CLEAR_RC_TO_PARAM: &str = "parameters.clearAllRcToParam";
const PARAM_INDEX_BY_NAME: i16 = -1;
const PARAM_INDEX_DISABLE: i16 = -2;
const TUNING_IDS: u8 = 3;

pub fn set_message(target: (u8, u8), name: &str, scale: f64, center: f64, tuning: u8, min: f64, max: f64) -> Outbound {
    Outbound::ParamMapRc { target, id: name.to_string(), index: PARAM_INDEX_BY_NAME, tuning, center: center as f32, scale: scale as f32, min: min as f32, max: max as f32 }
}

pub fn clear_messages(target: (u8, u8)) -> Vec<Outbound> {
    (0..TUNING_IDS).map(|tuning| Outbound::ParamMapRc { target, id: String::new(), index: PARAM_INDEX_DISABLE, tuning, center: 0.0, scale: 0.0, min: 0.0, max: 0.0 }).collect()
}

fn px4_target() -> Result<((u8, u8), u32), &'static str> {
    let hub = crate::hub::lock();
    let vehicle = hub.active().ok_or("No vehicle is connected.")?;
    if vehicle.autopilot != crate::modes::AUTOPILOT_PX4 {
        return Err("RC to Param is a PX4 feature.");
    }
    Ok(((vehicle.id, crate::hub::COMP_AUTOPILOT1), vehicle.link))
}

fn send(link: u32, outbound: &[Outbound]) {
    outbound.iter().filter_map(crate::mavout::encode_next).for_each(|bytes| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, link, &bytes);
    });
}

pub fn run(backend: &dyn crate::router::Backend, path: &str, args: &str) -> Value {
    if !crate::vehiclefacade::switched_on() {
        let method = if path == CLEAR_RC_TO_PARAM { "vehicle.clearAllParamMapRC" } else { "vehicle.sendParamMapRC" };
        return crate::read::object(&backend.invoke(method, args));
    }
    let (target, link) = match px4_target() {
        Ok(found) => found,
        Err(reason) => return json!({ "ok": false, "reason": reason }),
    };
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let number = |at: usize| given.get(at).and_then(Value::as_f64).filter(|v| v.is_finite());
    let outbound = match path {
        CLEAR_RC_TO_PARAM => clear_messages(target),
        _ => match (given.first().and_then(Value::as_str), number(1), number(2), number(3).filter(|t| (0.0..f64::from(TUNING_IDS)).contains(t)), number(4), number(5)) {
            (Some(name), Some(scale), Some(center), Some(tuning), Some(min), Some(max)) if !name.is_empty() => vec![set_message(target, name, scale, center, tuning as u8, min, max)],
            _ => return json!({ "ok": false, "reason": "Set RC to Param takes a parameter, scale, center value, tuning ID and min/max values." }),
        },
    };
    send(link, &outbound);
    json!({ "ok": true })
}

pub fn owns(path: &str) -> bool {
    [SET_RC_TO_PARAM, CLEAR_RC_TO_PARAM].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_match_send_param_map_rc_and_clear_all() {
        assert_eq!(set_message((1, 1), "MC_ROLL_P", 0.5, 6.5, 2, 0.0, 12.0), Outbound::ParamMapRc { target: (1, 1), id: "MC_ROLL_P".into(), index: -1, tuning: 2, center: 6.5, scale: 0.5, min: 0.0, max: 12.0 });
        let cleared = clear_messages((1, 1));
        assert_eq!(cleared.len(), 3, "clearAllParamMapRC disables tuning ids 0, 1 and 2");
        assert!(cleared.iter().enumerate().all(|(i, m)| matches!(m, Outbound::ParamMapRc { index: -2, tuning, .. } if usize::from(*tuning) == i)));
        assert!(crate::mavout::encode_next(&cleared[0]).is_some());
    }
}
