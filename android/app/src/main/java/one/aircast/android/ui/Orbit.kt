package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val ORBIT_VIEW = "view.orbit"

internal data class OrbitReading(
    val available: Boolean,
    val orbiting: Boolean?,
    val radiusText: String,
    val clockwise: Boolean?,
    val reason: String,
)

internal fun orbitReading(view: JSONObject?): OrbitReading? {
    if (view == null || view.optText("class") != "Orbit") return null
    return OrbitReading(
        available = view.optBoolean("available"),
        orbiting = if (view.isNull("orbiting")) null else view.optBoolean("orbiting"),
        radiusText = view.optText("radiusText"),
        clockwise = if (view.isNull("clockwise")) null else view.optBoolean("clockwise"),
        reason = view.optText("reason"),
    )
}

internal fun orbitLabel(reading: OrbitReading?): String? {
    if (reading == null || reading.orbiting != true) return null
    val turn = when (reading.clockwise) {
        true -> "clockwise"
        false -> "anticlockwise"
        null -> ""
    }
    return listOf("Orbiting", reading.radiusText, turn).filter { it.isNotBlank() }.joinToString(" ")
}
