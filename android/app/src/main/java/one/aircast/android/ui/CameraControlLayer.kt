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
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcStrings

private const val REFUSAL_MS = 4000L

private const val MANAGER = "vehicle.cameraManager"

@Composable
fun CameraControlLayer(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val labels by qgcStrings("$MANAGER.cameraLabels")
    val current by qgcDouble("$MANAGER.currentCamera", 0.0)
    val cameraJson by qgcPath(CAMERA_VIEW)
    val camera = remember(cameraJson) { cameraReading(cameraJson) }
    var refused by remember { mutableStateOf<String?>(null) }

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

            Button(
                onClick = {
                    offMainDetached { refused = Qgc.refusalOf(shutter.action) }
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

            lapsePlan(camera)?.let { plan ->
                Text(
                    text = plan,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
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
