package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal data class ObstacleWarning(val label: String, val close: Boolean)

internal fun sensorSaidStale(view: JSONObject): Boolean = !view.isNull("stale") && view.optBoolean("stale")

internal fun obstacleWarning(view: JSONObject?): ObstacleWarning? {
    if (view == null || !view.optBoolean("available") || sensorSaidStale(view)) {
        return null
    }
    val nearest = view.optJSONObject("nearest") ?: return null
    val distance = nearest.optText("distanceText").ifBlank { return null }
    val sector = nearest.optText("sectorText")
    return ObstacleWarning(
        label = listOf(distance, sector).filter { it.isNotBlank() }.joinToString(" "),
        close = nearest.optBoolean("close"),
    )
}

internal data class ObstacleSample(val bearingDegrees: Double, val metres: Double)

internal data class ObstacleRing(
    val samples: List<ObstacleSample>,
    val maxMetres: Double,
    val stale: Boolean,
)

internal fun obstacleRing(view: JSONObject?): ObstacleRing? {
    if (view == null || !view.optBoolean("available")) {
        return null
    }
    val ring = view.optJSONArray("ringMetres") ?: return null
    val increment = view.optDouble("ringIncrement", Double.NaN)
        .takeIf { it.isFinite() && it > 0.0 } ?: return null
    val ceiling = view.optDouble("rangeMaxMetres", Double.NaN)
        .takeIf { it.isFinite() && it > 0.0 } ?: return null
    val offset = view.optDouble("ringOffset", 0.0).takeIf { it.isFinite() } ?: 0.0
    val samples = (0 until ring.length()).mapNotNull { index ->
        if (ring.isNull(index)) {
            null
        } else {
            ring.optDouble(index, Double.NaN)
                .takeIf { it.isFinite() && it >= 0.0 }
                ?.let { ObstacleSample(offset + index * increment, it) }
        }
    }
    return if (samples.isEmpty()) null else ObstacleRing(samples, ceiling, sensorSaidStale(view))
}
