use serde_json::{Value, json};

use crate::router::Backend;

pub const VIEW: &str = "view.autoIntercept";
pub const DROP: &str = "vehicle.autoInterceptDrop";
pub const PICK_ANOTHER: &str = "vehicle.autoInterceptPickAnother";
pub const PICK: &str = "vehicle.cameraManager.currentCameraInstance.startTracking";
pub const TRACKING_ARMED: &str = "vehicle.cameraManager.currentCameraInstance.trackingEnabled";
pub const STOP_TRACKING: &str = "vehicle.cameraManager.currentCameraInstance.stopTracking";
pub const TRACKING_HINT: &str = "Tap or drag around what to track";
const TITLE: &str = "Auto Intercept";
const FIXED_TARGETS: [&str; 2] = ["marked point", "gimbal point"];
const NOTHING_TO_CHASE: &str =
    "Nothing to chase yet — track a target with the camera or mark one on the map.";

fn text<'a>(json: &'a Value, key: &str) -> &'a str {
    json.get(key).and_then(Value::as_str).unwrap_or("")
}

fn number(json: &Value, key: &str) -> Option<f64> {
    json.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
}

fn whole(value: f64) -> String {
    format!("{}", value.round() as i64)
}

fn target_name(progress: &Value) -> &str {
    Some(text(progress, "target"))
        .filter(|name| !name.is_empty())
        .unwrap_or("target")
}

