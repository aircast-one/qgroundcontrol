package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.settingControl
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString

private const val RC_CONTROLS = "settings.flyViewSettings.rcControls"

private val CAMERA_CHANNEL_SETTINGS = listOf(
    "gimbalTiltChannel" to "Gimbal tilt",
    "gimbalPanChannel" to "Gimbal pan",
    "cameraZoomChannel" to "Camera zoom",
    "cameraLightChannel" to "Camera light",
    "cameraRecordChannel" to "Camera record",
)

private const val UNDO_WINDOW_MS = 6000L

private data class Draft(val index: Int, val label: String, val channel: String, val type: RcControlType)

@Composable
private fun reservedChannels(): Map<Int, String> = CAMERA_CHANNEL_SETTINGS.associate { (name, owner) ->
    val channel by qgcDouble(settingControl("settings.flyViewSettings.$name"), 0.0)
    channel.toInt() to owner
}.filterKeys { it > 0 }

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun RcControlsEditor(modifier: Modifier = Modifier) {
    val json by qgcString(settingControl(RC_CONTROLS))
    val controls = remember(json) { parseRcControls(json) }
    val reserved = reservedChannels()
    var draft by remember { mutableStateOf<Draft?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var undo by remember { mutableStateOf<Pair<String, String>?>(null) }

    LaunchedEffect(undo) {
        if (undo != null) {
            kotlinx.coroutines.delay(UNDO_WINDOW_MS)
            undo = null
        }
    }

    fun save(next: String) {
        notice = null
        offMainDetached {
            if (!Qgc.set(RC_CONTROLS, next)) {
                notice = "The setting would not take that list."
            }
        }
    }

    Column(modifier) {
        SectionHeader("On-screen RC controls")
        notice?.let { message ->
            Text(
                text = message,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(16.dp),
            )
        }

        if (controls.isEmpty()) {
            Text(
                "No on-screen controls yet. Add one to drive a channel from the Fly view.",
                Modifier.padding(16.dp),
            )
        }

        controls.forEachIndexed { index, control ->
            val clash = channelOwner(json, control.channel, index, reserved)
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(control.label, style = MaterialTheme.typography.bodyLarge)
                    Text(
                        "Channel ${control.channel} · ${typeLabel(control.type)}",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    clash?.let {
                        Text(
                            "Channel ${control.channel} is already driving $it.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
                TextButton(onClick = {
                    draft = Draft(index, control.label, control.channel.toString(), control.type)
                }) { Text("Edit") }
                TextButton(onClick = {
                    undo = control.label to json
                    save(rcControlsRemoved(json, index))
                }) { Text("Remove") }
            }
            HorizontalDivider()
        }

        undo?.let { (name, previous) ->
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Removed $name.", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                TextButton(onClick = {
                    save(previous)
                    undo = null
                }) { Text("Undo") }
            }
        }

        Button(
            onClick = {
                draft = Draft(-1, "", firstFreeChannel(json, reserved).toString(), RcControlType.Slider)
            },
            modifier = Modifier.padding(16.dp),
        ) { Text("Add control") }
    }

    draft?.let { current ->
        val channel = current.channel.toIntOrNull() ?: 0
        val owner = channelOwner(json, channel, current.index, reserved)
        AlertDialog(
            onDismissRequest = { draft = null },
            title = { Text(if (current.index < 0) "New control" else "Edit control") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = current.label,
                        onValueChange = { draft = current.copy(label = it) },
                        label = { Text("Name") },
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = current.channel,
                        onValueChange = { draft = current.copy(channel = it.filter(Char::isDigit).take(2)) },
                        label = { Text("Channel") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                    if (!channelUsable(channel)) {
                        Text(
                            "Channels run from $RC_CHANNEL_MIN to $RC_CHANNEL_MAX.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                    owner?.let {
                        Text(
                            "Channel $channel is already driving $it.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        RcControlType.entries.forEach { type ->
                            FilterChip(
                                selected = type == current.type,
                                onClick = { draft = current.copy(type = type) },
                                label = { Text(typeLabel(type)) },
                            )
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = channelUsable(channel) && owner == null,
                    onClick = {
                        val label = current.label.ifBlank { "CH$channel" }
                        save(
                            if (current.index < 0) {
                                rcControlsAdded(json, label, channel, current.type)
                            } else {
                                rcControlsPatched(json, current.index, label, channel, current.type)
                            },
                        )
                        draft = null
                    },
                ) { Text("Save") }
            },
            dismissButton = { TextButton(onClick = { draft = null }) { Text("Cancel") } },
        )
    }
}
