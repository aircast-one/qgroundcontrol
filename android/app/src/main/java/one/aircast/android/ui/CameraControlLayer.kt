package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import androidx.compose.runtime.remember

private const val REFUSAL_MS = 4000L
private const val ZOOM_TICK = 10.0

private const val MANAGER = "vehicle.cameraManager"

private val SHUTTER_RING = Color(0xFFF5F5F5)

@Composable
fun CameraControlLayer(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
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

    if (!hasVehicle) {
        return
    }

    val shutter = camera?.let { shutterFor(it) } ?: return

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
    ) {
        Row(
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Button(
                onClick = {
                    offMainDetached { refused = Qgc.refusalOf(shutter.action) }
                },
                enabled = shutter.enabled,
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (shutter.recording) {
                        MaterialTheme.colorScheme.error
                    } else {
                        Color.Transparent
                    },
                    contentColor = if (shutter.recording) {
                        MaterialTheme.colorScheme.onError
                    } else {
                        MaterialTheme.colorScheme.onSurface
                    },
                ),
                border = if (shutter.recording) null else BorderStroke(2.dp, SHUTTER_RING),
            ) { Text(shutter.label) }

            FilterChip(
                selected = false,
                onClick = { details = true },
                label = {
                    Text(camera.labels.getOrElse(camera.selected ?: 0) { camera.title.ifBlank { "Camera" } })
                },
            )

            if (camera.hasModes) {
                camera.modeText.ifBlank { null }?.let { label ->
                    FilterChip(
                        selected = false,
                        enabled = camera.canChangeMode,
                        onClick = {
                            offMainDetached {
                                refused = Qgc.refusalOf(
                                    CAMERA_SET_MODE,
                                    if (camera.isVideoMode) "photo" else "video",
                                )
                            }
                        },
                        label = { Text(label) },
                    )
                }
            }

            zoomText(camera)?.let { label ->
                FilterChip(
                    selected = false,
                    enabled = zoomStep(camera, -ZOOM_TICK) != null,
                    onClick = {
                        zoomStep(camera, -ZOOM_TICK)?.let { level ->
                            offMainDetached { refused = Qgc.writeRefusal(CAMERA_ZOOM, level) }
                        }
                    },
                    label = { Text("\u2212") },
                )
                Text(label, style = MaterialTheme.typography.labelSmall)
                FilterChip(
                    selected = false,
                    enabled = zoomStep(camera, ZOOM_TICK) != null,
                    onClick = {
                        zoomStep(camera, ZOOM_TICK)?.let { level ->
                            offMainDetached { refused = Qgc.writeRefusal(CAMERA_ZOOM, level) }
                        }
                    },
                    label = { Text("+") },
                )
            }


            lapsePlan(camera)?.let { plan ->
                Text(
                    text = plan,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
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
            Modifier.padding(horizontal = 20.dp),
            style = MaterialTheme.typography.titleSmall,
        )
        cameraDetails(camera).forEach { (label, reading) ->
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(label, style = MaterialTheme.typography.bodyMedium)
                Text(reading, style = MaterialTheme.typography.bodyMedium)
            }
        }
        if (camera.labels.size > 1) {
            Text(
                "Cameras",
                Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                style = MaterialTheme.typography.titleSmall,
            )
            camera.labels.forEachIndexed { index, label ->
                FilterChip(
                    selected = index == current,
                    onClick = { onSelect(index) },
                    label = { Text(label) },
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp),
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
            Text(
                "Thermal View Mode",
                Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                style = MaterialTheme.typography.titleSmall,
            )
            THERMAL_MODES.forEach { token ->
                FilterChip(
                    selected = token == thermal.mode,
                    onClick = {
                        offMainDetached { Qgc.set(CAMERA_THERMAL_MODE, THERMAL_MODES.indexOf(token)) }
                    },
                    label = { Text(thermalModeLabel(token)) },
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp),
                )
            }
            if (thermalOpacityIsOffered(thermal)) {
                Text(
                    "Blend Opacity",
                    Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                    style = MaterialTheme.typography.titleSmall,
                )
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
