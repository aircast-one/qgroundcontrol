package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object ItemCameraBridge {
    fun read(index: Int): JSONObject? =
        runCatching { JSONObject(QGCBridge.get("view.itemCamera($index)")) }.getOrNull()

    fun set(index: Int, member: String, value: Any): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set(
                    "plan.missionController.visualItems.$index.cameraSection.$member",
                    JSONObject().put("value", value).toString(),
                ),
            ).optBoolean("ok")
        }.getOrDefault(false)

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

internal data class CameraExtras(
    val intervalTime: Double?,
    val intervalDistance: Double?,
    val distanceUnits: String,
    val modeSupported: Boolean,
    val commandsMode: Boolean,
    val mode: Int,
    val commandsGimbal: Boolean,
    val pitch: Double,
    val yaw: Double,
    val pitchRange: ClosedFloatingPointRange<Double>? = null,
    val yawRange: ClosedFloatingPointRange<Double>? = null,
)

private fun JSONObject.measured(key: String): Double? =
    optJSONObject(key)?.optDouble("value")?.takeIf { !it.isNaN() }

private fun JSONObject.userRange(key: String): ClosedFloatingPointRange<Double>? =
    optJSONObject(key)?.optJSONObject("slider")?.let { slider ->
        val (from, to) = slider.optDouble("from") to slider.optDouble("to")
        if (from.isNaN() || to.isNaN() || from == to) null else minOf(from, to)..maxOf(from, to)
    }

internal fun cameraExtras(view: JSONObject?): CameraExtras? =
    view?.takeIf { it.optBoolean("available") }?.let {
        CameraExtras(
            intervalTime = it.measured("intervalTime"),
            intervalDistance = it.measured("intervalDistance"),
            distanceUnits = it.optJSONObject("intervalDistance")?.optText("units").orEmpty().ifBlank { "m" },
            modeSupported = it.optBoolean("cameraModeSupported"),
            commandsMode = it.optBoolean("commandsMode"),
            mode = it.optJSONObject("cameraMode")?.optInt("choice", 0) ?: 0,
            commandsGimbal = it.optBoolean("commandsGimbal"),
            pitch = it.measured("gimbalPitch") ?: 0.0,
            yaw = it.measured("gimbalYaw") ?: 0.0,
            pitchRange = it.userRange("gimbalPitch"),
            yawRange = it.userRange("gimbalYaw"),
        )
    }

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

internal fun itemCameraNote(view: JSONObject?): String? =
    view?.takeIf { it.optBoolean("available") && !it.isNull("note") }?.optText("note")?.ifBlank { null }

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
