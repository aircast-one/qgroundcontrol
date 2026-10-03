use std::io::Read;

use serde_json::Value;

pub const PX4_FIRMWARE_URL: &str = "http://px4-travis.s3.amazonaws.com/Firmware";
pub const ARDUPILOT_MANIFEST_URL: &str = "https://firmware.ardupilot.org/manifest.json.gz";

const PX4_BOARDS: &[(u32, &str)] = &[
    (9, "px4_fmu-v2_default"),
    (255, "px4_fmu-v3_default"),
    (11, "px4_fmu-v4_default"),
    (13, "px4_fmu-v4pro_default"),
    (20, "uvify_core_default"),
    (50, "px4_fmu-v5_default"),
    (51, "px4_fmu-v5x_default"),
    (52, "px4_fmu-v6_default"),
    (53, "px4_fmu-v6x_default"),
    (54, "px4_fmu-v6u_default"),
    (56, "px4_fmu-v6c_default"),
    (57, "ark_fmu-v6x_default"),
    (59, "ark_fpv_default"),
    (35, "px4_fmu-v6xrt_default"),
    (55, "sky-drones_smartap-airlink_default"),
    (88, "airmind_mindpx-v2_default"),
    (12, "bitcraze_crazyflie_default"),
    (14, "bitcraze_crazyflie21_default"),
    (42, "omnibus_f4sd_default"),
    (33, "mro_x21_default"),
    (65, "intel_aerofc-v1_default"),
    (123, "holybro_kakutef7_default"),
    (41775, "modalai_fc-v1_default"),
    (41776, "modalai_fc-v2_default"),
    (78, "holybro_pix32v5_default"),
    (79, "holybro_can-gps-v1_default"),
    (28, "nxp_fmuk66-v3_default"),
    (30, "nxp_fmuk66-e_default"),
    (31, "nxp_fmurt1062-v1_default"),
    (37, "nxp_mr-tropic_default"),
    (85, "freefly_can-rtk-gps_default"),
    (120, "cubepilot_cubeyellow_default"),
    (136, "mro_x21-777_default"),
    (139, "holybro_durandal-v1_default"),
    (140, "cubepilot_cubeorange_default"),
    (1063, "cubepilot_cubeorangeplus_default"),
    (141, "mro_ctrl-zero-f7_default"),
    (142, "mro_ctrl-zero-f7-oem_default"),
    (212, "thepeach_k1_default"),
    (213, "thepeach_r1_default"),
    (1009, "cuav_nora_default"),
    (1010, "cuav_x7pro_default"),
    (1017, "mro_pixracerpro_default"),
    (1022, "mro_ctrl-zero-classic_default"),
    (1023, "mro_ctrl-zero-h7_default"),
    (1024, "mro_ctrl-zero-h7-oem_default"),
    (1048, "holybro_kakuteh7_default"),
    (1053, "holybro_kakuteh7v2_default"),
    (1054, "holybro_kakuteh7mini_default"),
    (1058, "holybro_kakuteh7mini_default"),
    (1105, "holybro_kakuteh7-wing_default"),
    (1110, "jfb_jfb110_default"),
    (1200, "jfb_jfb200_default"),
    (1209, "gearup_airbrainh743_default"),
    (1198, "aedrox_aedroxh7_default"),
    (1123, "siyi_n7_default"),
    (1124, "3dr_ctrl-zero-h7-oem-revg_default"),
    (5600, "zeroone_x6_default"),
    (6110, "svehicle_e2_default"),
    (7000, "cuav_7-nano_default"),
    (7001, "cuav_fmu-v6x_default"),
    (7002, "cuav_x25-evo_default"),
    (7003, "cuav_x25-super_default"),
    (7004, "cuav_x25-mega_default"),
    (7120, "accton-godwit_ga1_default"),
];

