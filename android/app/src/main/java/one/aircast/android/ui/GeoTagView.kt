package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val GEOTAG_ROOT = "geoTag"

internal data class GeoTagState(
    val logFile: String,
    val imageDirectory: String,
    val saveDirectory: String,
    val errorMessage: String,
    val progress: Double,
    val inProgress: Boolean,
    val tagged: Int,
    val skipped: Int,
    val failed: Int,
    val timeOffsetSecs: Double,
    val previewMode: Boolean,
)

internal fun geoTagState(json: JSONObject?): GeoTagState? =
    json?.takeIf { it.optText("class") == "GeoTagController" }?.let {
        GeoTagState(
            logFile = it.optText("logFile"),
            imageDirectory = it.optText("imageDirectory"),
            saveDirectory = it.optText("saveDirectory"),
            errorMessage = it.optText("errorMessage"),
            progress = it.optDouble("progress", 0.0),
            inProgress = it.optBoolean("inProgress"),
            tagged = it.optInt("taggedCount"),
            skipped = it.optInt("skippedCount"),
            failed = it.optInt("failedCount"),
            timeOffsetSecs = it.optDouble("timeOffsetSecs", 0.0),
            previewMode = it.optBoolean("previewMode"),
        )
    }

internal fun geoTagButton(state: GeoTagState): String = when {
    state.inProgress -> "Cancel"
    state.previewMode -> "Preview"
    else -> "Start Tagging"
}

internal fun geoTagSummary(state: GeoTagState): String? =
    state.takeIf { !it.inProgress && it.tagged > 0 }?.let {
        val details = listOfNotNull(
            "${it.skipped} skipped".takeIf { _ -> it.skipped > 0 },
            "${it.failed} failed".takeIf { _ -> it.failed > 0 },
        )
        "Successfully tagged ${it.tagged} images" + if (details.isEmpty()) "" else " (${details.joinToString(", ")})"
    }

internal fun geoTagStep(done: Boolean, number: Int): String = if (done) "✓" else "$number"

internal const val GEOTAG_IMAGE_EXTENSIONS = "jpg,jpeg,tiff,tif,dng"

internal fun isGeoTagImage(name: String): Boolean =
    name.substringAfterLast('.', "").lowercase() in GEOTAG_IMAGE_EXTENSIONS.split(',')
