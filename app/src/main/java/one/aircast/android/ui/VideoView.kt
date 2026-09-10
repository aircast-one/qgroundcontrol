package one.aircast.android.ui

import org.json.JSONObject

internal const val VIDEO_VIEW = "view.video"

internal data class VideoCamera(
    val slot: Int,
    val title: String,
    val status: String,
    val connecting: Boolean,
    val recording: Boolean,
    val configured: Boolean,
)

internal data class VideoReading(
    val available: Boolean,
    val decoding: Boolean,
    val summary: String,
    val activeSource: Int,
    val multipleSources: Boolean,
    val cameras: List<VideoCamera>,
)

internal fun videoReading(view: JSONObject?): VideoReading? {
    if (view == null || view.optString("class") != "Video") return null
    val cameras = view.optJSONArray("cameras")
    return VideoReading(
        available = view.optBoolean("available"),
        decoding = view.optBoolean("decoding"),
        summary = view.optString("summary"),
        activeSource = view.optInt("activeSource"),
        multipleSources = view.optBoolean("multipleSources"),
        cameras = (0 until (cameras?.length() ?: 0)).mapNotNull { index ->
            cameras?.optJSONObject(index)?.let { camera ->
                VideoCamera(
                    slot = camera.optInt("slot"),
                    title = camera.optString("title"),
                    status = camera.optString("status"),
                    connecting = camera.optBoolean("connecting"),
                    recording = camera.optBoolean("recording"),
                    configured = camera.optBoolean("configured"),
                )
            }
        },
    )
}

internal fun switchableSources(reading: VideoReading?): List<VideoCamera> =
    reading?.takeIf { it.multipleSources }
        ?.cameras
        ?.filter { it.configured }
        ?.takeIf { it.size > 1 }
        .orEmpty()
