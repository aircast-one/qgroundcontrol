package one.aircast.mapspike

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

private val CAMERA_MODES = listOf("Photo", "Video", "Survey")

@Composable
private fun NumberEntry(label: String, value: Double, enabled: Boolean = true, onDone: (Double) -> Unit) {
    var typed by remember(value) { mutableStateOf(trimmedNumber(value)) }
    OutlinedTextField(
        value = typed,
        onValueChange = { typed = it },
        label = { Text(label) },
        enabled = enabled,
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { typed.trim().toDoubleOrNull()?.let(onDone) }),
        modifier = Modifier.width(110.dp),
        textStyle = MaterialTheme.typography.bodySmall,
    )
}

internal fun trimmedNumber(value: Double): String =
    if (value == Math.floor(value)) value.toLong().toString() else value.toString()

@Composable
internal fun CameraSectionExtras(extras: CameraExtras, write: (String, Any) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        extras.intervalTime?.let { time -> NumberEntry("Time (s)", time) { write("cameraPhotoIntervalTime", it) } }
        extras.intervalDistance?.let { distance -> NumberEntry("Distance (m)", distance) { write("cameraPhotoIntervalDistance", it) } }
        if (extras.modeSupported) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Mode", style = MaterialTheme.typography.labelMedium)
                Switch(checked = extras.commandsMode, onCheckedChange = { write("specifyCameraMode", it) })
                CAMERA_MODES.forEachIndexed { at, label ->
                    FilterChip(selected = extras.mode == at, enabled = extras.commandsMode, onClick = { write("cameraMode", at) }, label = { Text(label) })
                }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Text("Gimbal", style = MaterialTheme.typography.labelMedium)
            Switch(checked = extras.commandsGimbal, onCheckedChange = { write("specifyGimbal", it) })
            NumberEntry("Pitch", extras.pitch, enabled = extras.commandsGimbal) { write("gimbalPitch", it.coerceIn(-90.0, 0.0)) }
            NumberEntry("Yaw", extras.yaw, enabled = extras.commandsGimbal) { write("gimbalYaw", it.coerceIn(-180.0, 180.0)) }
        }
    }
}
