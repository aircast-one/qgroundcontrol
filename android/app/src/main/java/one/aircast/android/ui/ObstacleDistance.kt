package one.aircast.android.ui

import one.aircast.mapspike.optText
import org.json.JSONObject

internal data class ObstacleWarning(val label: String, val close: Boolean)

internal fun ageUnknownIsFresh(view: JSONObject): Boolean = !view.isNull("stale") && view.optBoolean("stale")

internal fun obstacleWarning(view: JSONObject?): ObstacleWarning? {
    if (view == null || !view.optBoolean("available") || ageUnknownIsFresh(view)) {
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
