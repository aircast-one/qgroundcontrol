package one.aircast.android.ui

import androidx.compose.foundation.layout.heightIn
import androidx.compose.ui.semantics.Role
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.RadioButton
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import one.aircast.android.R
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.foundation.BorderStroke
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.material3.Slider
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.delay
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.isActive
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.settingControl
import androidx.compose.runtime.remember

private const val REFUSAL_MS = 4000L
private const val ZOOM_TICK = 10.0

private const val MANAGER = "vehicle.cameraManager"

internal const val SHOW_PHOTO_VIDEO_CONTROL = "settings.flyViewSettings.showPhotoVideoControl"
private const val CAMERA_SCRIM_ALPHA = 0.55f
private val SHUTTER_SIZE = 40.dp

@Composable
fun CameraControlLayer(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
    val shown by qgcBool(settingControl(SHOW_PHOTO_VIDEO_CONTROL))
    val cameraJson by qgcPath(CAMERA_VIEW)
    val camera = remember(cameraJson) { cameraReading(cameraJson) }
    var refused by remember { mutableStateOf<String?>(null) }
    var details by remember { mutableStateOf(false) }

    LaunchedEffect(refused) {
        if (refused != null) {
            delay(REFUSAL_MS)
            refused = null
        }
    }

    if (!hasVehicle || !shown) {
        return
    }

    val shutter = camera?.let { shutterFor(it) } ?: return

    Surface(
        modifier = modifier,
        shape = MaterialTheme.shapes.extraLarge,
        color = Color.Black.copy(alpha = CAMERA_SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
    ) {
        Column {
        Row(
            Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                onClick = { offMainDetached { refused = Qgc.refusalOf(shutter.action) } },
                enabled = shutter.enabled,
                modifier = Modifier.size(SHUTTER_SIZE).semantics { contentDescription = shutter.label },
                shape = CircleShape,
                color = Color.Transparent,
                border = BorderStroke(3.dp, MaterialTheme.aircast.outdoorForeground),
            ) {
                Box(Modifier.padding(5.dp), contentAlignment = Alignment.Center) {
                    Box(
                        Modifier
                            .size(if (shutter.recording) 14.dp else SHUTTER_SIZE)
                            .background(
                                if (shutter.recording || camera.isVideoMode) MaterialTheme.colorScheme.error else MaterialTheme.aircast.outdoorForeground,
                                if (shutter.recording) MaterialTheme.shapes.extraSmall else CircleShape,
                            ),
                    )
                }
            }

            if (!camera.isVideoMode && camera.canPhoto) PhotoCount()

            if (camera.hasModes) {
                Row(
                    Modifier.background(Color.Black.copy(alpha = CAMERA_SCRIM_ALPHA), CircleShape).padding(2.dp),
                    horizontalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    listOf(false to R.drawable.ic_photo_camera, true to R.drawable.ic_videocam).map { (video, icon) ->
                        val selected = camera.isVideoMode == video
                        Surface(
                            onClick = {
                                if (!selected) offMainDetached {
                                    refused = Qgc.refusalOf(CAMERA_SET_MODE, if (video) "video" else "photo")
                                }
                            },
                            enabled = camera.canChangeMode,
                            shape = CircleShape,
                            color = if (selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent,
                            contentColor = if (selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.aircast.outdoorForeground,
                        ) {
                            Icon(
                                painterResource(icon),
                                if (video) "Video" else "Photo",
                                Modifier.padding(6.dp).size(20.dp),
                            )
                        }
                    }
                }
            }

            zoomText(camera)?.let { label ->
                IconButton(
                    onClick = {
                        zoomStep(camera, -ZOOM_TICK)?.let { level ->
                            offMainDetached { refused = Qgc.writeRefusal(CAMERA_ZOOM, level) }
                        }
                    },
                    enabled = zoomStep(camera, -ZOOM_TICK) != null,
                    modifier = Modifier.size(32.dp),
                ) { Text("\u2212", style = MaterialTheme.typography.titleMedium) }
                Text(label, style = MaterialTheme.typography.labelLarge)
                IconButton(
                    onClick = {
                        zoomStep(camera, ZOOM_TICK)?.let { level ->
                            offMainDetached { refused = Qgc.writeRefusal(CAMERA_ZOOM, level) }
                        }
                    },
                    enabled = zoomStep(camera, ZOOM_TICK) != null,
                    modifier = Modifier.size(32.dp),
                ) { Text("+", style = MaterialTheme.typography.titleMedium) }
            }

            TextButton(onClick = { details = true }, colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.aircast.outdoorForeground)) {
                Text(
                    camera.labels.getOrElse(camera.selected ?: 0) { camera.title.ifBlank { "Camera" } },
                    style = MaterialTheme.typography.labelLarge,
                    maxLines = 1,
                )
            }

            lapsePlan(camera)?.let { plan ->
                Text(text = plan, style = MaterialTheme.typography.labelSmall)
            }
        }

        if (details) {
            CameraDetailsSheet(
                camera = camera,
                thermal = remember(cameraJson) { thermalReading(cameraJson) },
                tracking = remember(cameraJson) { trackingReading(cameraJson) },
                destructive = remember(cameraJson) { destructiveActions(cameraJson) },
                current = camera.selected ?: 0,
                onSelect = { index ->
                    offMainDetached { Qgc.set("$MANAGER.currentCamera", index) }
                    details = false
                },
                onDismiss = { details = false },
            )
        }

        refused?.let { sentence ->
            Text(
                text = sentence,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
            )
        }
        }
    }
}

@Composable
private fun SheetRadioRow(label: String, selected: Boolean, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().selectable(selected = selected, role = Role.RadioButton, onClick = onClick).heightIn(min = 56.dp).padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        RadioButton(selected = selected, onClick = null)
        Text(label, style = MaterialTheme.typography.bodyLarge)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CameraDetailsSheet(
    camera: CameraReading,
    thermal: ThermalReading?,
    tracking: TrackingReading?,
    destructive: List<DestructiveAction>,
    current: Int,
    onSelect: (Int) -> Unit,
    onDismiss: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Text(
            camera.title.ifBlank { "Camera" },
            Modifier.padding(horizontal = 16.dp),
            style = MaterialTheme.typography.titleLarge,
        )
        cameraDetails(camera).forEach { (label, reading) ->
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(label, style = MaterialTheme.typography.bodyMedium)
                Text(reading, style = MaterialTheme.typography.bodyMedium)
            }
        }
        if (camera.labels.size > 1) {
            SectionHeader("Cameras")
            camera.labels.forEachIndexed { index, label ->
                SheetRadioRow(label, index == current) { onSelect(index) }
            }
        }
        if (camera.streamLabels.size > 1) {
            SectionHeader("Video Stream")
            camera.streamLabels.forEachIndexed { index, label ->
                SheetRadioRow(label, index == camera.currentStream) { offMainDetached { Qgc.set(CAMERA_CURRENT_STREAM, index) } }
            }
        }
        if (camera.canPhoto) {
            SectionHeader("Photo Mode")
            Row(Modifier.padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf("Single" to false, "Time Lapse" to true).forEachIndexed { index, (label, lapse) ->
                    FilterChip(
                        selected = camera.timelapse == lapse,
                        onClick = { offMainDetached { Qgc.set(CAMERA_PHOTO_MODE, index) } },
                        label = { Text(label) },
                    )
                }
            }
            if (camera.timelapse) {
                var interval by remember(camera.lapseSeconds) {
                    mutableFloatStateOf((camera.lapseSeconds ?: 1.0).toFloat().coerceIn(PHOTO_LAPSE_MIN_S, PHOTO_LAPSE_MAX_S))
                }
                Text(
                    "Photo Interval (seconds)  ${interval.toInt()}",
                    Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                    style = MaterialTheme.typography.titleSmall,
                )
                Slider(
                    value = interval,
                    onValueChange = { interval = it },
                    onValueChangeFinished = { offMainDetached { Qgc.set(CAMERA_PHOTO_LAPSE, interval.toInt().toDouble()) } },
                    valueRange = PHOTO_LAPSE_MIN_S..PHOTO_LAPSE_MAX_S,
                    steps = (PHOTO_LAPSE_MAX_S - PHOTO_LAPSE_MIN_S).toInt() - 1,
                    modifier = Modifier.padding(horizontal = 20.dp),
                )
            }
        }
        tracking?.let { reading ->
            TextButton(
                onClick = {
                    offMainDetached {
                        when {
                            reading.requested -> {
                                Qgc.set(CAMERA_TRACKING_ARMED, false)
                                Qgc.invoke(CAMERA_STOP_TRACKING)
                            }
                            else -> Qgc.set(CAMERA_TRACKING_ARMED, true)
                        }
                    }
                },
                modifier = Modifier.padding(horizontal = 12.dp),
            ) { Text(trackingToggleLabel(reading)) }
        }

        thermal?.let { thermal ->
            SectionHeader("Thermal View Mode")
            THERMAL_MODES.forEach { token ->
                SheetRadioRow(thermalModeLabel(token), token == thermal.mode) {
                    offMainDetached { Qgc.set(CAMERA_THERMAL_MODE, THERMAL_MODES.indexOf(token)) }
                }
            }
            if (thermalOpacityIsOffered(thermal)) {
                SectionHeader("Blend Opacity")
                var typed by remember(thermal.opacity) {
                    mutableFloatStateOf((thermal.opacity ?: 0.0).toFloat())
                }
                Slider(
                    value = typed,
                    onValueChange = { typed = it },
                    onValueChangeFinished = {
                        offMainDetached { Qgc.set(CAMERA_THERMAL_OPACITY, typed.toDouble()) }
                    },
                    valueRange = 0f..100f,
                    modifier = Modifier.padding(horizontal = 20.dp),
                )
            }
        }

        CameraDefinitionSettings()

        destructive.forEach { action ->
            var confirming by remember(action.id) { mutableStateOf(false) }

            TextButton(
                onClick = { confirming = true },
                enabled = action.ready,
                modifier = Modifier.padding(horizontal = 12.dp),
            ) {
                Text(action.title, color = MaterialTheme.colorScheme.error)
            }
            destructiveReasonFor(action)?.let { reason ->
                Text(
                    text = reason,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 20.dp),
                )
            }

            if (confirming) {
                AlertDialog(
                    onDismissRequest = { confirming = false },
                    title = { Text(action.title) },
                    text = { Text(action.prompt) },
                    confirmButton = {
                        TextButton(onClick = {
                            confirming = false
                            destructiveInvokePath(action.id)?.let { path ->
                                offMainDetached {
                                    if (action.id == "formatStorage") {
                                        Qgc.invoke(path, 1)
                                    } else {
                                        Qgc.invoke(path)
                                    }
                                }
                            }
                        }) { Text("Confirm", color = MaterialTheme.colorScheme.error) }
                    },
                    dismissButton = {
                        TextButton(onClick = { confirming = false }) { Text("Cancel") }
                    },
                )
            }
        }

        Spacer(Modifier.padding(bottom = 24.dp))
    }
}

private const val TRIGGER_COUNT = "vehicle.cameraTriggerPoints.count"
private const val TRIGGER_POLL_MS = 1000L

internal fun photoCountText(count: Int): String = ("00000$count").takeLast(5)

@Composable
private fun PhotoCount() {
    var count by remember { mutableStateOf(0) }
    LaunchedEffect(Unit) {
        while (isActive) {
            count = withContext(Dispatchers.Default) { Qgc.get(TRIGGER_COUNT)?.optInt("value") ?: 0 }
            delay(TRIGGER_POLL_MS)
        }
    }
    Text(photoCountText(count), style = MaterialTheme.typography.labelMedium)
}
