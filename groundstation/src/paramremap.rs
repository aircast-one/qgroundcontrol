const HIGHEST_MINOR: &[(u8, u8)] = &[(4, 7)];
const NO_REMAP: &str = "noremap.";

const REMAPS: &[(&str, u8, u8, &str, &str)] = &[
    ("Copter", 4, 0, "TUNE_MIN", "TUNE_LOW"),
    ("Copter", 4, 0, "TUNE_MAX", "TUNE_HIGH"),
    ("Copter", 4, 7, "PSC_NE_VEL_P", "PSC_VELXY_P"),
    ("Copter", 4, 7, "PSC_NE_VEL_I", "PSC_VELXY_I"),
    ("Copter", 4, 7, "PSC_NE_VEL_D", "PSC_VELXY_D"),
    ("Copter", 4, 7, "PSC_NE_VEL_IMAX", "PSC_VELXY_IMAX"),
    ("Copter", 4, 7, "PSC_NE_VEL_FLTE", "PSC_VELXY_FLTE"),
    ("Copter", 4, 7, "PSC_NE_VEL_FLTD", "PSC_VELXY_FLTD"),
    ("Copter", 4, 7, "PSC_NE_VEL_FF", "PSC_VELXY_FF"),
    ("Copter", 4, 7, "PSC_D_VEL_P", "PSC_VELZ_P"),
    ("Copter", 4, 7, "PSC_D_VEL_I", "PSC_VELZ_I"),
    ("Copter", 4, 7, "PSC_D_VEL_D", "PSC_VELZ_D"),
    ("Copter", 4, 7, "PSC_D_VEL_IMAX", "PSC_VELZ_IMAX"),
    ("Copter", 4, 7, "PSC_D_VEL_FLTE", "PSC_VELZ_FLTE"),
    ("Copter", 4, 7, "PSC_D_VEL_FF", "PSC_VELZ_FF"),
    ("Copter", 4, 7, "PSC_D_ACC_P", "PSC_ACCZ_P"),
    ("Copter", 4, 7, "PSC_D_ACC_I", "PSC_ACCZ_I"),
    ("Copter", 4, 7, "PSC_D_ACC_D", "PSC_ACCZ_D"),
    ("Copter", 4, 7, "PSC_D_ACC_IMAX", "PSC_ACCZ_IMAX"),
    ("Copter", 4, 7, "PSC_D_ACC_FLTD", "PSC_ACCZ_FLTD"),
    ("Copter", 4, 7, "PSC_D_ACC_FLTE", "PSC_ACCZ_FLTE"),
    ("Copter", 4, 7, "PSC_D_ACC_FLTT", "PSC_ACCZ_FLTT"),
    ("Copter", 4, 7, "PSC_D_ACC_FF", "PSC_ACCZ_FF"),
    ("Copter", 4, 7, "PSC_D_ACC_SMAX", "PSC_ACCZ_SMAX"),
    ("Copter", 4, 7, "PSC_NE_POS_P", "PSC_POSXY_P"),
    ("Copter", 4, 7, "PSC_D_POS_P", "PSC_POSZ_P"),
    ("Copter", 4, 7, "WP_ACC", "WPNAV_ACCEL"),
    ("Copter", 4, 7, "WP_ACC_CNR", "WPNAV_ACCEL_C"),
    ("Copter", 4, 7, "WP_ACC_Z", "WPNAV_ACCEL_Z"),
    ("Copter", 4, 7, "WP_RADIUS_M", "WPNAV_RADIUS"),
    ("Copter", 4, 7, "WP_SPD", "WPNAV_SPEED"),
    ("Copter", 4, 7, "WP_SPD_DN", "WPNAV_SPEED_DN"),
    ("Copter", 4, 7, "WP_SPD_UP", "WPNAV_SPEED_UP"),
    ("Copter", 4, 7, "RTL_ALT_M", "RTL_ALT"),
    ("Copter", 4, 7, "RTL_SPEED_MS", "RTL_SPEED"),
    ("Copter", 4, 7, "RTL_ALT_FINAL_M", "RTL_ALT_FINAL"),
    ("Copter", 4, 7, "RTL_CLIMB_MIN_M", "RTL_CLIMB_MIN"),
    ("Copter", 4, 7, "LAND_SPD_MS", "LAND_SPEED"),
    ("Copter", 4, 7, "LAND_SPD_HIGH_MS", "LAND_SPEED_HIGH"),
    ("Copter", 4, 7, "LAND_ALT_LOW_M", "LAND_ALT_LOW"),
    ("Copter", 4, 7, "LOIT_SPEED_MS", "LOIT_SPEED"),
    ("Copter", 4, 7, "LOIT_ACC_MAX_M", "LOIT_ACC_MAX"),
    ("Copter", 4, 7, "LOIT_BRK_ACC_M", "LOIT_BRK_ACCEL"),
    ("Copter", 4, 7, "LOIT_BRK_JRK_M", "LOIT_BRK_JERK"),
    ("Copter", 4, 7, "PILOT_ACC_Z", "PILOT_ACCEL_Z"),
    ("Copter", 4, 7, "PILOT_SPD_UP", "PILOT_SPEED_UP"),
    ("Copter", 4, 7, "PILOT_SPD_DN", "PILOT_SPEED_DN"),
    ("Copter", 4, 7, "PILOT_TKO_ALT_M", "PILOT_TKOFF_ALT"),
    ("Copter", 4, 7, "ATC_ANGLE_MAX", "ANGLE_MAX"),
    ("Copter", 4, 7, "ATC_ACC_R_MAX", "ATC_ACCEL_R_MAX"),
    ("Copter", 4, 7, "ATC_ACC_P_MAX", "ATC_ACCEL_P_MAX"),
    ("Copter", 4, 7, "ATC_ACC_Y_MAX", "ATC_ACCEL_Y_MAX"),
    ("Copter", 4, 7, "ATC_RATE_WPY_MAX", "ATC_SLEW_YAW"),
    ("Copter", 4, 7, "CIRCLE_RADIUS_M", "CIRCLE_RADIUS"),
    ("Copter", 4, 7, "PHLD_BRK_ANGLE", "PHLD_BRAKE_ANGLE"),
    ("Copter", 4, 7, "PHLD_BRK_RATE", "PHLD_BRAKE_RATE"),
    ("Copter", 4, 7, "EK3_FLOW_MAX", "EK3_MAX_FLOW"),
    ("Plane", 4, 5, "AIRSPEED_MIN", "ARSPD_FBW_MIN"),
    ("Plane", 4, 5, "AIRSPEED_MAX", "ARSPD_FBW_MAX"),
    ("Plane", 4, 5, "RTL_ALTITUDE", "ALT_HOLD_RTL"),
    ("Plane", 4, 7, "Q_A_ANGLE_MAX", "Q_ANGLE_MAX"),
    ("Plane", 4, 7, "Q_A_ACC_R_MAX", "Q_A_ACCEL_R_MAX"),
    ("Plane", 4, 7, "Q_A_ACC_P_MAX", "Q_A_ACCEL_P_MAX"),
    ("Plane", 4, 7, "Q_A_ACC_Y_MAX", "Q_A_ACCEL_Y_MAX"),
    ("Plane", 4, 7, "Q_A_RATE_WPY_MAX", "Q_A_SLEW_YAW"),
    ("Plane", 4, 7, "Q_LOIT_SPEED_MS", "Q_LOIT_SPEED"),
    ("Plane", 4, 7, "Q_LOIT_ACC_MAX_M", "Q_LOIT_ACC_MAX"),
    ("Plane", 4, 7, "Q_LOIT_BRK_ACC_M", "Q_LOIT_BRK_ACCEL"),
    ("Plane", 4, 7, "Q_LOIT_BRK_JRK_M", "Q_LOIT_BRK_JERK"),
    ("Plane", 4, 7, "Q_PILOT_SPD_UP", "Q_PILOT_SPEED_UP"),
    ("Plane", 4, 7, "Q_PILOT_SPD_DN", "Q_PILOT_SPEED_DN"),
    ("Plane", 4, 7, "Q_PILOT_TKO_ALT_M", "Q_PILOT_TKOFF_ALT"),
    ("Plane", 4, 7, "Q_P_NE_VEL_P", "Q_P_VELXY_P"),
    ("Plane", 4, 7, "Q_P_NE_VEL_I", "Q_P_VELXY_I"),
    ("Plane", 4, 7, "Q_P_NE_VEL_D", "Q_P_VELXY_D"),
    ("Plane", 4, 7, "Q_P_NE_VEL_IMAX", "Q_P_VELXY_IMAX"),
    ("Plane", 4, 7, "Q_P_NE_VEL_FLTE", "Q_P_VELXY_FLTE"),
    ("Plane", 4, 7, "Q_P_NE_VEL_FLTD", "Q_P_VELXY_FLTD"),
    ("Plane", 4, 7, "Q_P_NE_VEL_FF", "Q_P_VELXY_FF"),
    ("Plane", 4, 7, "Q_P_D_VEL_P", "Q_P_VELZ_P"),
    ("Plane", 4, 7, "Q_P_D_VEL_I", "Q_P_VELZ_I"),
    ("Plane", 4, 7, "Q_P_D_VEL_D", "Q_P_VELZ_D"),
    ("Plane", 4, 7, "Q_P_D_VEL_IMAX", "Q_P_VELZ_IMAX"),
    ("Plane", 4, 7, "Q_P_D_VEL_FLTE", "Q_P_VELZ_FLTE"),
    ("Plane", 4, 7, "Q_P_D_VEL_FF", "Q_P_VELZ_FF"),
    ("Plane", 4, 7, "Q_P_D_ACC_P", "Q_P_ACCZ_P"),
    ("Plane", 4, 7, "Q_P_D_ACC_I", "Q_P_ACCZ_I"),
    ("Plane", 4, 7, "Q_P_D_ACC_D", "Q_P_ACCZ_D"),
    ("Plane", 4, 7, "Q_P_D_ACC_IMAX", "Q_P_ACCZ_IMAX"),
    ("Plane", 4, 7, "Q_P_D_ACC_FLTD", "Q_P_ACCZ_FLTD"),
    ("Plane", 4, 7, "Q_P_D_ACC_FLTE", "Q_P_ACCZ_FLTE"),
    ("Plane", 4, 7, "Q_P_D_ACC_FLTT", "Q_P_ACCZ_FLTT"),
    ("Plane", 4, 7, "Q_P_D_ACC_FF", "Q_P_ACCZ_FF"),
    ("Plane", 4, 7, "Q_P_D_ACC_SMAX", "Q_P_ACCZ_SMAX"),
    ("Plane", 4, 7, "Q_WP_ACC", "Q_WP_ACCEL"),
    ("Plane", 4, 7, "Q_WP_ACC_CNR", "Q_WP_ACCEL_C"),
    ("Plane", 4, 7, "Q_WP_ACC_Z", "Q_WP_ACCEL_Z"),
    ("Plane", 4, 7, "Q_WP_RADIUS_M", "Q_WP_RADIUS"),
    ("Plane", 4, 7, "Q_WP_SPD", "Q_WP_SPEED"),
    ("Plane", 4, 7, "Q_WP_SPD_DN", "Q_WP_SPEED_DN"),
    ("Plane", 4, 7, "Q_WP_SPD_UP", "Q_WP_SPEED_UP"),
    ("Plane", 4, 7, "EK3_FLOW_MAX", "EK3_MAX_FLOW"),
    ("Plane", 4, 7, "ARMING_SKIPCHK", "ARMING_CHECK"),
    ("Rover", 4, 7, "EK3_FLOW_MAX", "EK3_MAX_FLOW"),
    ("Rover", 4, 7, "ARMING_SKIPCHK", "ARMING_CHECK"),
    ("Sub", 4, 7, "PSC_NE_VEL_P", "PSC_VELXY_P"),
    ("Sub", 4, 7, "PSC_NE_VEL_I", "PSC_VELXY_I"),
    ("Sub", 4, 7, "PSC_NE_VEL_D", "PSC_VELXY_D"),
    ("Sub", 4, 7, "PSC_NE_VEL_IMAX", "PSC_VELXY_IMAX"),
    ("Sub", 4, 7, "PSC_NE_VEL_FLTE", "PSC_VELXY_FLTE"),
    ("Sub", 4, 7, "PSC_NE_VEL_FLTD", "PSC_VELXY_FLTD"),
    ("Sub", 4, 7, "PSC_NE_VEL_FF", "PSC_VELXY_FF"),
    ("Sub", 4, 7, "PSC_D_VEL_P", "PSC_VELZ_P"),
    ("Sub", 4, 7, "PSC_D_VEL_I", "PSC_VELZ_I"),
    ("Sub", 4, 7, "PSC_D_VEL_D", "PSC_VELZ_D"),
    ("Sub", 4, 7, "PSC_D_VEL_IMAX", "PSC_VELZ_IMAX"),
    ("Sub", 4, 7, "PSC_D_VEL_FLTE", "PSC_VELZ_FLTE"),
    ("Sub", 4, 7, "PSC_D_VEL_FF", "PSC_VELZ_FF"),
    ("Sub", 4, 7, "PSC_D_ACC_P", "PSC_ACCZ_P"),
    ("Sub", 4, 7, "PSC_D_ACC_I", "PSC_ACCZ_I"),
    ("Sub", 4, 7, "PSC_D_ACC_D", "PSC_ACCZ_D"),
    ("Sub", 4, 7, "PSC_D_ACC_IMAX", "PSC_ACCZ_IMAX"),
    ("Sub", 4, 7, "PSC_D_ACC_FLTD", "PSC_ACCZ_FLTD"),
    ("Sub", 4, 7, "PSC_D_ACC_FLTE", "PSC_ACCZ_FLTE"),
    ("Sub", 4, 7, "PSC_D_ACC_FLTT", "PSC_ACCZ_FLTT"),
    ("Sub", 4, 7, "PSC_D_ACC_FF", "PSC_ACCZ_FF"),
    ("Sub", 4, 7, "PSC_D_ACC_SMAX", "PSC_ACCZ_SMAX"),
    ("Sub", 4, 7, "PSC_NE_POS_P", "PSC_POSXY_P"),
    ("Sub", 4, 7, "PSC_D_POS_P", "PSC_POSZ_P"),
    ("Sub", 4, 7, "WP_ACC", "WPNAV_ACCEL"),
    ("Sub", 4, 7, "WP_ACC_CNR", "WPNAV_ACCEL_C"),
    ("Sub", 4, 7, "WP_ACC_Z", "WPNAV_ACCEL_Z"),
    ("Sub", 4, 7, "WP_RADIUS_M", "WPNAV_RADIUS"),
    ("Sub", 4, 7, "WP_SPD", "WPNAV_SPEED"),
    ("Sub", 4, 7, "WP_SPD_DN", "WPNAV_SPEED_DN"),
    ("Sub", 4, 7, "WP_SPD_UP", "WPNAV_SPEED_UP"),
    ("Sub", 4, 7, "ATC_ACC_R_MAX", "ATC_ACCEL_R_MAX"),
    ("Sub", 4, 7, "ATC_ACC_P_MAX", "ATC_ACCEL_P_MAX"),
    ("Sub", 4, 7, "ATC_ACC_Y_MAX", "ATC_ACCEL_Y_MAX"),
    ("Sub", 4, 7, "ATC_RATE_WPY_MAX", "ATC_SLEW_YAW"),
    ("Sub", 4, 7, "CIRCLE_RADIUS_M", "CIRCLE_RADIUS"),
    ("Sub", 4, 7, "EK3_FLOW_MAX", "EK3_MAX_FLOW"),
    ("Sub", 4, 7, "ARMING_SKIPCHK", "ARMING_CHECK"),
];

