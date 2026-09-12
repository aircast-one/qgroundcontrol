package one.aircast.android.ui

internal const val SENSORS = "Sensors"
internal const val RADIO = "Radio"
internal const val REMOTE_SUPPORT = "Remote Support"
internal const val MOTORS = "Motors"
internal const val FLIGHT_MODES_PAGE = "Flight Modes"

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
            page.parameterSections)