fn waiting_text(progress: &Value) -> String {
    if progress.is_null() {
        return "Auto Intercept starting".to_string();
    }
    let target = target_name(progress);
    let left = format!(
        " · {} s left",
        progress
            .get("secondsLeft")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    );
    match text(progress, "holding") {
        "climbing" => format!("Chasing the {target} · climbing to speed"),
        "searching" => format!("Lost the {target} — tap it to pick it up again{left}"),
        "turning" => format!("Holding: the {target} is turning{left}"),
        "junction" => format!("Holding: junction ahead{left}"),
        "unseen" => format!("Holding: waiting to see the {target} again{left}"),
        "offTarget" => format!("Holding: not lined up yet{left}"),
        "outsideDropZone" => format!("Holding: it would land outside the drop zone{left}"),
        _ => [
            Some(format!("Chasing the {target}")),
            number(progress, "distanceMeters").map(|m| format!("{} m", whole(m))),
            number(progress, "speedMps").map(|v| format!("{} m/s", whole(v))),
            number(progress, "releaseInSeconds")
                .map(|s| format!("release in ~{} s", whole(s.max(1.0)))),
            (!FIXED_TARGETS.contains(&target)
                && progress.get("moving").and_then(Value::as_bool) == Some(false))
            .then(|| "not moving — is this the right one?".to_string()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · "),
    }
}

pub fn confirm_text(target: &Value, payload: &Value) -> String {
    let label = text(target, "label");
    if label.is_empty() {
        return NOTHING_TO_CHASE.to_string();
    }
    let source = text(target, "source");
    let what = match source {
        "tracked" => format!("the {label} you're tracking"),
        "manual" => "the marked point".to_string(),
        "vehicle" => label.to_string(),
        _ => "the point the camera is aimed at".to_string(),
    };
    let moving = number(target, "speedMps").unwrap_or(0.0) >= 1.0;
    let about = [
        number(target, "distanceMeters").map(|m| format!("{} m", whole(m))),
        ["tracked", "vehicle"]
            .contains(&source)
            .then(|| if moving { "moving" } else { "not moving" }.to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let about = if about.is_empty() {
        String::new()
    } else {
        format!(" ({about})")
    };
    let bays = payload.get("bays").and_then(Value::as_i64).unwrap_or(1);
    let drop = match payload.get("bay").and_then(Value::as_i64) {
        Some(bay) if bays > 1 => format!("bay {bay}"),
        _ => "the payload".to_string(),
    };
    let loaded = match payload.is_object() {
        true => format!(
            " · {} loaded",
            payload.get("left").and_then(Value::as_i64).unwrap_or(0)
        ),
        false => String::new(),
    };
    format!(
        "Chase {what}{about} and drop {drop} when it lines up{loaded}. The drone steers and drops by itself."
    )
}

fn action(id: &str, label: &str, path: &str, args: Value, confirm: Option<&str>) -> Value {
    json!({
        "id": id,
        "label": label,
        "path": path,
        "args": args,
        "confirm": confirm.map(|text| json!({ "title": TITLE, "text": text })),
    })
}

fn fired(status: &Value) -> (String, bool, Vec<&'static str>) {
    let target = status.get("progress").map(target_name).unwrap_or("target");
    let left = status["payload"]
        .get("left")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let on_target = status
        .get("estimatedOnTarget")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let seen = text(status, "impact") == "seen";
    let outcome = match text(status, "impact") {
        "seen" if on_target => format!("Hit the {target}"),
        "seen" => format!(
            "Missed by about {} m",
            whole(
                number(status, "estimatedMissMeters")
                    .unwrap_or(0.0)
                    .max(1.0)
            )
        ),
        "unseen" => "Dropped — couldn't see where it landed".to_string(),
        _ => "Dropped — falling".to_string(),
    };
    let remaining = match left > 0 {
        true => format!("{left} left"),
        false => "no bottles left".to_string(),
    };
    (
        format!("{outcome} · {remaining}"),
        seen && !on_target,
        (left > 0).then_some("dropAgain").into_iter().collect(),
    )
}

pub fn shown(status: &Value, target: &Value, tracking: &Value) -> Value {
    let state = text(status, "state");
    let locked = tracking.get("supported").and_then(Value::as_bool) == Some(true)
        && tracking.get("reported").and_then(Value::as_bool) == Some(true);
    let picking = tracking.get("supported").and_then(Value::as_bool) == Some(true)
        && tracking.get("requested").and_then(Value::as_bool) == Some(true)
        && tracking["shapes"].as_array().is_some_and(|s| !s.is_empty());
    let line: Option<(String, bool, Vec<&str>)> = match state {
        "waiting" => Some((
            waiting_text(&status["progress"]),
            status["progress"].get("searching").and_then(Value::as_bool) == Some(true),
            vec!["stop"],
        )),
        "fired" => Some(fired(status)),
        "gaveUp" => Some((
            match status.get("timedOut").and_then(Value::as_bool) == Some(true) {
                true => format!(
                    "Couldn't line up a safe drop in {} s — holding position",
                    whole(crate::autodrop::MAX_RUN_S)
                ),
                false => format!(
                    "Auto Intercept stopped: {} — holding position",
                    text(status, "reason")
                ),
            },
            true,
            vec!["tryAgain", "pickAnother"],
        )),
        _ => locked.then(|| ("Locked on".to_string(), false, vec!["dropOnIt"])),
    };
    let confirm = confirm_text(target, &status["payload"]);
    let make = |id: &str| match id {
        "stop" => action(id, "Stop", DROP, json!(["stop"]), None),
        "pickAnother" => action(id, "Pick another target", PICK_ANOTHER, json!([]), None),
        "tryAgain" => action(id, "Try again", DROP, json!([]), Some(&confirm)),
        "dropAgain" => action(id, "Drop again", DROP, json!([]), Some(&confirm)),
        _ => action(id, "Drop on it", DROP, json!([]), Some(&confirm)),
    };
    let entry = match state == "waiting" {
        true => action("stop", "Stop Auto Intercept", DROP, json!(["stop"]), None),
        false => action("start", TITLE, DROP, json!([]), Some(&confirm)),
    };
    let (headline, problem, ids) = line.unwrap_or_default();
    json!({
        "kind": "object",
        "class": "AutoIntercept",
        "visible": !headline.is_empty(),
        "tone": if problem { "problem" } else { "info" },
        "headline": headline,
        "actions": ids.into_iter().map(make).collect::<Vec<_>>(),
        "entry": entry,
        "hint": (picking && !locked && state != "waiting").then_some(TRACKING_HINT),
        "picking": picking,
        "pickPath": PICK,
    })
}

pub fn deps() -> Vec<String> {
    crate::video::CAMERA_DEPS
        .iter()
        .copied()
        .chain(["vehicle.autodropTarget"])
        .map(str::to_string)
        .collect()
}

pub fn view(backend: &dyn Backend, _args: &[String]) -> Value {
    let status = crate::autodrop::status_view(backend, &[]);
    let target = backend.value("vehicle.autodropTarget")["value"].clone();
    let tracking = crate::video::camera_view(backend, &[])["tracking"].clone();
    shown(&status, &target, &tracking)
}

pub fn pick_another(backend: &dyn Backend) -> Value {
    backend.invoke(STOP_TRACKING, "[]");
    let armed =
        crate::read::object(&backend.set(TRACKING_ARMED, &json!({ "value": true }).to_string()));
    match armed["ok"] == true {
        true => json!({ "ok": true }),
        false => json!({
            "ok": false,
            "reason": Some(text(&armed, "reason"))
                .filter(|reason| !reason.is_empty())
                .unwrap_or("This camera cannot pick a target to track."),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(status: Value) -> (String, String, Vec<String>) {
        let shown = shown(&status, &Value::Null, &Value::Null);
        (
            shown["headline"].as_str().unwrap().to_string(),
            shown["tone"].as_str().unwrap().to_string(),
            shown["actions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["id"].as_str().unwrap().to_string())
                .collect(),
        )
    }

    fn headline(status: Value) -> String {
        line(status).0
    }

    #[test]
    fn auto_intercept_says_what_it_chases_why_it_waits_how_it_ended_and_what_to_do_next() {
        let idle = shown(&json!({"state":"idle"}), &Value::Null, &Value::Null);
        assert_eq!(idle["visible"], false, "idle has nothing to say");
        let locked = shown(
            &json!({"state":"idle"}),
            &Value::Null,
            &json!({"supported":true,"requested":true,"reported":true,"shapes":["point"]}),
        );
        assert_eq!(
            (
                &locked["headline"],
                &locked["actions"][0]["id"],
                &locked["actions"][0]["label"]
            ),
            (
                &json!("Locked on"),
                &json!("dropOnIt"),
                &json!("Drop on it")
            )
        );
        assert_eq!(
            line(
                json!({"state":"waiting","progress":{"target":"car","distanceMeters":45.2,"speedMps":5.0,"moving":true,"releaseInSeconds":3.2,"holding":null,"secondsLeft":71}})
            ),
            (
                "Chasing the car · 45 m · 5 m/s · release in ~3 s".to_string(),
                "info".to_string(),
                vec!["stop".to_string()]
            )
        );
        assert_eq!(
            headline(
                json!({"state":"waiting","progress":{"target":"car","distanceMeters":60,"speedMps":0.1,"moving":false,"releaseInSeconds":9,"secondsLeft":71}})
            ),
            "Chasing the car · 60 m · 0 m/s · release in ~9 s · not moving — is this the right one?"
        );
        assert_eq!(
            headline(
                json!({"state":"waiting","progress":{"target":"marked point","distanceMeters":60,"speedMps":0,"moving":false,"secondsLeft":71}})
            ),
            "Chasing the marked point · 60 m · 0 m/s",
            "a fixed point is never asked whether it should be moving"
        );
        [
            ("climbing", "Chasing the car · climbing to speed"),
            ("turning", "Holding: the car is turning · 52 s left"),
            ("junction", "Holding: junction ahead · 52 s left"),
            ("unseen", "Holding: waiting to see the car again · 52 s left"),
            ("offTarget", "Holding: not lined up yet · 52 s left"),
            ("outsideDropZone", "Holding: it would land outside the drop zone · 52 s left"),
        ]
        .into_iter()
        .for_each(|(holding, said)| {
            assert_eq!(
                headline(json!({"state":"waiting","progress":{"target":"car","holding":holding,"secondsLeft":52}})),
                said
            )
        });
        assert_eq!(
            headline(json!({"state":"waiting","progress":null})),
            "Auto Intercept starting"
        );
        assert_eq!(
            line(
                json!({"state":"waiting","progress":{"target":"car","holding":"searching","searching":true,"secondsLeft":40}})
            ),
            (
                "Lost the car — tap it to pick it up again · 40 s left".to_string(),
                "problem".to_string(),
                vec!["stop".to_string()]
            )
        );
        assert_eq!(
            line(
                json!({"state":"fired","impact":"seen","estimatedMissMeters":0.7,"estimatedOnTarget":true,"progress":{"target":"car"},"payload":{"bays":4,"bay":2,"left":3}})
            ),
            (
                "Hit the car · 3 left".to_string(),
                "info".to_string(),
                vec!["dropAgain".to_string()]
            )
        );
        assert_eq!(
            line(
                json!({"state":"fired","impact":"seen","estimatedMissMeters":2.6,"estimatedOnTarget":false,"progress":{"target":"car"},"payload":{"bays":1,"bay":null,"left":0}})
            ),
            (
                "Missed by about 3 m · no bottles left".to_string(),
                "problem".to_string(),
                vec![]
            )
        );
        assert_eq!(
            headline(
                json!({"state":"fired","impact":"falling","progress":{"target":"car"},"payload":{"left":2}})
            ),
            "Dropped — falling · 2 left"
        );
        assert_eq!(
            headline(
                json!({"state":"fired","impact":"unseen","progress":{"target":"car"},"payload":{"left":0}})
            ),
            "Dropped — couldn't see where it landed · no bottles left"
        );
        assert_eq!(
            line(
                json!({"state":"gaveUp","reason":"no release window opened within the time limit","timedOut":true})
            ),
            (
                "Couldn't line up a safe drop in 90 s — holding position".to_string(),
                "problem".to_string(),
                vec!["tryAgain".to_string(), "pickAnother".to_string()]
            )
        );
        assert_eq!(
            headline(json!({"state":"gaveUp","reason":"stopped by the pilot","timedOut":false})),
            "Auto Intercept stopped: stopped by the pilot — holding position"
        );
    }

    #[test]
    fn every_button_says_where_it_goes_and_only_a_start_asks_first() {
        let fired = shown(
            &json!({"state":"fired","impact":"seen","estimatedOnTarget":true,"progress":{"target":"car"},"payload":{"bays":4,"bay":2,"left":3}}),
            &json!({"label":"car","source":"tracked","speedMps":5.0,"distanceMeters":61.6}),
            &Value::Null,
        );
        assert_eq!(
            (
                &fired["actions"][0]["path"],
                &fired["actions"][0]["args"],
                &fired["actions"][0]["confirm"]["title"]
            ),
            (&json!(DROP), &json!([]), &json!("Auto Intercept"))
        );
        let waiting = shown(
            &json!({"state":"waiting","progress":null}),
            &Value::Null,
            &Value::Null,
        );
        assert_eq!(
            (
                &waiting["actions"][0]["args"],
                &waiting["actions"][0]["confirm"]
            ),
            (&json!(["stop"]), &Value::Null),
            "Stop goes out on the tap"
        );
        assert_eq!(
            (&waiting["entry"]["label"], &waiting["entry"]["args"]),
            (&json!("Stop Auto Intercept"), &json!(["stop"])),
            "the More tile stops a run in flight"
        );
        let gave_up = shown(
            &json!({"state":"gaveUp","timedOut":true}),
            &Value::Null,
            &Value::Null,
        );
        assert_eq!(
            (&gave_up["actions"][1]["path"], &gave_up["entry"]["label"]),
            (&json!(PICK_ANOTHER), &json!("Auto Intercept"))
        );
    }

    #[test]
    fn the_confirmation_names_the_target_and_the_bay() {
        assert_eq!(
            confirm_text(
                &json!({"label":"car","source":"tracked","speedMps":5.0,"distanceMeters":61.6}),
                &json!({"bays":4,"bay":2,"left":3})
            ),
            "Chase the car you're tracking (62 m, moving) and drop bay 2 when it lines up · 3 loaded. The drone steers and drops by itself."
        );
        assert_eq!(
            confirm_text(
                &json!({"label":"marked point","source":"manual","speedMps":0,"distanceMeters":120}),
                &json!({"bays":1,"bay":0,"left":1})
            ),
            "Chase the marked point (120 m) and drop the payload when it lines up · 1 loaded. The drone steers and drops by itself."
        );
        assert_eq!(confirm_text(&Value::Null, &Value::Null), NOTHING_TO_CHASE);
    }

    #[test]
    fn the_picture_asks_for_a_target_only_while_tracking_is_armed_nothing_is_locked_and_no_run_is_going()
     {
        let tracking = |requested: bool, reported: bool| json!({"supported":true,"requested":requested,"reported":reported,"shapes":["rectangle","point"]});
        let hint = |state: &str, tracking: Value| {
            shown(&json!({ "state": state }), &Value::Null, &tracking)["hint"].clone()
        };
        assert_eq!(hint("idle", tracking(true, false)), json!(TRACKING_HINT));
        assert_eq!(
            hint("idle", tracking(true, true)),
            Value::Null,
            "already locked"
        );
        assert_eq!(
            hint("idle", tracking(false, false)),
            Value::Null,
            "not armed"
        );
        assert_eq!(
            hint("waiting", tracking(true, false)),
            Value::Null,
            "a run is going"
        );
        assert_eq!(
            shown(
                &json!({"state":"idle"}),
                &Value::Null,
                &tracking(true, false)
            )["picking"],
            true
        );
    }
}
