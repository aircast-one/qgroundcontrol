package one.aircast.android.ui

import android.Manifest
import android.content.pm.PackageManager
import android.hardware.Camera
import android.view.View
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private const val DEVICE_CAMERA = "deviceCamera"
private const val QUARTER_TURN = 90
private const val FULL_TURN = 360

internal fun deviceCameraRotation(sensorDegrees: Int, front: Boolean, displayDegrees: Int): Int =
    if (front) (sensorDegrees + displayDegrees) % FULL_TURN else (sensorDegrees - displayDegrees + FULL_TURN) % FULL_TURN

private fun displayDegrees(view: View): Int = (view.display?.rotation ?: 0) * QUARTER_TURN

@Suppress("DEPRECATION")
private fun cameraRotation(camera: Int, displayDegrees: Int): Int? = runCatching {
    val info = Camera.CameraInfo().also { Camera.getCameraInfo(camera, it) }
    deviceCameraRotation(info.orientation, info.facing == Camera.CameraInfo.CAMERA_FACING_FRONT, displayDegrees)
}.getOrNull()

@Composable
fun DeviceCameraHost() {
    val videoJson by qgcPath(VIDEO_VIEW)
    val camera = videoJson?.takeUnless { it.isNull(DEVICE_CAMERA) }?.optInt(DEVICE_CAMERA)
    val context = LocalContext.current
    val view = LocalView.current
    val configuration = LocalConfiguration.current
    val display = remember(configuration) { displayDegrees(view) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) offMainDetached { VideoCommands.restart() }
    }

    LaunchedEffect(camera) {
        if (camera != null && context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            permission.launch(Manifest.permission.CAMERA)
        }
    }

    LaunchedEffect(camera, display) {
        val rotation = camera?.let { cameraRotation(it, display) } ?: return@LaunchedEffect
        offMainDetached { VideoCommands.setDeviceCameraRotation(rotation) }
    }
}
