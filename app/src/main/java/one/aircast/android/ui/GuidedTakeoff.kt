package one.aircast.android.ui

import java.util.Locale
import org.json.JSONObject

internal const val GUIDED_TAKEOFF = "view.guidedTakeoff"

internal data class GuidedTakeoff(
    val label: String,
    val unit: String,
    val initial: Double?,
    val minimum: Double?,
    val maximum: Double?,
    val sentence: String,
    val targetMeters: Double,
)

internal fun guidedTakeoffPath(target: Double): String =
    "$GUIDED_TAKEOFF(${String.format(Locale.US, "%.2f", target)})"

private fun JSONObject.numberOrNull(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun guidedTakeoff(view: JSONObject?): GuidedTakeoff? {
    if (view == null || !view.optBoolean("available")) return null
    return GuidedTakeoff(
        label = view.optString("label"),
        unit = view.optString("unit"),
        initial = view.numberOrNull("initial"),
        minimum = view.numberOrNull("minimum"),
        maximum = view.numberOrNull("maximum"),
        sentence = if (view.isNull("sentence")) "" else view.optString("sentence"),
        targetMeters = view.optDouble("targetMeters", 0.0),
    )
}

internal fun takeoffRangeUsable(takeoff: GuidedTakeoff?): Boolean =
    takeoff?.minimum != null && takeoff.maximum != null && takeoff.maximum > takeoff.minimum
