package one.aircast.android.ui

internal const val SENSORS = "Sensors"
internal const val RADIO = "Radio"
internal const val REMOTE_SUPPORT = "Remote Support"
internal const val MOTORS = "Motors"
internal const val FLIGHT_MODES_PAGE = "Flight Modes"
internal const val PX4_TUNING_SCREEN = "px4Tuning"

internal val KNOWN_PAGES = mapOf(
    "sensors" to SENSORS,
    "radio" to RADIO,
    "flightModes" to FLIGHT_MODES_PAGE,
)

internal fun headPage(component: SetupComponent): String =
    component.known?.let { KNOWN_PAGES[it] } ?: component.name

internal fun headCanOpen(page: SetupPage?, name: String): Boolean =
    page != null &&
        (name == SENSORS || name == RADIO || name == REMOTE_SUPPORT || name == MOTORS ||
            page.parameterSections || page.screen == PX4_TUNING_SCREEN || page.screen == PX4_AIRFRAME_SCREEN || page.screen == ACTUATORS_SCREEN || page.screen == APM_SERVOS_SCREEN || page.screen == APM_FOLLOW_SCREEN || page.screen == SCRIPTING_SCREEN || page.screen == JOYSTICK_SCREEN || page.screen == ESP_BRIDGE_SCREEN || page.screen == APM_SUB_FRAME_SCREEN || page.screen == APM_SUB_MOTORS_SCREEN)
