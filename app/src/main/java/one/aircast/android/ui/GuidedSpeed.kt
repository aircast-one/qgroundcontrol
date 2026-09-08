package one.aircast.android.ui

import java.util.Locale
import org.json.JSONObject

internal const val GUIDED_SPEED = "view.guidedSpeed"

internal data class GuidedSpeed(
    val label: String,
    val unit: String,
    val command: String?,
    val initial: Double?,
    val minimum: Double?,
    val maximum: Double?,
    val sentence: String,
    val targetMetersSecond: Double,
)

internal fun guidedSpeedPath(target: Double): String =
    "$GUIDED_SPEED(${String.format(Locale.US, "%.2f", target)})"

private fun JSONObject.numberOrNull(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

private fun JSONObject.textOrNull(key: String): String? =
    if (isNull(key)) null else optString(key).ifBlank { null }

internal fun guidedSpeed(view: JSONObject?): GuidedSpeed? {
    if (view == null || !view.optBoolean("available")) return null
    return GuidedSpeed(
        label = view.textOrNull("label") ?: "Speed",
        unit = view.optString("unit"),
        command = view.textOrNull("command"),
        initial = view.numberOrNull("initial"),
        minimum = view.numberOrNull("minimum"),
        maximum = view.numberOrNull("maximum"),
        sentence = if (view.isNull("sentence")) "" else view.optString("sentence"),
        targetMetersSecond = view.optDouble("targetMetersSecond", 0.0),
    )
}

internal fun speedRangeUsable(speed: GuidedSpeed?): Boolean =
    speed?.command != null &&
        speed.minimum != null &&
        speed.maximum != null &&
        speed.maximum > speed.minimum
