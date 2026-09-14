package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object ItemCameraBridge {
    fun read(index: Int): JSONObject? =
        runCatching { JSONObject(QGCBridge.get("view.itemCamera($index)")) }.getOrNull()
}

internal fun measureText(view: JSONObject?, key: String): String {
    val measure = view?.optJSONObject(key) ?: return ""
    val text = measure.optText("text")
    if (text.isBlank()) return ""
    val units = measure.optText("units")
    return if (units.isBlank()) text else "$text $units"
}

internal fun namedAction(text: String): String =
    if (text.isBlank() || text.trim().toDoubleOrNull() != null) "" else text

internal fun itemCameraText(view: JSONObject?): String? {
    if (view == null || !view.optBoolean("available")) return null
    val action = namedAction(measureText(view, "cameraAction"))
    val gimbal = when {
        !view.optBoolean("commandsGimbal") -> ""
        else -> listOf(measureText(view, "gimbalPitch"), measureText(view, "gimbalYaw"))
            .filter { it.isNotBlank() }
            .joinToString(" / ")
            .ifBlank { "" }
            .let { if (it.isBlank()) "" else "gimbal $it" }
    }
    return listOf(action, gimbal).filter { it.isNotBlank() }.joinToString(" · ").ifBlank { null }
}
