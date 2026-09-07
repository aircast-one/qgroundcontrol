package one.aircast.android.ui

private val SAFETY_APM = listOf(
    ParameterSection(
        "Throttle and link failsafe",
        listOf("FS_THR_ENABLE", "FS_THR_VALUE", "FS_GCS_ENABLE", "FS_OPTIONS"),
    ),
    ParameterSection(
        "Battery failsafe",
        listOf(
            "BATT_MONITOR", "BATT_FS_LOW_ACT", "BATT_FS_CRT_ACT",
            "BATT_LOW_VOLT", "BATT_LOW_MAH", "BATT_CRT_VOLT", "BATT_CRT_MAH",
        ),
    ),
    ParameterSection(
        "Second battery",
        listOf(
            "BATT2_MONITOR", "BATT2_FS_LOW_ACT", "BATT2_FS_CRT_ACT",
            "BATT2_LOW_VOLT", "BATT2_LOW_MAH", "BATT2_CRT_VOLT", "BATT2_CRT_MAH",
        ),
    ),
    ParameterSection(
        "Geofence",
        listOf(
            "FENCE_ENABLE", "FENCE_TYPE", "FENCE_ACTION",
            "FENCE_ALT_MAX", "FENCE_RADIUS", "FENCE_MARGIN",
        ),
    ),
    ParameterSection(
        "Return and land",
        listOf("RTL_ALT", "RTL_ALT_FINAL", "RTL_LOIT_TIME", "LAND_SPEED"),
    ),
    ParameterSection("Arming", listOf("ARMING_CHECK")),
)

private val SAFETY_PX4 = listOf(
    ParameterSection(
        "Link failsafe",
        listOf("NAV_RCL_ACT", "COM_RC_LOSS_T", "NAV_DLL_ACT", "COM_DL_LOSS_T"),
    ),
    ParameterSection(
        "Battery failsafe",
        listOf("COM_LOW_BAT_ACT", "BAT_LOW_THR", "BAT_CRIT_THR", "BAT_EMERGEN_THR"),
    ),
    ParameterSection(
        "Geofence",
        listOf("GF_ACTION", "GF_MAX_HOR_DIST", "GF_MAX_VER_DIST"),
    ),
    ParameterSection(
        "Return and land",
        listOf(
            "RTL_RETURN_ALT", "RTL_DESCEND_ALT", "RTL_LAND_DELAY",
            "MPC_LAND_SPEED", "COM_DISARM_LAND",
        ),
    ),
)

private val LIGHTS_APM = listOf(
    ParameterSection(
        "Light channels",
        (5..16).map { "SERVO${it}_FUNCTION" },
    ),
    ParameterSection(
        "Brightness steps",
        listOf("JS_LIGHTS_STEPS", "JS_LIGHTS_STEP", "BRD_PWM_COUNT"),
    ),
)

private val CAMERA_APM = listOf(
    ParameterSection("Gimbal", listOf("MNT_TYPE", "MNT1_TYPE", "MNT_DEFLT_MODE")),
    ParameterSection(
        "Angle limits",
        listOf(
            "MNT_ANGMIN_PAN", "MNT_ANGMAX_PAN",
            "MNT_ANGMIN_ROL", "MNT_ANGMAX_ROL",
            "MNT_ANGMIN_TIL", "MNT_ANGMAX_TIL",
        ),
    ),
    ParameterSection("Neutral angles", listOf("MNT_NEUTRAL_X", "MNT_NEUTRAL_Y", "MNT_NEUTRAL_Z")),
    ParameterSection("Retract angles", listOf("MNT_RETRACT_X", "MNT_RETRACT_Y", "MNT_RETRACT_Z")),
    ParameterSection("Stabilisation", listOf("MNT_STAB_PAN", "MNT_STAB_ROLL", "MNT_STAB_TILT")),
    ParameterSection("RC input", listOf("MNT_RC_IN_PAN", "MNT_RC_IN_ROLL", "MNT_RC_IN_TILT")),
)

private val FLIGHT_BEHAVIOR_PX4 = listOf(
    ParameterSection(
        "Responsiveness",
        listOf("SYS_VEHICLE_RESP", "MPC_XY_VEL_ALL", "MPC_Z_VEL_ALL"),
    ),
)

private val FLIGHT_MODES_APM = listOf(
    ParameterSection("Mode switch channel", listOf("FLTMODE_CH")),
    ParameterSection(
        "Mode slots",
        (1..6).map { "FLTMODE$it" },
        "This firmware reports every slot after the first as read-only, so they are " +
            "shown but cannot be changed here.",
    ),
    ParameterSection("Options", listOf("SIMPLE", "SUPER_SIMPLE", "INITIAL_MODE")),
)