pub fn px4_url(board_id: u32, build: Build) -> Option<String> {
    let name = PX4_BOARDS.iter().rev().find(|(id, _)| *id == board_id)?.1;
    let folder = match build {
        Build::Stable => "stable",
        Build::Beta => "beta",
        Build::Developer => "master",
    };
    Some(format!("{PX4_FIRMWARE_URL}/{folder}/{name}.px4"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    Stable,
    Beta,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vehicle {
    Copter,
    Heli,
    Plane,
    Rover,
    Sub,
}

impl Build {
    pub fn parse(word: &str) -> Option<Build> {
        Some(match word {
            "stable" => Build::Stable,
            "beta" => Build::Beta,
            "dev" | "developer" => Build::Developer,
            _ => return None,
        })
    }

    fn from_manifest(word: &str) -> Option<Build> {
        Some(match word {
            "OFFICIAL" => Build::Stable,
            "BETA" => Build::Beta,
            "DEV" => Build::Developer,
            _ => return None,
        })
    }
}

impl Vehicle {
    pub fn parse(word: &str) -> Option<Vehicle> {
        Some(match word {
            "copter" => Vehicle::Copter,
            "heli" => Vehicle::Heli,
            "plane" => Vehicle::Plane,
            "rover" => Vehicle::Rover,
            "sub" => Vehicle::Sub,
            _ => return None,
        })
    }

    fn from_manifest(word: &str) -> Option<Vehicle> {
        Some(match word {
            "Copter" => Vehicle::Copter,
            "HELICOPTER" => Vehicle::Heli,
            "FIXED_WING" => Vehicle::Plane,
            "GROUND_ROVER" => Vehicle::Rover,
            "SUBMARINE" => Vehicle::Sub,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub board_id: u32,
    pub build: Build,
    pub vehicle: Vehicle,
    pub url: String,
    pub chibios: bool,
    pub fmuv2: bool,
    pub bootloader_strings: Vec<String>,
    pub name: String,
}

fn entry(json: &Value) -> Option<Entry> {
    let text = |key: &str| json.get(key).and_then(Value::as_str).unwrap_or_default();
    let vehicle = Vehicle::from_manifest(text("mav-type"))?;
    let build = Build::from_manifest(text("mav-firmware-version-type"))?;
    let format = text("format");
    let platform = text("platform");
    if !matches!(format, "apj" | "px4") || (platform.contains("-heli") && vehicle != Vehicle::Heli) {
        return None;
    }
    let brand = text("brand_name");
    Some(Entry {
        board_id: json.get("board_id").and_then(Value::as_u64).and_then(|id| u32::try_from(id).ok()).unwrap_or_default(),
        build,
        vehicle,
        url: text("url").to_string(),
        chibios: format == "apj",
        fmuv2: platform.contains("fmuv2"),
        bootloader_strings: json.get("bootloader_str").and_then(Value::as_array).map(|listed| listed.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
        name: format!("{} - {}", if brand.is_empty() { platform } else { brand }, text("mav-firmware-version")),
    })
}

pub fn parse_manifest(text: &str) -> Result<Vec<Entry>, String> {
    let json: Value = serde_json::from_str(text).map_err(|e| format!("The ArduPilot manifest is not JSON: {e}"))?;
    Ok(json.get("firmware").and_then(Value::as_array).map(|listed| listed.iter().filter_map(entry).collect()).unwrap_or_default())
}

pub fn apm_url(entries: &[Entry], board_id: u32, build: Build, vehicle: Vehicle, chibios: bool, board_description: &str) -> Result<(Vec<(String, String)>, Option<usize>), String> {
    let raw_board = if board_id == crate::bootloader::BOARD_ID_PX4_FMU_V3 { crate::bootloader::BOARD_ID_PX4_FMU_V2 } else { board_id };
    let fmuv3 = board_id == crate::bootloader::BOARD_ID_PX4_FMU_V3;
    let matching: Vec<&Entry> = entries.iter().filter(|e| e.build == build && e.chibios == chibios && e.vehicle == vehicle && e.board_id == raw_board && !(e.fmuv2 && fmuv3)).collect();
    let best = board_description.ends_with("-BL").then(|| matching.iter().position(|e| e.bootloader_strings.iter().any(|s| s == board_description))).flatten().or((matching.len() == 1).then_some(0));
    match matching.is_empty() {
        true => Err("No Firmware Available".to_string()),
        false => Ok((matching.iter().map(|e| (e.name.clone(), e.url.clone())).collect(), best)),
    }
}

pub fn download(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url).call().map_err(|e| format!("Download of {url} failed: {e}"))?;
    let bytes = response.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|e| format!("Download of {url} failed: {e}"))?;
    match url.ends_with(".gz") {
        true => {
            let mut unpacked = Vec::new();
            flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut unpacked).map_err(|e| format!("{url} did not decompress: {e}"))?;
            Ok(unpacked)
        }
        false => Ok(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{"firmware":[
        {"mav-type":"Copter","mav-firmware-version-type":"OFFICIAL","format":"apj","platform":"fmuv5","board_id":50,"url":"https://f/copter-fmuv5.apj","mav-firmware-version":"4.6.2","bootloader_str":["fmuv5-BL","PX4 BL FMU v5.x"]},
        {"mav-type":"Copter","mav-firmware-version-type":"OFFICIAL","format":"apj","platform":"CUAVv5","brand_name":"CUAVv5","board_id":50,"url":"https://f/copter-cuav.apj","mav-firmware-version":"4.6.2","bootloader_str":["CUAVv5-BL","PX4 BL FMU v5.x"]},
        {"mav-type":"Copter","mav-firmware-version-type":"STABLE-4.6.2","format":"apj","platform":"fmuv5","board_id":50,"url":"https://f/pinned.apj","mav-firmware-version":"4.6.2"},
        {"mav-type":"Copter","mav-firmware-version-type":"OFFICIAL","format":"hex","platform":"fmuv5","board_id":50,"url":"https://f/x.hex"},
        {"mav-type":"FIXED_WING","mav-firmware-version-type":"OFFICIAL","format":"apj","platform":"fmuv5-heli","board_id":50,"url":"https://f/plane-heli.apj"},
        {"mav-type":"FIXED_WING","mav-firmware-version-type":"BETA","format":"apj","platform":"fmuv2","board_id":9,"url":"https://f/plane-fmuv2.apj","mav-firmware-version":"4.6.0"},
        {"mav-type":"FIXED_WING","mav-firmware-version-type":"BETA","format":"px4","platform":"px4-v2","board_id":9,"url":"https://f/plane-nuttx.px4","mav-firmware-version":"3.9.0"}
    ]}"#;

    #[test]
    fn the_manifest_keeps_what_qts_filter_keeps() {
        let entries = parse_manifest(MANIFEST).unwrap();
        assert_eq!(entries.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), ["https://f/copter-fmuv5.apj", "https://f/copter-cuav.apj", "https://f/plane-fmuv2.apj", "https://f/plane-nuttx.px4"], "pinned STABLE-x.y.z builds, hex images and a heli platform under another vehicle are dropped");
        assert_eq!(entries[1].name, "CUAVv5 - 4.6.2");
    }

    #[test]
    fn a_board_is_offered_every_fitting_build_with_the_bootloader_match_preselected() {
        let entries = parse_manifest(MANIFEST).unwrap();
        let offered = |board: u32, build: Build, vehicle: Vehicle, chibios: bool, description: &str| {
            apm_url(&entries, board, build, vehicle, chibios, description).map(|(listed, best)| (listed.into_iter().map(|(_, url)| url).collect::<Vec<_>>(), best))
        };
        let both = vec!["https://f/copter-fmuv5.apj".to_string(), "https://f/copter-cuav.apj".to_string()];
        assert_eq!(offered(50, Build::Stable, Vehicle::Copter, true, "CUAVv5-BL"), Ok((both.clone(), Some(1))), "apmFirmwareNamesBestIndex is the bootloader string match");
        assert_eq!(offered(50, Build::Stable, Vehicle::Copter, true, "PX4 BL FMU v5.x"), Ok((both, None)), "several builds and no bootloader match preselect nothing, as QGC's Choose board type placeholder");
        assert_eq!(offered(9, Build::Beta, Vehicle::Plane, true, ""), Ok((vec!["https://f/plane-fmuv2.apj".to_string()], Some(0))), "a single fitting build is preselected and still shown with its version");
        assert_eq!(offered(9, Build::Beta, Vehicle::Plane, false, ""), Ok((vec!["https://f/plane-nuttx.px4".to_string()], Some(0))), "the apmChibiOS setting at NuttX picks the .px4 build");
        assert_eq!(offered(crate::bootloader::BOARD_ID_PX4_FMU_V3, Build::Beta, Vehicle::Plane, true, ""), Err("No Firmware Available".to_string()), "an fmuv2 build is never offered to an fmuv3 board");
    }

    #[test]
    fn px4_files_are_named_from_the_bootloaders_board_id() {
        assert_eq!(px4_url(50, Build::Stable).as_deref(), Some("http://px4-travis.s3.amazonaws.com/Firmware/stable/px4_fmu-v5_default.px4"));
        assert_eq!(px4_url(140, Build::Beta).as_deref(), Some("http://px4-travis.s3.amazonaws.com/Firmware/beta/cubepilot_cubeorange_default.px4"));
        assert_eq!(px4_url(4242, Build::Stable), None);
        assert_eq!(px4_url(50, Build::Developer).as_deref(), Some("http://px4-travis.s3.amazonaws.com/Firmware/master/px4_fmu-v5_default.px4"), "FirmwareUpgradeController files the developer build under master");
    }
}
