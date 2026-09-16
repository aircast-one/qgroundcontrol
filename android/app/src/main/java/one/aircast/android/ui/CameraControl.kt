package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val CAM_MODE_UNDEFINED = -1
internal const val CAM_MODE_PHOTO = 0
internal const val CAM_MODE_VIDEO = 1

internal const val VIDEO_CAPTURE_STOPPED = 0
internal const val VIDEO_CAPTURE_RUNNING = 1

internal const val PHOTO_CAPTURE_IDLE = 0
internal const val PHOTO_CAPTURE_IN_PROGRESS = 1

data class CameraShutter(
    val label: String,
    val recording: Boolean,
    val enabled: Boolean,
    val action: String,
)


internal const val CAMERA_VIEW = "view.camera"

internal data class CameraReading(
    val hasModes: Boolean,
    val canChangeMode: Boolean,
    val modeText: String,
    val isRecording: Boolean,
    val canPhoto: Boolean,
    val canRecord: Boolean,
    val isTakingPhoto: Boolean,
    val isVideoMode: Boolean,
    val timelapse: Boolean,
    val canStopPhoto: Boolean,
    val lapseSeconds: Double?,
    val lapseCount: Int?,
    val lapseUnlimited: Boolean,
    val title: String,
    val labels: List<String>,
    val stateText: String,
    val reportsStorage: Boolean,
    val storageText: String,
    val shotsText: String,
    val batteryText: String,
    val hasZoom: Boolean,
    val zoomLevel: Double,
)

internal fun cameraReading(view: JSONObject?): CameraReading? {
    if (view == null || !view.optBoolean("present")) return null
    return CameraReading(
        hasModes = view.optBoolean("hasModes"),
        canChangeMode = view.optBoolean("canChangeMode"),
        modeText = view.optText("modeText"),
        isRecording = view.optBoolean("isRecording"),
        canPhoto = view.optBoolean("canPhoto"),
        canRecord = view.optBoolean("canRecord"),
        isTakingPhoto = view.optBoolean("isTakingPhoto"),
        isVideoMode = view.optInt("mode", CAM_MODE_UNDEFINED) == CAM_MODE_VIDEO &&
            view.optBoolean("modeKnown"),
        timelapse = view.optText("photoMode") == "timelapse",
        canStopPhoto = view.optBoolean("canStopPhoto"),
        lapseSeconds = view.optDouble("lapseSeconds").takeIf { it.isFinite() },
        lapseCount = if (view.isNull("lapseCount")) null else view.optInt("lapseCount"),
        lapseUnlimited = view.optBoolean("lapseUnlimited"),
        title = view.optText("title"),
        labels = view.optJSONArray("labels").let { listed ->
            (0 until (listed?.length() ?: 0)).map { listed?.optText(it).orEmpty() }
        },
        stateText = view.optText("stateText"),
        reportsStorage = view.optBoolean("reportsStorage"),
        storageText = view.optText("storageText"),
        shotsText = view.optText("shotsText"),
        batteryText = view.optText("batteryText"),
        hasZoom = view.optBoolean("hasZoom"),
        zoomLevel = view.optDouble("zoomLevel", ZOOM_LOWEST).takeIf { it.isFinite() } ?: ZOOM_LOWEST,
    )
}

internal const val ZOOM_LOWEST = 0.0
internal const val ZOOM_HIGHEST = 100.0
internal const val CAMERA_ZOOM = "vehicle.cameraManager.currentCameraInstance.zoomLevel"
internal const val CAMERA_THERMAL_MODE = "vehicle.cameraManager.currentCameraInstance.thermalMode"
internal const val CAMERA_THERMAL_OPACITY = "vehicle.cameraManager.currentCameraInstance.thermalOpacity"

internal val THERMAL_MODES = listOf("off", "blend", "full", "picInPic")

internal fun thermalModeLabel(token: String): String = when (token) {
    "off" -> "Off"
    "blend" -> "Blend"
    "full" -> "Full"
    "picInPic" -> "Picture in picture"
    else -> token
}

internal data class ThermalReading(val mode: String, val opacity: Double?)

internal fun thermalReading(view: JSONObject?): ThermalReading? {
    if (view == null || !view.optBoolean("thermalAvailable")) {
        return null
    }
    val mode = view.optText("thermalMode").ifBlank { return null }
    val opacity = if (view.isNull("thermalOpacity")) null else view.optDouble("thermalOpacity")
    return ThermalReading(mode, opacity?.takeIf { it.isFinite() })
}

internal fun thermalOpacityIsOffered(reading: ThermalReading?): Boolean =
    reading != null && reading.opacity != null

internal const val CAMERA_RESET = "vehicle.cameraManager.currentCameraInstance.resetSettings"

internal const val RESET_TITLE = "Reset Camera to Factory Settings"
internal const val RESET_PROMPT = "Confirm resetting all settings?"

internal fun cameraCanReset(camera: CameraReading?): Boolean = camera != null

internal fun zoomStep(camera: CameraReading, by: Double): Double? {
    if (!camera.hasZoom) return null
    val wanted = (camera.zoomLevel + by).coerceIn(ZOOM_LOWEST, ZOOM_HIGHEST)
    return wanted.takeIf { it != camera.zoomLevel }
}

internal fun zoomText(camera: CameraReading): String? =
    if (camera.hasZoom) "Zoom ${camera.zoomLevel.toInt()}%" else null

internal const val CAMERA_PHOTO = "camera.takePhoto"
internal const val CAMERA_RECORD = "camera.toggleRecording"
internal const val CAMERA_STOP_PHOTO = "camera.stopPhoto"
internal const val CAMERA_SET_MODE = "camera.setMode"

internal fun cameraDetails(camera: CameraReading): List<Pair<String, String>> = listOfNotNull(
    "State" to camera.stateText,
    ("Storage" to camera.storageText).takeIf { camera.reportsStorage && camera.storageText.isNotBlank() },
    ("Photos" to camera.shotsText).takeIf { camera.shotsText.isNotBlank() },
    ("Battery" to camera.batteryText).takeIf { camera.batteryText.isNotBlank() },
)

internal fun lapsePlan(camera: CameraReading): String? {
    if (!camera.timelapse) return null
    val every = camera.lapseSeconds?.takeIf { it > 0 }?.let { "every ${trimmed(it)} s" }
    val many = when {
        camera.lapseUnlimited -> "until stopped"
        else -> camera.lapseCount?.takeIf { it > 0 }?.let { "$it shots" }
    }
    return listOfNotNull(every, many).joinToString(", ").ifBlank { "interval capture" }
}

private fun trimmed(value: Double): String =
    if (value == value.toLong().toDouble()) value.toLong().toString() else "%.1f".format(value)

internal fun shutterFor(camera: CameraReading): CameraShutter? = when {
    camera.canStopPhoto -> CameraShutter(
        label = "Stop lapse",
        recording = true,
        enabled = true,
        action = CAMERA_STOP_PHOTO,
    )
    camera.isVideoMode && camera.canRecord -> CameraShutter(
        label = if (camera.isRecording) "Stop" else "Record",
        recording = camera.isRecording,
        enabled = true,
        action = CAMERA_RECORD,
    )
    !camera.isVideoMode && camera.canPhoto -> CameraShutter(
        label = if (camera.timelapse) "Start lapse" else "Take Photo",
        recording = false,
        enabled = !camera.isTakingPhoto,
        action = CAMERA_PHOTO,
    )
    else -> null
}
