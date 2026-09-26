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
    val selected: Int? = null,
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
        selected = if (view.isNull("selected")) null else view.optInt("selected"),
    )
}

internal const val ZOOM_LOWEST = 0.0
internal const val ZOOM_HIGHEST = 100.0
internal const val CAMERA_ZOOM = "vehicle.cameraManager.currentCameraInstance.zoomLevel"
internal const val CAMERA_START_TRACKING = "vehicle.cameraManager.currentCameraInstance.startTracking"
internal const val CAMERA_STOP_TRACKING = "vehicle.cameraManager.currentCameraInstance.stopTracking"

internal data class TrackingBox(val x: Double, val y: Double, val width: Double, val height: Double)

internal data class TrackingReading(
    val supported: Boolean,
    val requested: Boolean,
    val reported: Boolean,
    val shapes: List<String>,
    val box: TrackingBox?,
)

internal fun trackingReading(view: JSONObject?): TrackingReading? {
    val tracking = view?.optJSONObject("tracking") ?: return null
    if (!tracking.optBoolean("supported")) {
        return null
    }
    val shapes = tracking.optJSONArray("shapes").let { listed ->
        (0 until (listed?.length() ?: 0)).mapNotNull { listed?.optString(it)?.ifBlank { null } }
    }
    val rect = tracking.optJSONObject("rect")
    return TrackingReading(
        supported = true,
        requested = tracking.optBoolean("requested"),
        reported = tracking.optBoolean("reported"),
        shapes = shapes,
        box = rect?.let {
            TrackingBox(it.optDouble("x"), it.optDouble("y"), it.optDouble("width"), it.optDouble("height"))
        }?.takeIf { it.width > 0.0 && it.height > 0.0 },
    )
}

internal const val CAMERA_TRACKING_ARMED =
    "vehicle.cameraManager.currentCameraInstance.trackingEnabled"

internal fun trackingToggleLabel(reading: TrackingReading): String =
    if (reading.requested) "Stop tracking" else "Track something"

internal fun trackingCanAim(reading: TrackingReading?): Boolean =
    reading != null && reading.requested && reading.shapes.isNotEmpty()

internal fun trackingCanStart(reading: TrackingReading?): Boolean =
    reading != null && !reading.requested && reading.shapes.isNotEmpty()

internal fun trackingCanStop(reading: TrackingReading?): Boolean =
    reading != null && reading.requested

internal fun trackingRectObject(box: TrackingBox): JSONObject = JSONObject()
    .put("x", box.x).put("y", box.y).put("width", box.width).put("height", box.height)

internal const val TRACK_POINT_SLOP_DP = 10.0
internal const val TRACK_POINT_RADIUS_DP = 50.0

internal sealed interface TrackingRequest {
    data class Box(val rect: TrackingBox) : TrackingRequest
    data class Point(val x: Double, val y: Double, val radius: Double) : TrackingRequest
}

internal fun trackingRequest(
    pressX: Double,
    pressY: Double,
    releaseX: Double,
    releaseY: Double,
    picture: PaintedRect,
): TrackingRequest? {
    if (picture.width <= 0.0 || picture.height <= 0.0) {
        return null
    }
    fun acrossX(value: Double) = ((value - picture.left) / picture.width).coerceIn(0.0, 1.0)
    fun acrossY(value: Double) = ((value - picture.top) / picture.height).coerceIn(0.0, 1.0)

    val x0 = acrossX(minOf(pressX, releaseX))
    val x1 = acrossX(maxOf(pressX, releaseX))
    val y0 = acrossY(minOf(pressY, releaseY))
    val y1 = acrossY(maxOf(pressY, releaseY))

    val tapped = kotlin.math.abs(releaseX - pressX) < TRACK_POINT_SLOP_DP &&
        kotlin.math.abs(releaseY - pressY) < TRACK_POINT_SLOP_DP
    return when {
        tapped -> TrackingRequest.Point(x0, y0, TRACK_POINT_RADIUS_DP / picture.width)
        x1 - x0 <= 0.0 || y1 - y0 <= 0.0 -> null
        else -> TrackingRequest.Box(TrackingBox(x0, y0, x1 - x0, y1 - y0))
    }
}

internal fun trackingPointObject(point: TrackingRequest.Point): JSONObject =
    JSONObject().put("x", point.x).put("y", point.y)

internal val TRACKING_CENTRE = TrackingBox(0.4, 0.4, 0.2, 0.2)

internal const val CAMERA_FORMAT = "vehicle.cameraManager.currentCameraInstance.formatCard"

internal data class DestructiveAction(
    val id: String,
    val title: String,
    val prompt: String,
    val offer: String,
    val reason: String,
) {
    val ready: Boolean get() = offer == "ready"
    val blocked: Boolean get() = offer == "blocked"
    val shown: Boolean get() = offer != "hidden"
}

internal fun destructiveActions(view: JSONObject?): List<DestructiveAction> {
    val listed = view?.optJSONArray("destructiveActions") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            DestructiveAction(
                id = entry.optText("id").ifBlank { return@mapNotNull null },
                title = entry.optText("title"),
                prompt = entry.optText("prompt"),
                offer = entry.optText("offer"),
                reason = entry.optText("reason"),
            )
        }
    }.filter { it.shown }
}

internal fun destructiveReasonFor(action: DestructiveAction): String? =
    action.reason.ifBlank { null }?.takeIf { action.blocked }

internal fun destructiveInvokePath(id: String): String? = when (id) {
    "resetSettings" -> CAMERA_RESET
    "formatStorage" -> CAMERA_FORMAT
    else -> null
}

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
