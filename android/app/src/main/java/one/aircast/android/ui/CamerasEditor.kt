package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.CameraCommands
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private const val CAMERA_UNDO_WINDOW_MS = 6000L

private data class CameraDraft(val stored: Int?, val name: String, val source: String, val url: String, val refusal: String? = null)

private data class RemovedCamera(val slot: Int, val name: String, val source: String, val url: String, val title: String)

@Composable
fun CamerasEditor(modifier: Modifier = Modifier) {
    val view by qgcPath(CAMERAS_VIEW)
    val reading = remember(view) { camerasReading(view) }
    val cameras = reading?.cameras.orEmpty()
    val storedCount = reading?.stored?.size ?: 0
    val editable = reading?.readable != false
    var draft by remember { mutableStateOf<CameraDraft?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var removed by remember { mutableStateOf<RemovedCamera?>(null) }

    LaunchedEffect(removed) {
        if (removed != null) {
            kotlinx.coroutines.delay(CAMERA_UNDO_WINDOW_MS)
            removed = null
        }
    }

    Column(modifier) {
        if (!editable) ErrorLine(reading?.reason.orEmpty())
        notice?.let { ErrorLine(it) }
        if (cameras.isEmpty()) {
            Text(
                "No cameras yet. Add every camera this ground station should show, then switch between them on the Fly view.",
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                style = MaterialTheme.typography.bodyMedium,
            )
        }
        cameras.map { camera ->
            CameraRow(
                camera = camera,
                canMoveUp = camera.stored != null && camera.stored > 0,
                canMoveDown = camera.stored != null && camera.stored < storedCount - 1,
                onEdit = { draft = camera.stored?.let { CameraDraft(it, camera.name, camera.source, camera.url) } }.takeIf { camera.stored != null && editable },
                onShow = { offMainDetached { VideoCommands.setActiveSource(camera.slot) } }.takeIf { !camera.active && camera.problem == null },
                onMove = { offset -> camera.stored?.let { from -> offMainDetached { notice = CameraCommands.move(from, from + offset) } } },
            )
            HorizontalDivider()
        }
        removed?.let { gone ->
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Removed ${gone.title}.", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                TextButton(onClick = {
                    removed = null
                    offMainDetached {
                        notice = CameraCommands.add(gone.name, gone.source, gone.url)
                            ?: CameraCommands.move(storedCount, gone.slot).takeIf { gone.slot < storedCount }
                    }
                }) { Text("Undo") }
            }
        }
        Button(
            onClick = { draft = CameraDraft(null, "", reading?.kinds?.firstOrNull()?.raw.orEmpty(), "") },
            enabled = editable,
            modifier = Modifier.padding(16.dp),
        ) { Text("Add camera") }
    }

    draft?.let { current ->
        CameraDialog(
            draft = current,
            kinds = reading?.kinds.orEmpty(),
            onChange = { draft = it },
            onDismiss = { draft = null },
            onSave = {
                offMainDetached {
                    val refusal = current.stored?.let { CameraCommands.update(it, current.name, current.source, current.url) }
                        ?: CameraCommands.add(current.name, current.source, current.url)
                    draft = refusal?.let { current.copy(refusal = it) }
                }
            },
            onRemove = current.stored?.let { slot ->
                {
                    val title = cameras.firstOrNull { it.stored == slot }?.title.orEmpty()
                    offMainDetached {
                        val refusal = CameraCommands.remove(slot)
                        notice = refusal
                        if (refusal == null) removed = RemovedCamera(slot, current.name, current.source, current.url, title)
                        draft = null
                    }
                }
            },
        )
    }
}

@Composable
private fun ErrorLine(text: String) {
    Text(text, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
}

@Composable
private fun CameraRow(
    camera: CameraEntry,
    canMoveUp: Boolean,
    canMoveDown: Boolean,
    onEdit: (() -> Unit)?,
    onShow: (() -> Unit)?,
    onMove: (Int) -> Unit,
) {
    Row(
        Modifier.fillMaxWidth().then(if (onEdit != null) Modifier.clickable(onClick = onEdit) else Modifier).padding(start = 16.dp, end = 4.dp, top = 8.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(camera.title, style = MaterialTheme.typography.bodyLarge)
            Text(camera.summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            camera.problem?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                if (camera.active) Text("On screen", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                if (camera.fromDrone) Text("From the drone", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                onShow?.let { show -> TextButton(onClick = show) { Text("Show") } }
            }
        }
        if (camera.stored != null) {
            IconButton(onClick = { onMove(-1) }, enabled = canMoveUp) { Icon(Icons.Default.KeyboardArrowUp, "Move ${camera.title} up") }
            IconButton(onClick = { onMove(1) }, enabled = canMoveDown) { Icon(Icons.Default.KeyboardArrowDown, "Move ${camera.title} down") }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CameraDialog(
    draft: CameraDraft,
    kinds: List<CameraKind>,
    onChange: (CameraDraft) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit,
    onRemove: (() -> Unit)?,
) {
    val kind = kinds.firstOrNull { it.raw == draft.source }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (draft.stored == null) "New camera" else "Edit camera") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = draft.name,
                    onValueChange = { onChange(draft.copy(name = it, refusal = null)) },
                    label = { Text("Name") },
                    singleLine = true,
                )
                kinds.groupBy { it.group }.map { (group, members) ->
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(group, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            members.map { option ->
                                FilterChip(
                                    selected = option.raw == draft.source,
                                    onClick = { onChange(draft.copy(source = option.raw, refusal = null)) },
                                    label = { Text(kindLabel(option.label)) },
                                )
                            }
                        }
                    }
                }
                if (kind?.needsUrl == true) {
                    OutlinedTextField(
                        value = draft.url,
                        onValueChange = { onChange(draft.copy(url = it, refusal = null)) },
                        label = { Text("Address") },
                        placeholder = { Text(kind.hint) },
                        singleLine = true,
                    )
                }
                draft.refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = onSave) { Text("Save") } },
        dismissButton = {
            Row {
                onRemove?.let { remove -> TextButton(onClick = remove) { Text("Remove", color = MaterialTheme.colorScheme.error) } }
                TextButton(onClick = onDismiss) { Text("Cancel") }
            }
        },
    )
}
