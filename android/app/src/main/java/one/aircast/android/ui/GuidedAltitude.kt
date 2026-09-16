package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val GUIDED_ALTITUDE = "view.guidedAltitude"

internal data class GuidedAltitude(
    val available: Boolean,
    val label: String,
    val unit: String,
    val current: Double?,
    val minimum: Double?,
    val maximum: Double?,
    val sentence: String,
    val deltaMeters: Double,
    val sends: Boolean,
)

internal fun guidedAltitudePath(target: Double, pause: Boolean = false): String =
    "$GUIDED_ALTITUDE(${String.format(java.util.Locale.US, "%.2f", target)}${if (pause) ",pause" else ""})"

private fun JSONObject.numberOrNull(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun guidedAltitude(view: JSONObject?): GuidedAltitude? {
    if (view == null || !view.optBoolean("available")) return null
    return GuidedAltitude(
        available = true,
        label = view.optText("label"),
        unit = view.optText("unit"),
        current = view.numberOrNull("current"),
        minimum = view.numberOrNull("minimum"),
        maximum = view.numberOrNull("maximum"),
        sentence = if (view.isNull("sentence")) "" else view.optText("sentence"),
        deltaMeters = view.optDouble("deltaMeters", 0.0),
        sends = view.optBoolean("sends"),
    )
}

internal fun altitudeCommandArgs(reading: GuidedAltitude, pauses: Boolean): List<Any>? =
    reading.takeIf { it.sends }?.let { listOf(it.deltaMeters, pauses) }

internal fun altitudeRangeUsable(reading: GuidedAltitude?): Boolean =
    reading?.minimum != null && reading.maximum != null && reading.maximum > reading.minimum

internal fun rangeLabel(minimum: Double?, maximum: Double?, unit: String): String? {
    if (minimum == null || maximum == null || maximum <= minimum) return null
    val show = { value: Double ->
        val text = String.format(java.util.Locale.US, "%.1f", value)
        if (text == "-0.0") "0.0" else text
    }
    return listOf("${show(minimum)} to ${show(maximum)}", unit)
        .filter { it.isNotBlank() }
        .joinToString(" ")
}
