package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.map.optText

internal const val VIDEO_VIEW = "view.video"

internal data class VideoCamera(
    val slot: Int,
    val status: String,
    val configured: Boolean,
)

internal data class SourceSize(val width: Int, val height: Int)

internal data class VideoReading(
    val available: Boolean,
    val decoding: Boolean,
    val sourceSize: SourceSize?,
    val summary: String,
    val activeSource: Int,
    val cameras: List<VideoCamera>,
    val streaming: Boolean = false,
    val streamEnabled: Boolean = true,
    val noVideoText: String = "",
    val noVideoReason: String = "",
    val sourceChosen: Boolean = true,
)

internal fun videoReading(view: JSONObject?): VideoReading? {
    if (view == null || view.optText("class") != "Video") return null
    val cameras = view.optJSONArray("cameras")
    return VideoReading(
        available = view.optBoolean("available"),
        decoding = view.optBoolean("decoding"),
        sourceSize = view.optJSONObject("sourceSize")?.let { size ->
            val width = size.optInt("width")
            val height = size.optInt("height")
            if (width > 0 && height > 0) SourceSize(width, height) else null
        },
        summary = view.optText("summary"),
        activeSource = view.optInt("activeSource"),
        streaming = view.optBoolean("streaming"),
        streamEnabled = view.optBoolean("streamEnabled", true),
        noVideoText = view.optText("noVideoText"),
        noVideoReason = view.optText("noVideoReason"),
        sourceChosen = view.optBoolean("sourceChosen", true),
        cameras = (0 until (cameras?.length() ?: 0)).mapNotNull { index ->
            cameras?.optJSONObject(index)?.let { camera ->
                VideoCamera(
                    slot = camera.optInt("slot"),
                    status = camera.optText("status"),
                    configured = camera.optBoolean("configured"),
                )
            }
        },
    )
}

internal data class PaintedRect(val left: Double, val top: Double, val width: Double, val height: Double)

internal fun paintedRect(surfaceWidth: Double, surfaceHeight: Double, source: SourceSize?): PaintedRect {
    val whole = PaintedRect(0.0, 0.0, surfaceWidth, surfaceHeight)
    if (source == null || surfaceWidth <= 0.0 || surfaceHeight <= 0.0) {
        return whole
    }
    val scale = minOf(surfaceWidth / source.width, surfaceHeight / source.height)
    val width = source.width * scale
    val height = source.height * scale
    return PaintedRect((surfaceWidth - width) / 2, (surfaceHeight - height) / 2, width, height)
}
