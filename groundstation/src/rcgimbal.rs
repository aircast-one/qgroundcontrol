use serde_json::{Value, json};

use crate::read::flag;
use crate::router::Backend;

pub const PAN_CHANNEL: &str = "settings.flyViewSettings.gimbalPanChannel";
const TILT_CHANNEL: &str = "settings.flyViewSettings.gimbalTiltChannel";

fn channel(backend: &dyn Backend, path: &str) -> Option<i64> {
    crate::read::value_number(&backend.value(&format!("{path}.rawValue")))
        .map(|channel| channel as i64)
        .filter(|channel| *channel > 0)
}

pub fn pan_channel(backend: &dyn Backend) -> Option<i64> {
    channel(backend, PAN_CHANNEL)
}

pub fn curve(deflection: f64) -> f64 {
    let held = deflection.clamp(-1.0, 1.0);
    held * held.abs()
}

pub fn stick_pwms(pan: Option<i64>, tilt: Option<i64>, x: f64, y: f64) -> Vec<(i64, i64)> {
    let centre = f64::from(crate::joystick::PWM_CENTER);
    let span = f64::from(crate::joystick::PWM_HALF_SPAN);
    [(pan.filter(|pan| Some(*pan) != tilt), x), (tilt, -y)]
        .into_iter()
        .filter_map(|(channel, deflection)| {
            channel.map(|channel| (channel, (centre + curve(deflection) * span).round() as i64))
        })
        .collect()
}

static RADIO_TAKES_BACK: std::sync::Mutex<Option<(i64, bool)>> = std::sync::Mutex::new(None);

fn receiver_healthy(backend: &dyn Backend) -> bool {
    let info = backend.value("vehicle.sysStatusSensorInfo");
    info["sensorNames"]
        .as_array()
        .and_then(|names| names.iter().position(|name| name == "RC Receiver"))
        .and_then(|at| info["sensorHealthy"].get(at).cloned())
        .is_some_and(|healthy| {
            healthy
                .as_bool()
                .unwrap_or(healthy.as_i64().unwrap_or(0) != 0)
        })
}

fn held_channels(backend: &dyn Backend) -> Vec<i64> {
    backend.value("vehicle.rcChannelOverrides")["value"]
        .as_object()
        .map(|held| {
            held.keys()
                .filter_map(|channel| channel.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

pub fn radio_takes_back(held: bool, receiver_healthy: bool, remembered: Option<bool>) -> bool {
    match held {
        false => receiver_healthy,
        true => remembered.unwrap_or(false),
    }
}

pub fn stick(backend: &dyn Backend, x: f64, y: f64) -> Option<Value> {
    let centre = f64::from(crate::joystick::PWM_CENTER) as i64;
    let pwms = stick_pwms(
        channel(backend, PAN_CHANNEL),
        channel(backend, TILT_CHANNEL),
        x,
        y,
    );
    (!pwms.is_empty()).then(|| {
        let vehicle = crate::guided::active_id(backend);
        let held = held_channels(backend);
        let mut memory = RADIO_TAKES_BACK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let radio = radio_takes_back(
            pwms.iter().any(|(channel, _)| held.contains(channel)),
            receiver_healthy(backend),
            memory
                .filter(|(known, _)| Some(*known) == vehicle)
                .map(|(_, radio)| radio),
        );
        *memory = vehicle.map(|known| (known, radio));
        drop(memory);
        pwms.iter()
            .map(|(channel, pwm)| match *pwm == centre && radio {
                true => crate::rcoverride::release(
                    backend,
                    crate::rcoverride::RELEASE_CHANNEL,
                    &json!([channel]).to_string(),
                ),
                false => crate::rcoverride::set(
                    backend,
                    crate::rcoverride::SET,
                    &json!([channel, pwm]).to_string(),
                ),
            })
            .find(|sent| !flag(sent, "ok"))
            .unwrap_or_else(|| json!({ "ok": true }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stick_on_rc_camera_channels_holds_their_speed_and_centres_back_to_stop() {
        assert_eq!(
            stick_pwms(Some(13), Some(14), 1.0, 0.0),
            vec![(13, 2000), (14, 1500)],
            "right at full stick is the pan channel's top speed"
        );
        assert_eq!(
            stick_pwms(Some(13), Some(14), 0.0, 1.0),
            vec![(13, 1500), (14, 1000)],
            "up is the low end: owel-red-01's Viewpro tilts down at 1900"
        );
        assert_eq!(
            stick_pwms(Some(13), Some(14), 0.5, -0.5),
            vec![(13, 1625), (14, 1625)],
            "halfway is a quarter of the speed, like the gimbal's own stick"
        );
        assert_eq!(
            stick_pwms(Some(14), Some(14), 1.0, 1.0),
            vec![(14, 1000)],
            "a pan set on the tilt channel is ignored, as the camera rail ignores it"
        );
        assert_eq!(stick_pwms(None, None, 1.0, 1.0), vec![]);
    }

    #[test]
    fn the_radio_gets_the_camera_back_only_if_it_was_healthy_before_the_app_took_the_channels() {
        assert!(radio_takes_back(false, true, None));
        assert!(
            !radio_takes_back(false, false, Some(true)),
            "nothing held: the receiver decides afresh"
        );
        assert!(
            !radio_takes_back(true, true, None),
            "while the app holds a channel the receiver reads healthy on the app's overrides, so it proves nothing"
        );
        assert!(radio_takes_back(true, true, Some(true)));
        assert!(!radio_takes_back(true, true, Some(false)));
    }
}
