package one.aircast.android.ui

import org.json.JSONObject

internal const val DETECTIONS = "view.detections"

internal data class DetectionBox(
    val x: Double,
    val y: Double,
    val w: Double,
    val h: Double,
    val label: String,
    val confidence: Double,
    val target: Boolean,
)

internal data class Detections(
    val available: Boolean,
    val stale: Boolean,
    val boxes: List<DetectionBox>,
    val error: String?,
)

internal fun detections(view: JSONObject?): Detections? {
    if (view == null || view.optString("class") != "Detections") return null
    val boxes = view.optJSONArray("boxes")
    return Detections(
        available = view.optBoolean("available"),
        stale = view.optBoolean("stale"),
        boxes = (0 until (boxes?.length() ?: 0)).mapNotNull { index ->
            boxes?.optJSONObject(index)?.let { box ->
                DetectionBox(
                    x = box.optDouble("x", 0.0),
                    y = box.optDouble("y", 0.0),
                    w = box.optDouble("w", 0.0),
                    h = box.optDouble("h", 0.0),
                    label = box.optString("label"),
                    confidence = box.optDouble("confidence", 0.0),
                    target = box.optBoolean("target"),
                )
            }
        },
        error = view.takeIf { !it.isNull("error") }?.optString("error")?.ifBlank { null },
    )
}

internal fun visibleBoxes(reading: Detections?): List<DetectionBox> =
    reading?.takeIf { it.available && !it.stale }?.boxes.orEmpty()

internal const val NO_FRAMES = "no frames from the detector"

internal fun detectionTrouble(reading: Detections?): String? =
    reading?.takeIf { it.available }?.let { it.error ?: if (it.stale) NO_FRAMES else null }

internal fun boxCaption(box: DetectionBox): String {
    val percent = (box.confidence * 100).toInt()
    return when {
        box.label.isBlank() && percent <= 0 -> ""
        box.label.isBlank() -> "$percent%"
        percent <= 0 -> box.label
        else -> "${box.label} $percent%"
    }
}
