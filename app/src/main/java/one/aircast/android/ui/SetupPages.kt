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

internal fun setupSectionsFor(componentName: String, isPx4: Boolean): List<ParameterSection>? =
    when (componentName) {
        "Safety" -> if (isPx4) SAFETY_PX4 else SAFETY_APM
        else -> null
    }
