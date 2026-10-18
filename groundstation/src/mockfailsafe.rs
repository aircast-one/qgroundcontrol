use mavlink::dialects::ardupilotmega::MavSeverity;

use crate::mockbattery::Battery;
use crate::mocktuning::{Level, LinkLoss, Reaction};

#[derive(Debug, Clone, PartialEq)]
pub struct Alarm {
    pub severity: MavSeverity,
    pub text: String,
    pub reaction: Reaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Situation {
    pub auto: bool,
    pub pilot: bool,
    pub landing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Watch {
    link_lost: bool,
    fence_out: bool,
    battery_levels: usize,
    navigation_lost: bool,
}

fn worded(px4: bool, ardupilot: &str, px4_text: &str) -> String {
    if px4 { px4_text } else { ardupilot }.to_string()
}

impl Watch {
    pub fn battery(
        &mut self,
        levels: &[Level],
        battery: Battery,
        capacity_mah: f64,
        cells: f64,
        armed: bool,
        px4: bool,
    ) -> Option<Alarm> {
        let volts = battery.volts(capacity_mah, cells);
        let crossed = |level: &Level| {
            (level.mah_left > 0.0 && battery.left_mah(capacity_mah) <= level.mah_left)
                || (level.volts > 0.0 && volts <= level.volts)
                || (level.fraction > 0.0 && battery.fraction(capacity_mah) <= level.fraction)
        };
        let reached = levels.iter().rposition(crossed).map_or(0, |at| at + 1);
        (armed && reached > self.battery_levels).then(|| {
            self.battery_levels = reached;
            let level = levels[reached - 1];
            let text = match (px4, level.name) {
                (false, name) => format!(
                    "Battery 1 is {name} {volts:.2}V used {:.0} mAh",
                    battery.consumed_mah
                ),
                (true, "low") => "Low battery level! Return advised".to_string(),
                (true, "critical") => "Critical battery level! Landing advised".to_string(),
                (true, _) => "Dangerous battery level! Landing immediately".to_string(),
            };
            Alarm {
                severity: MavSeverity::MAV_SEVERITY_CRITICAL,
                text,
                reaction: level.reaction,
            }
        })
    }

    pub fn link(
        &mut self,
        loss: Option<LinkLoss>,
        quiet_s: Option<f64>,
        now: Situation,
        armed: bool,
        px4: bool,
    ) -> Option<Alarm> {
        let loss = loss?;
        let lost = quiet_s.is_some_and(|quiet| quiet >= loss.after_s);
        let was = std::mem::replace(&mut self.link_lost, lost);
        let spared = (loss.keeps_auto && now.auto)
            || (loss.keeps_pilot && now.pilot)
            || (loss.keeps_landing && now.landing);
        match (lost, was) {
            (true, false) => Some(Alarm {
                severity: MavSeverity::MAV_SEVERITY_CRITICAL,
                text: worded(px4, "GCS Failsafe", "Connection to ground station lost"),
                reaction: if armed && !spared {
                    loss.reaction
                } else {
                    Reaction::Warn
                },
            }),
            (false, true) => Some(Alarm {
                severity: MavSeverity::MAV_SEVERITY_INFO,
                text: worded(px4, "GCS Failsafe Cleared", "Data link regained"),
                reaction: Reaction::Warn,
            }),
            _ => None,
        }
    }

    pub fn fence(&mut self, outside: bool, reaction: Reaction, px4: bool) -> Option<Alarm> {
        let was = std::mem::replace(&mut self.fence_out, outside);
        (outside && !was).then(|| Alarm {
            severity: MavSeverity::MAV_SEVERITY_CRITICAL,
            text: worded(px4, "Fence Breached", "Geofence violation"),
            reaction,
        })
    }

    pub fn navigation(&mut self, lost: bool, gps_lost: bool, px4: bool) -> Option<Alarm> {
        let was = self.navigation_lost;
        self.navigation_lost = lost || (was && gps_lost);
        (lost && !was).then(|| Alarm {
            severity: MavSeverity::MAV_SEVERITY_CRITICAL,
            text: worded(
                px4,
                "EKF Failsafe",
                "Navigation failure! Land and recalibrate the sensors",
            ),
            reaction: Reaction::Land,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOSS: LinkLoss = LinkLoss {
        after_s: 5.0,
        reaction: Reaction::Return,
        keeps_auto: false,
        keeps_pilot: true,
        keeps_landing: false,
    };

    #[test]
    fn a_lost_link_alarms_once_spares_a_pilot_mode_and_clears_when_heard_again() {
        let mut watch = Watch::default();
        let flying = Situation::default();
        assert_eq!(watch.link(Some(LOSS), Some(4.9), flying, true, false), None);
        assert_eq!(
            watch
                .link(Some(LOSS), Some(5.0), flying, true, false)
                .map(|a| (a.text, a.reaction)),
            Some(("GCS Failsafe".to_string(), Reaction::Return))
        );
        assert_eq!(
            watch.link(Some(LOSS), Some(6.0), flying, true, false),
            None,
            "it alarms once per loss"
        );
        assert_eq!(
            watch
                .link(Some(LOSS), Some(0.0), flying, true, false)
                .map(|a| a.reaction),
            Some(Reaction::Warn),
            "hearing the GCS again clears it"
        );
        let piloting = Situation {
            pilot: true,
            ..Situation::default()
        };
        assert_eq!(
            watch
                .link(Some(LOSS), Some(5.0), piloting, true, true)
                .map(|a| (a.text, a.reaction)),
            Some((
                "Connection to ground station lost".to_string(),
                Reaction::Warn
            )),
            "FS_OPTIONS spares a pilot mode, and PX4 words it its own way"
        );
        assert_eq!(
            Watch::default().link(None, Some(60.0), flying, true, false),
            None,
            "no failsafe configured, no alarm"
        );
    }

    #[test]
    fn a_battery_alarm_names_the_deepest_level_crossed_and_never_repeats_it() {
        let levels = [
            Level {
                name: "low",
                mah_left: 1000.0,
                volts: 0.0,
                fraction: 0.0,
                reaction: Reaction::Return,
            },
            Level {
                name: "critical",
                mah_left: 500.0,
                volts: 0.0,
                fraction: 0.0,
                reaction: Reaction::Land,
            },
        ];
        let mut watch = Watch::default();
        let spent = |mah: f64| Battery {
            consumed_mah: mah,
            current_a: 0.0,
        };
        assert_eq!(
            watch.battery(&levels, spent(4200.0), 5000.0, 4.0, false, false),
            None,
            "a disarmed vehicle raises no battery failsafe"
        );
        assert_eq!(
            watch
                .battery(&levels, spent(4200.0), 5000.0, 4.0, true, false)
                .map(|a| a.reaction),
            Some(Reaction::Return)
        );
        assert_eq!(
            watch.battery(&levels, spent(4300.0), 5000.0, 4.0, true, false),
            None
        );
        assert!(
            watch
                .battery(&levels, spent(4600.0), 5000.0, 4.0, true, false)
                .is_some_and(
                    |a| a.text.starts_with("Battery 1 is critical") && a.reaction == Reaction::Land
                )
        );
    }
}
