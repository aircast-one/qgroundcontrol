package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object ItemCameraBridge {
    fun read(index: Int): JSONObject? =
        runCatching { JSONObject(QGCBridge.get("view.itemCamera($index)")) }.getOrNull()

    fun chooseAction(index: Int, choice: Int): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set(
                    "plan.missionController.visualItems.$index.cameraSection.cameraAction.enumIndex",
                    JSONObject().put("value", choice).toString(),
                ),
            ).optBoolean("ok")
        }.getOrDefault(false)
}

const val VIDEO_VIEW_PATH = "view.video"

object VideoBridge {
    fun read(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(VIDEO_VIEW_PATH)) }.getOrNull()
}

fun shotPoints(view: JSONObject?): List<TrackPoint> {
    val listed = view?.optJSONArray("shotPoints") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let(::coordinate)
    }
}

internal data class CameraChoices(val labels: List<String>, val chosen: Int)

internal fun cameraChoices(view: JSONObject?): CameraChoices? {
    val measure = view?.takeIf { it.optBoolean("available") }?.optJSONObject("cameraAction")
    val listed = measure?.optJSONArray("choices") ?: return null
    val labels = (0 until listed.length()).map { listed.optString(it) }
    if (labels.none { it.isNotBlank() }) return null
    return CameraChoices(labels, measure.optInt("choice", -1))
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

internal fun actionLabel(view: JSONObject?): String {
    val choices = cameraChoices(view)
    return choices?.labels?.getOrNull(choices.chosen) ?: namedAction(measureText(view, "cameraAction"))
}

internal fun itemCameraTextBeside(view: JSONObject?, pickerLabel: String?): String? =
    itemCameraText(view)?.takeIf { it != pickerLabel }

internal fun itemCameraText(view: JSONObject?): String? {
    if (view == null || !view.optBoolean("available")) return null
    val action = actionLabel(view)
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
