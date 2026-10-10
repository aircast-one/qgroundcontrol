package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal const val CAMERAS_VIEW = "view.cameras"
internal const val VIDEO_SOURCES_PAGE = "Video sources"
private const val CAMERA_GROUP_STREAMS = "Video streams"
private const val CAMERA_GROUP_DEVICE = "This device"
private val CAMERA_GROUP_ORDER = listOf(CAMERA_GROUP_STREAMS, "Vehicle and radio presets", CAMERA_GROUP_DEVICE)
private const val FROM_THE_DRONE = "From the drone"

internal enum class CameraStatus(val token: String, val label: String) {
    Live("live", "Live"),
    Connecting("connecting", "Connecting"),
    NoSignal("noSignal", "No signal"),
    Idle("idle", "Not playing"),
}

internal data class CameraKind(
    val raw: String,
    val label: String,
    val group: String,
    val needsUrl: Boolean,
    val hint: String,
    val description: String = "",
    val defaultAddress: String = "",
)

internal data class CameraEntry(
    val slot: Int,
    val stored: Int?,
    val title: String,
    val short: String,
    val name: String,
    val source: String,
    val url: String,
    val problem: String?,
    val fromDrone: Boolean,
    val active: Boolean,
    val status: CameraStatus,
)

internal data class CameraPip(val enabled: Boolean, val slot: Int?)

internal data class CamerasReading(
    val readable: Boolean,
    val reason: String,
    val pip: CameraPip,
    val cameras: List<CameraEntry>,
    val kinds: List<CameraKind>,
) {
    val stored: List<CameraEntry> get() = cameras.filter { it.stored != null }
}

private fun JSONObject.objects(key: String): List<JSONObject> =
    optJSONArray(key)?.let { array -> (0 until array.length()).mapNotNull { array.optJSONObject(it) } }.orEmpty()

private fun JSONObject.textOrNull(key: String): String? = takeUnless { it.isNull(key) }?.optText(key)?.takeIf { it.isNotBlank() }

private fun JSONObject.intOrNull(key: String): Int? = takeUnless { it.isNull(key) }?.optInt(key)

internal fun camerasReading(view: JSONObject?): CamerasReading? {
    if (view == null || view.optText("class") != "Cameras") return null
    val pip = view.optJSONObject("pip")
    return CamerasReading(
        readable = view.optBoolean("readable", true),
        reason = view.optText("reason"),
        pip = CameraPip(enabled = pip?.optBoolean("enabled") == true, slot = pip?.intOrNull("slot")),
        cameras = view.objects("cameras").map { camera ->
            CameraEntry(
                slot = camera.optInt("slot"),
                stored = camera.intOrNull("stored"),
                title = camera.optText("title"),
                short = camera.optText("short"),
                name = camera.optText("name"),
                source = camera.optText("source"),
                url = camera.optText("url"),
                problem = camera.textOrNull("problem"),
                fromDrone = camera.optBoolean("fromDrone"),
                active = camera.optBoolean("active"),
                status = CameraStatus.entries.firstOrNull { it.token == camera.optText("status") } ?: CameraStatus.Idle,
            )
        },
        kinds = view.objects("kinds").map { kind ->
            CameraKind(
                raw = kind.optText("raw"),
                label = kind.optText("label"),
                group = kind.optText("group"),
                needsUrl = kind.optBoolean("needsUrl"),
                hint = kind.optText("hint"),
                description = kind.optText("description"),
                defaultAddress = kind.optText("defaultAddress"),
            )
        },
    )
}

internal fun cameraDetail(camera: CameraEntry): String = when {
    camera.fromDrone -> FROM_THE_DRONE
    camera.url.contains("://") -> camera.url
    camera.url.isNotBlank() -> "${kindLabel(camera.source)} · ${camera.url}"
    else -> kindLabel(camera.source)
}

internal fun addableKinds(reading: CamerasReading?, keeping: String? = null): List<CameraKind> =
    reading?.kinds.orEmpty()
        .filter { kind -> kind.needsUrl || kind.raw == keeping || reading?.stored.orEmpty().none { it.source == kind.raw } }
        .sortedBy { kind -> CAMERA_GROUP_ORDER.indexOf(kind.group).let { if (it < 0) CAMERA_GROUP_ORDER.size else it } }

internal fun kindTitle(kind: CameraKind): String = if (kind.raw == SYNTHETIC_SOURCE) SYNTHETIC_LABEL else kindLabel(kind.label)

internal fun addedName(kind: CameraKind): String = if (kind.raw == SYNTHETIC_SOURCE) SYNTHETIC_LABEL else ""

internal fun kindLabel(label: String): String = sentenceCase(label.removeSuffix(" Video Stream").ifBlank { label })

internal fun camerasGlance(reading: CamerasReading?): String {
    val cameras = reading?.cameras.orEmpty()
    val active = cameras.firstOrNull { it.active } ?: cameras.firstOrNull()
    return when {
        active == null -> ""
        cameras.size == 1 -> active.title
        else -> "${active.title} · ${cameras.size} cameras"
    }
}
