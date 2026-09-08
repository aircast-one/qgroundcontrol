package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcStrings

private const val MANAGER = "vehicle.cameraManager"
private const val CAMERA = "$MANAGER.currentCameraInstance"

@Composable
fun CameraControlLayer(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val labels by qgcStrings("$MANAGER.cameraLabels")
    val current by qgcDouble("$MANAGER.currentCamera", 0.0)
    val capturesPhotos by qgcBool("$CAMERA.capturesPhotos")
    val capturesVideo by qgcBool("$CAMERA.capturesVideo")
    val hasModes by qgcBool("$CAMERA.hasModes")
    val mode by qgcDouble("$CAMERA.cameraMode", CAM_MODE_UNDEFINED.toDouble())
    val videoStatus by qgcDouble("$CAMERA.videoCaptureStatus", VIDEO_CAPTURE_STOPPED.toDouble())
    val photoStatus by qgcDouble("$CAMERA.photoCaptureStatus", PHOTO_CAPTURE_IDLE.toDouble())

    if (!hasVehicle) {
        return
    }

    val shutter = shutterFor(
        mode = mode.toInt(),
        capturesPhotos = capturesPhotos,
        capturesVideo = capturesVideo,
        videoStatus = videoStatus.toInt(),
        photoStatus = photoStatus.toInt(),
    ) ?: return

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
    ) {
        Row(
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (labels.size > 1) {
                FilterChip(
                    selected = false,
                    onClick = {
                        val next = (current.toInt() + 1) % labels.size
                        offMainDetached { Qgc.set("$MANAGER.currentCamera", next) }
                    },
                    label = { Text(labels.getOrElse(current.toInt()) { "Camera" }) },
                )
            }

            if (hasModes) {
                cameraModeLabel(mode.toInt())?.let { label ->
                    FilterChip(
                        selected = false,
                        onClick = {
                            offMainDetached { Qgc.invoke("$CAMERA.toggleCameraMode") }
                        },
                        label = { Text(label) },
                    )
                }
            }

            Button(
                onClick = {
                    offMainDetached {
                        if (mode.toInt() == CAM_MODE_VIDEO) {
                            Qgc.invoke("$CAMERA.toggleVideoRecording")
                        } else {
                            Qgc.invoke("$CAMERA.takePhoto")
                        }
                    }
                },
                enabled = shutter.enabled,
                colors = if (shutter.recording) {
                    ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error,
                    )
                } else {
                    ButtonDefaults.buttonColors()
                },
            ) { Text(shutter.label) }
        }
    }
}
