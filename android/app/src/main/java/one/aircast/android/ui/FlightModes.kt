package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val FLIGHT_MODES = "view.flightModes"

internal data class FlightModeOption(
    val name: String,
    val summary: String,
    val current: Boolean,
    val needsConfirm: Boolean,
)

internal data class FlightModesView(
    val canSet: Boolean,
    val current: String,
    val currentSummary: String,
    val everyday: List<FlightModeOption>,
    val folded: List<FlightModeOption>,
)

private fun options(view: JSONObject?, key: String): List<FlightModeOption> {
    val items = view?.optJSONArray(key) ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { mode ->
            FlightModeOption(
                name = mode.optText("name"),
                summary = mode.optText("summary"),
                current = mode.optBoolean("current"),
                needsConfirm = mode.optBoolean("needsConfirm"),
            )
        }
    }
}

internal fun flightModesView(view: JSONObject?): FlightModesView? {
    if (view == null || !view.optBoolean("available")) return null
    return FlightModesView(
        canSet = view.optBoolean("canSet"),
        current = view.optText("current"),
        currentSummary = view.optText("currentSummary"),
        everyday = options(view, "everyday"),
        folded = options(view, "folded"),
    )
}

internal fun modeHeading(modes: FlightModesView?): String? {
    val summary = modes?.currentSummary?.ifBlank { null } ?: return null
    val name = modes.current.ifBlank { null } ?: return null
    return "$name — $summary"
}