private val FLIGHT_MODES_PX4 = listOf(
    ParameterSection("Mode switch channel", listOf("RC_MAP_FLTMODE")),
    ParameterSection(
        "Mode slots",
        (1..6).map { "COM_FLTMODE$it" },
        "Slots the firmware reports as read-only are shown but cannot be changed here.",
    ),
    ParameterSection(
        "Single function switches",
        listOf(
            "RC_MAP_RETURN_SW", "RC_MAP_KILL_SW", "RC_MAP_ARM_SW",
            "RC_MAP_LOITER_SW", "RC_MAP_OFFB_SW", "RC_MAP_GEAR_SW", "RC_MAP_TRANS_SW",
        ),
    ),
)

private val POWER_APM = listOf(
    ParameterSection("Battery 1", listOf("BATT_MONITOR", "BATT_CAPACITY")),
    ParameterSection(
        "Battery 1 sensor calibration",
        listOf(
            "BATT_VOLT_PIN", "BATT_CURR_PIN", "BATT_VOLT_MULT",
            "BATT_AMP_PERVLT", "BATT_AMP_OFFSET",
        ),
        "Measured with a multimeter during calibration. The calibration wizard is " +
            "not here yet, so these are the raw values it would write.",
    ),
    ParameterSection("Battery 2", listOf("BATT2_MONITOR", "BATT2_CAPACITY")),
    ParameterSection(
        "Battery 2 sensor calibration",
        listOf(
            "BATT2_VOLT_PIN", "BATT2_CURR_PIN", "BATT2_VOLT_MULT",
            "BATT2_AMP_PERVLT", "BATT2_AMP_OFFSET",
        ),
    ),
)

private val POWER_PX4 = listOf(
    ParameterSection(
        "Battery",
        listOf(
            "BAT_N_CELLS", "BAT_V_CHARGED", "BAT_V_EMPTY", "BAT_CAPACITY",
            "BAT1_N_CELLS", "BAT1_V_CHARGED", "BAT1_V_EMPTY", "BAT1_CAPACITY",
        ),
    ),
    ParameterSection(
        "Sensor calibration",
        listOf("BAT_V_DIV", "BAT_A_PER_V", "BAT1_V_DIV", "BAT1_A_PER_V"),
        "Measured during calibration. The calibration wizard is not here yet, so " +
            "these are the raw values it would write.",
    ),
)

private val TUNING_APM = listOf(
    ParameterSection(
        "Feel",
        listOf("ATC_INPUT_TC"),
        "Higher is softer on the sticks, lower is sharper.",
    ),
    ParameterSection(
        "Attitude gains",
        listOf("ATC_ANG_RLL_P", "ATC_ANG_PIT_P", "ATC_ANG_YAW_P"),
    ),
    ParameterSection(
        "Rate gains",
        listOf(
            "ATC_RAT_RLL_P", "ATC_RAT_RLL_I", "ATC_RAT_RLL_D",
            "ATC_RAT_PIT_P", "ATC_RAT_PIT_I", "ATC_RAT_PIT_D",
            "ATC_RAT_YAW_P", "ATC_RAT_YAW_I",
        ),
        "Change these in small steps and test fly between changes. Autotune is " +
            "not carried over yet, so nothing here will retune the aircraft for you.",
    ),
    ParameterSection("Vertical position", listOf("PSC_ACCZ_P", "PSC_ACCZ_I")),
)

private val FRAME_APM = listOf(
    ParameterSection(
        "Airframe",
        listOf("FRAME_CLASS", "FRAME_TYPE"),
        "Changing the class or type changes how the motors are numbered and which " +
            "way they spin. Re-check motor order and direction afterwards, before flying.",
    ),
)

internal fun setupSectionsFor(componentName: String, isPx4: Boolean): List<ParameterSection>? =
    when (componentName) {
        "Safety" -> if (isPx4) SAFETY_PX4 else SAFETY_APM
        "Power" -> if (isPx4) POWER_PX4 else POWER_APM
        "Tuning" -> if (isPx4) null else TUNING_APM
        "Frame" -> if (isPx4) null else FRAME_APM
        "Flight Modes" -> if (isPx4) FLIGHT_MODES_PX4 else FLIGHT_MODES_APM
        "Lights" -> if (isPx4) null else LIGHTS_APM
        "Camera" -> if (isPx4) null else CAMERA_APM
        "Flight Behavior" -> if (isPx4) FLIGHT_BEHAVIOR_PX4 else null
        else -> null
    }

internal const val REMOTE_SUPPORT = "Remote Support"
internal const val SENSORS = "Sensors"

internal fun hasNativeSetupPage(componentName: String, isPx4: Boolean) =
    componentName == REMOTE_SUPPORT ||
        (componentName == SENSORS && !isPx4) ||
        setupSectionsFor(componentName, isPx4) != null