pub fn versioned(family: Option<&str>, version: Option<(u8, u8)>, name: &str) -> String {
    if let Some(literal) = name.strip_prefix(NO_REMAP) {
        return literal.to_string();
    }
    let (Some(family), Some((major, minor))) = (family, version) else { return name.to_string() };
    let Some(highest) = HIGHEST_MINOR.iter().find(|(m, _)| *m == major).map(|(_, highest)| *highest) else { return name.to_string() };
    (minor.saturating_add(1)..=highest).rev().fold(name.to_string(), |current, step| {
        REMAPS
            .iter()
            .find(|(f, ma, mi, from, _)| *f == family && *ma == major && *mi == step && *from == current)
            .map_or(current, |(.., to)| (*to).to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_parameter_name_reads_the_name_older_firmware_uses() {
        assert_eq!(versioned(Some("Copter"), Some((4, 5)), "PSC_D_ACC_P"), "PSC_ACCZ_P", "4.7 renamed it, so a 4.5 vehicle answers to the old name");
        assert_eq!(versioned(Some("Copter"), Some((4, 7)), "PSC_D_ACC_P"), "PSC_D_ACC_P", "the vehicle already uses the new name");
        assert_eq!(versioned(Some("Copter"), Some((3, 6)), "PSC_D_ACC_P"), "PSC_D_ACC_P", "no table for another major version");
        assert_eq!(versioned(Some("Copter"), None, "PSC_D_ACC_P"), "PSC_D_ACC_P", "an unknown version is left alone, as _remapParamNameToVersion does");
        assert_eq!(versioned(None, Some((4, 5)), "PSC_D_ACC_P"), "PSC_D_ACC_P", "PX4 has no remap table");
        assert_eq!(versioned(Some("Copter"), Some((4, 5)), "noremap.PSC_D_ACC_P"), "PSC_D_ACC_P");
        assert_eq!(versioned(Some("Copter"), Some((3, 9)), "TUNE_MIN"), "TUNE_MIN");
    }

    #[test]
    fn every_minor_above_the_vehicle_applies() {
        assert_eq!(versioned(Some("Plane"), Some((4, 4)), "AIRSPEED_MIN"), "ARSPD_FBW_MIN", "a 4.5 rename reaches a 4.4 plane");
        assert_eq!(versioned(Some("Plane"), Some((4, 5)), "AIRSPEED_MIN"), "AIRSPEED_MIN", "a 4.5 plane already has it");
        assert_eq!(versioned(Some("Copter"), Some((4, 0)), "TUNE_MIN"), "TUNE_MIN", "the vehicle's own minor is never walked, so the 4.0 table cannot apply to any 4.x");
    }
}
