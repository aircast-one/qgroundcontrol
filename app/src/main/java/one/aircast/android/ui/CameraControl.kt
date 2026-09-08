package one.aircast.android.ui

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

internal fun cameraModeLabel(mode: Int): String? = when (mode) {
    CAM_MODE_PHOTO -> "Photo"
    CAM_MODE_VIDEO -> "Video"
    else -> null
}

internal fun shutterFor(
    mode: Int,
    capturesPhotos: Boolean,
    capturesVideo: Boolean,
    videoStatus: Int,
    photoStatus: Int,
): CameraShutter? = when {
    mode == CAM_MODE_VIDEO && capturesVideo -> CameraShutter(
        label = if (videoStatus == VIDEO_CAPTURE_RUNNING) "Stop" else "Record",
        recording = videoStatus == VIDEO_CAPTURE_RUNNING,
        enabled = true,
    )
    mode == CAM_MODE_PHOTO && capturesPhotos -> CameraShutter(
        label = "Take Photo",
        recording = false,
        enabled = photoStatus != PHOTO_CAPTURE_IN_PROGRESS,
    )
    else -> null
}
