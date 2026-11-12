use std::collections::BTreeMap;

use crate::radiocamera::Layout;

const RC_CHANNELS: u8 = 16;
const SERVO_OUTPUTS: u8 = 32;
const ARDUPILOT_RCIN1: f64 = 51.0;
const PX4_SWITCHES: [&str; 11] = [
    "RC_MAP_ARM_SW",
    "RC_MAP_KILL_SW",
    "RC_MAP_RETURN_SW",
    "RC_MAP_LOITER_SW",
    "RC_MAP_OFFB_SW",
    "RC_MAP_AUX1",
    "RC_MAP_AUX2",
    "RC_MAP_AUX3",
    "RC_MAP_AUX4",
    "RC_MAP_AUX5",
    "RC_MAP_AUX6",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    Stick,
    FlightModes,
    Option { param: String, value: u16 },
    Switch { param: String },
    Passthrough { param: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DroneFacts {
    pub claims: BTreeMap<usize, Claim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub channel: usize,
    pub names: Vec<&'static str>,
    pub claim: Claim,
}

fn channel(value: Option<f64>) -> Option<usize> {
    value
        .filter(|c| (1.0..=255.0).contains(c))
        .map(|c| c as usize)
}

impl DroneFacts {
    pub fn read(
        vehicle_type: u8,
        px4: bool,
        parameter: impl Fn(&str) -> Option<f64>,
    ) -> DroneFacts {
        let roles = crate::usbradio::roles(vehicle_type, px4, &parameter);
        let sticks = roles
            .map(|roles| roles.channels.map(usize::from).to_vec())
            .unwrap_or_default()
            .into_iter()
            .map(|at| (at, Claim::Stick));
        let modes = roles
            .map(|roles| usize::from(roles.mode_channel))
            .filter(|at| *at > 0)
            .map(|at| (at, Claim::FlightModes));
        let options = (1..=RC_CHANNELS).filter_map(|at| {
            let param = format!("RC{at}_OPTION");
            parameter(&param)
                .filter(|value| *value != 0.0 && !px4)
                .map(|value| {
                    (
                        usize::from(at),
                        Claim::Option {
                            param,
                            value: value as u16,
                        },
                    )
                })
        });
        let switches = PX4_SWITCHES.iter().filter(|_| px4).filter_map(|param| {
            channel(parameter(param)).map(|at| {
                (
                    at,
                    Claim::Switch {
                        param: param.to_string(),
                    },
                )
            })
        });
        let passthrough = (1..=SERVO_OUTPUTS).filter(|_| !px4).filter_map(|output| {
            let param = format!("SERVO{output}_FUNCTION");
            parameter(&param)
                .map(|function| function - ARDUPILOT_RCIN1 + 1.0)
                .filter(|rc| (1.0..=f64::from(RC_CHANNELS)).contains(rc))
                .map(|rc| (rc as usize, Claim::Passthrough { param }))
        });
        let claims: Vec<(usize, Claim)> = sticks
            .chain(modes)
            .chain(options)
            .chain(switches)
            .chain(passthrough)
            .collect();
        DroneFacts {
            claims: claims.into_iter().rev().collect(),
        }
    }
}

pub fn bind(layout: Layout, facts: &DroneFacts) -> (Layout, Vec<Conflict>) {
    let conflicts: Vec<Conflict> = facts
        .claims
        .iter()
        .filter_map(|(channel, claim)| {
            let names = layout.named_on(*channel);
            (!names.is_empty()).then(|| Conflict {
                channel: *channel,
                names,
                claim: claim.clone(),
            })
        })
        .collect();
    let bound = conflicts
        .iter()
        .fold(layout, |layout, conflict| layout.without(conflict.channel));
    (bound, conflicts)
}

pub fn notice(conflict: &Conflict, label: impl Fn(&str, u16) -> Option<String>) -> String {
    let channel = conflict.channel;
    let switch = conflict.names.join(" and ");
    match &conflict.claim {
        Claim::FlightModes => crate::radiocamera::clash_notice(channel, &conflict.names),
        Claim::Stick => format!(
            "The drone flies on CH{channel}, so the radio's {switch} switch is off. Move the switch to another channel."
        ),
        Claim::Option { param, value } => format!(
            "The drone uses CH{channel} for {} ({param} {value}), so the radio's {switch} switch is off. Clear {param} or move the switch to another channel.",
            label(param, *value).unwrap_or_else(|| format!("option {value}"))
        ),
        Claim::Switch { param } => format!(
            "The drone reads {param} on CH{channel}, so the radio's {switch} switch is off. Move {param} or the switch to another channel."
        ),
        Claim::Passthrough { param } => format!(
            "The drone passes CH{channel} straight to an output ({param}), so the radio's {switch} switch is off. Move that output or the switch to another channel."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radiocamera::R2D2;

    const COPTER: u8 = 2;
    const PLANE: u8 = 1;
    const HEXA: u8 = 13;

    fn facts(vehicle_type: u8, file: &str) -> DroneFacts {
        let params: BTreeMap<String, f64> = crate::paramfile::parse(file)
            .into_iter()
            .filter_map(|p| p.value.parse().ok().map(|value| (p.name, value)))
            .collect();
        DroneFacts::read(vehicle_type, false, |name| params.get(name).copied())
    }

    fn conflicts(facts: &DroneFacts) -> Vec<(usize, Vec<&'static str>)> {
        bind(R2D2, facts)
            .1
            .into_iter()
            .map(|conflict| (conflict.channel, conflict.names))
            .collect()
    }

    #[test]
    fn the_radio_layout_gives_way_to_every_channel_the_drone_reads_on_each_drone_we_flew() {
        let vampire = facts(HEXA, include_str!("../tests/fixtures/drones/vampire.param"));
        assert_eq!(vampire.claims.get(&10), Some(&Claim::FlightModes));
        assert!(
            conflicts(&vampire).is_empty(),
            "the Vampire reads flight modes on CH10 and nothing the R2D2 layout uses"
        );

        let guta = facts(
            PLANE,
            include_str!("../tests/fixtures/drones/guta-test.param"),
        );
        assert_eq!(
            guta.claims.get(&5),
            Some(&Claim::Option {
                param: "RC5_OPTION".into(),
                value: 153
            })
        );
        assert_eq!(
            conflicts(&guta),
            vec![(5, vec!["bay"])],
            "guta-test arms and disarms on CH5, where the R2D2 layout reads the 6POS"
        );
        assert_eq!(bind(R2D2, &guta).0.bay, 0);

        let andrey = facts(
            COPTER,
            include_str!("../tests/fixtures/drones/andrey-carrier.param"),
        );
        assert_eq!(
            conflicts(&andrey),
            vec![(5, vec!["bay"]), (11, vec!["drop"]), (12, vec!["arm"])],
            "Andrey's carrier has RC options on the 6POS, drop and BOMARM channels"
        );
        assert_eq!(
            andrey.claims.get(&7),
            Some(&Claim::Passthrough {
                param: "SERVO11_FUNCTION".into()
            }),
            "his S3 pad passes CH7 straight through"
        );
    }

    #[test]
    fn a_channel_the_drone_flies_on_or_reads_a_mode_from_is_named_first() {
        let params: BTreeMap<&str, f64> = [
            ("FLTMODE_CH", 5.0),
            ("RC5_OPTION", 153.0),
            ("RC1_OPTION", 4.0),
        ]
        .into();
        let facts = DroneFacts::read(COPTER, false, |name| params.get(name).copied());
        assert_eq!(facts.claims.get(&5), Some(&Claim::FlightModes));
        assert_eq!(facts.claims.get(&1), Some(&Claim::Stick));
    }

    #[test]
    fn px4_switch_maps_claim_their_channels() {
        let params: BTreeMap<&str, f64> = [("RC_MAP_ARM_SW", 12.0), ("RC5_OPTION", 153.0)].into();
        let facts = DroneFacts::read(COPTER, true, |name| params.get(name).copied());
        assert_eq!(
            facts.claims.get(&12),
            Some(&Claim::Switch {
                param: "RC_MAP_ARM_SW".into()
            })
        );
        assert_eq!(facts.claims.get(&5), None, "RCn_OPTION is ArduPilot's");
    }

    #[test]
    fn each_kind_of_claim_says_what_the_drone_does_with_the_channel() {
        let conflict = |claim| Conflict {
            channel: 5,
            names: vec!["bay"],
            claim,
        };
        let label = |param: &str, value: u16| {
            (param == "RC5_OPTION" && value == 153).then(|| "ArmDisarm".to_string())
        };
        assert_eq!(
            notice(
                &conflict(Claim::Option {
                    param: "RC5_OPTION".into(),
                    value: 153
                }),
                label
            ),
            "The drone uses CH5 for ArmDisarm (RC5_OPTION 153), so the radio's bay switch is off. Clear RC5_OPTION or move the switch to another channel."
        );
        assert_eq!(
            notice(&conflict(Claim::FlightModes), label),
            "The drone reads flight modes on CH5, so the radio's bay switch is off. Move flight modes or the switch to another channel."
        );
    }
}
