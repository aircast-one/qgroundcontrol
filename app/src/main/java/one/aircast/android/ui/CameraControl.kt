package one.aircast.android.ui

import org.json.JSONObject

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
)


internal const val CAMERA_VIEW = "view.camera"

internal data class CameraReading(
    val present: Boolean,
    val hasModes: Boolean,
    val canChangeMode: Boolean,
    val modeText: String,
    val isRecording: Boolean,
    val canPhoto: Boolean,
    val canRecord: Boolean,
    val isTakingPhoto: Boolean,
    val isVideoMode: Boolean,
)

internal fun cameraReading(view: JSONObject?): CameraReading? {
    if (view == null || !view.optBoolean("present")) return null
    return CameraReading(
        present = true,
        hasModes = view.optBoolean("hasModes"),
        canChangeMode = view.optBoolean("canChangeMode"),
        modeText = view.optString("modeText"),
        isRecording = view.optBoolean("isRecording"),
        canPhoto = view.optBoolean("canPhoto"),
        canRecord = view.optBoolean("canRecord"),
        isTakingPhoto = view.optBoolean("isTakingPhoto"),
        isVideoMode = view.optInt("mode", CAM_MODE_UNDEFINED) == CAM_MODE_VIDEO &&
            view.optBoolean("modeKnown"),
    )
}

internal fun shutterFor(camera: CameraReading): CameraShutter? = when {
    camera.isVideoMode && camera.canRecord -> CameraShutter(
        label = if (camera.isRecording) "Stop" else "Record",
        recording = camera.isRecording,
        enabled = true,
    )
    !camera.isVideoMode && camera.canPhoto -> CameraShutter(
        label = "Take Photo",
        recording = false,
        enabled = !camera.isTakingPhoto,
    )
    else -> null
}
