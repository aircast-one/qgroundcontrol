package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal const val CAMERAS_VIEW = "view.cameras"

internal data class CameraKind(
    val raw: String,
    val label: String,
    val group: String,
    val needsUrl: Boolean,
    val hint: String,
    val more: Boolean = false,
    val schemes: List<String> = emptyList(),
)

internal data class CameraEntry(
    val slot: Int,
    val stored: Int?,
    val title: String,
    val name: String,
    val source: String,
    val url: String,
    val summary: String,
    val problem: String?,
    val fromDrone: Boolean,
    val active: Boolean,
)

internal data class CamerasReading(
    val readable: Boolean,
    val reason: String,
    val cameras: List<CameraEntry>,
    val kinds: List<CameraKind>,
) {
    val stored: List<CameraEntry> get() = cameras.filter { it.stored != null }
}

private fun JSONObject.objects(key: String): List<JSONObject> =
    optJSONArray(key)?.let { array -> (0 until array.length()).mapNotNull { array.optJSONObject(it) } }.orEmpty()

internal fun camerasReading(view: JSONObject?): CamerasReading? {
    if (view == null || view.optText("class") != "Cameras") return null
    return CamerasReading(
        readable = view.optBoolean("readable", true),
        reason = view.optText("reason"),
        cameras = view.objects("cameras").map { camera ->
            CameraEntry(
                slot = camera.optInt("slot"),
                stored = camera.takeUnless { it.isNull("stored") }?.optInt("stored"),
                title = camera.optText("title"),
                name = camera.optText("name"),
                source = camera.optText("source"),
                url = camera.optText("url"),
                summary = camera.optText("summary"),
                problem = camera.takeUnless { it.isNull("problem") }?.optText("problem")?.takeIf { it.isNotBlank() },
                fromDrone = camera.optBoolean("fromDrone"),
                active = camera.optBoolean("active"),
            )
        },
        kinds = view.objects("kinds").map { kind ->
            CameraKind(
                raw = kind.optText("raw"),
                label = kind.optText("label"),
                group = kind.optText("group"),
                needsUrl = kind.optBoolean("needsUrl"),
                hint = kind.optText("hint"),
                more = kind.optBoolean("more"),
                schemes = kind.optJSONArray("schemes")?.let { array -> (0 until array.length()).map { array.optString(it) } }.orEmpty(),
            )
        },
    )
}

internal fun inferredKind(kinds: List<CameraKind>, current: String, address: String): String {
    val typed = address.trim().lowercase()
    val scheme = typed.substringBefore("://", "").takeIf { it.isNotEmpty() }?.let { "$it://" } ?: return current
    val accepts = kinds.firstOrNull { it.raw == current }?.schemes.orEmpty()
    return if (scheme in accepts) current else kinds.firstOrNull { scheme in it.schemes }?.raw ?: current
}

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
