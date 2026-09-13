package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val GEOTAG_VIEW = "view.geoTag"

internal data class TelemetryLog(val path: String, val name: String, val bytes: Long)

internal data class GeoTagReading(
    val readable: Boolean,
    val bytes: Long,
    val triggerCount: Int,
    val refusal: String,
)

internal fun geoTagPath(path: String): String = "$GEOTAG_VIEW($path)"

internal fun geoTagReading(view: JSONObject?): GeoTagReading? {
    if (view == null || view.optText("class") != "GeoTag") return null
    return GeoTagReading(
        readable = view.optBoolean("readable"),
        bytes = view.optLong("bytes"),
        triggerCount = view.optInt("triggerCount"),
        refusal = view.optText("refusal"),
    )
}

internal fun triggerSummary(reading: GeoTagReading): String = when {
    !reading.readable -> "This file could not be read."
    reading.triggerCount == 0 -> "No camera triggers were recorded, so there is nothing to match photographs against."
    reading.triggerCount == 1 -> "1 camera trigger recorded."
    else -> "${reading.triggerCount} camera triggers recorded."
}

internal fun logSize(bytes: Long): String = when {
    bytes >= 1024 * 1024 -> "%.1f MB".format(bytes / 1048576.0)
    bytes >= 1024 -> "%.0f KB".format(bytes / 1024.0)
    else -> "$bytes B"
}
