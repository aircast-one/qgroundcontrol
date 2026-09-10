package one.aircast.android.ui

import org.json.JSONObject

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

internal fun guidedAltitudePath(target: Double): String =
    "$GUIDED_ALTITUDE(${String.format(java.util.Locale.US, "%.2f", target)})"

private fun JSONObject.numberOrNull(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun guidedAltitude(view: JSONObject?): GuidedAltitude? {
    if (view == null || !view.optBoolean("available")) return null
    return GuidedAltitude(
        available = true,
        label = view.optString("label"),
        unit = view.optString("unit"),
        current = view.numberOrNull("current"),
        minimum = view.numberOrNull("minimum"),
        maximum = view.numberOrNull("maximum"),
        sentence = if (view.isNull("sentence")) "" else view.optString("sentence"),
        deltaMeters = view.optDouble("deltaMeters", 0.0),
        sends = view.optBoolean("sends"),
    )
}

internal fun altitudeRangeUsable(reading: GuidedAltitude?): Boolean =
    reading?.minimum != null && reading.maximum != null && reading.maximum > reading.minimum
