package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
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
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.CameraCommands
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath

private const val CAMERA_UNDO_WINDOW_MS = 6000L
private const val CAMERA_LIST_SETTLE_MS = 2000L

private data class CameraDraft(val stored: Int?, val name: String, val source: String, val url: String, val refusal: String? = null)

private data class RemovedCamera(val slot: Int, val name: String, val source: String, val url: String, val title: String, val active: Boolean)

private fun undoRemoval(gone: RemovedCamera, storedCount: Int): String? {
    val restored = CameraCommands.add(gone.name, gone.source, gone.url)
        ?: if (gone.slot < storedCount) CameraCommands.move(storedCount, gone.slot) else null
    if (restored == null && gone.active) VideoCommands.setActiveSource(gone.slot)
    return restored
}

@Composable
fun CamerasEditor(modifier: Modifier = Modifier) {
    val view by qgcPath(CAMERAS_VIEW)
    val reading = remember(view) { camerasReading(view) }
    val cameras = reading?.cameras.orEmpty()
    val storedCount = reading?.stored?.size ?: 0
    val editable = reading?.readable == true
    var draft by remember { mutableStateOf<CameraDraft?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var removed by remember { mutableStateOf<RemovedCamera?>(null) }
    var pending by remember { mutableStateOf(false) }
    var settlingFrom by remember { mutableStateOf<List<Triple<String, String, String>>?>(null) }
    val storedNow = reading?.stored?.map { Triple(it.name, it.source, it.url) }
    val busy = pending || (settlingFrom != null && settlingFrom == storedNow)
    val change: (() -> String?, (String?) -> Unit) -> Unit = { action, after ->
        val before = storedNow
        pending = true
        offMainInOrder {
            val refusal = action()
            after(refusal)
            settlingFrom = before.takeIf { refusal == null }
            pending = false
        }
    }

    LaunchedEffect(settlingFrom) {
        if (settlingFrom != null) {
            delay(CAMERA_LIST_SETTLE_MS)
            settlingFrom = null
        }
    }

    LaunchedEffect(removed) {
        if (removed != null) {
            delay(CAMERA_UNDO_WINDOW_MS)
            removed = null
        }
    }

    Column(modifier) {
        reading?.takeIf { !it.readable }?.let { ErrorLine(it.reason) }
        notice?.let { ErrorLine(it) }
        if (reading != null && cameras.isEmpty()) {
            Text(
                "No cameras yet. Add every camera this ground station should show, then switch between them on the Fly view.",
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                style = MaterialTheme.typography.bodyMedium,
            )
        }
        cameras.map { camera ->
            CameraRow(
                camera = camera,
                canMoveUp = !busy && camera.stored != null && camera.stored > 0,
                canMoveDown = !busy && camera.stored != null && camera.stored < storedCount - 1,
                onEdit = { draft = camera.stored?.let { CameraDraft(it, camera.name, camera.source, camera.url) } }.takeIf { camera.stored != null && editable },
                onShow = { offMainInOrder { VideoCommands.setActiveSource(camera.slot) } }.takeIf { !camera.active && camera.problem == null },
                onMove = { offset -> camera.stored?.let { from -> change({ CameraCommands.move(from, from + offset) }) { notice = it } } },
            )
            HorizontalDivider()
        }
        removed?.let { gone ->
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Removed ${gone.title}.", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                TextButton(enabled = !busy, onClick = {
                    removed = null
                    change({ undoRemoval(gone, storedCount) }) { notice = it }
                }) { Text("Undo") }
            }
        }
        Button(
            onClick = { draft = CameraDraft(null, "", reading?.kinds?.firstOrNull()?.raw.orEmpty(), "") },
            enabled = editable && !busy,
            modifier = Modifier.padding(16.dp),
        ) { Text("Add camera") }
    }

    draft?.let { current ->
        CameraDialog(
            draft = current,
            kinds = reading?.kinds.orEmpty(),
            busy = busy,
            onChange = { draft = it },
            onDismiss = { draft = null },
            onSave = {
                change({
                    current.stored?.let { CameraCommands.update(it, current.name, current.source, current.url) }
                        ?: CameraCommands.add(current.name, current.source, current.url)
                }) { refusal -> draft = refusal?.let { current.copy(refusal = it) } }
            },
            onRemove = current.stored?.let { slot ->
                cameras.firstOrNull { it.stored == slot }?.let { entry ->
                    {
                        change({ CameraCommands.remove(slot) }) { refusal ->
                            notice = refusal
                            if (refusal == null) removed = RemovedCamera(slot, entry.name, entry.source, entry.url, entry.title, entry.active)
                            draft = null
                        }
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
        onEdit?.let { edit -> IconButton(onClick = edit) { Icon(Icons.Default.Edit, "Edit ${camera.title}") } }
    }
}

@Composable
private fun CameraDialog(
    draft: CameraDraft,
    kinds: List<CameraKind>,
    busy: Boolean,
    onChange: (CameraDraft) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit,
    onRemove: (() -> Unit)?,
) {
    val save = { if (!busy) onSave() }
    val kind = kinds.firstOrNull { it.raw == draft.source }
    val needsUrl = kind?.needsUrl == true
    var showMore by remember { mutableStateOf(kind?.more == true) }
    val (more, common) = kinds.groupBy { it.group }.toList().partition { (_, members) -> members.all { it.more } }
    val group: @Composable (String, List<CameraKind>) -> Unit = { name, members ->
        KindGroup(name, members, draft.source) { onChange(draft.copy(source = it, refusal = null)) }
        if (needsUrl && members.any { it.raw == draft.source }) {
            OutlinedTextField(
                value = draft.url,
                onValueChange = { typed -> onChange(draft.copy(url = typed, source = inferredKind(kinds, draft.source, typed), refusal = null)) },
                label = { Text("Address") },
                placeholder = { Text(kind?.hint.orEmpty()) },
                singleLine = true,
                isError = draft.refusal != null,
                supportingText = draft.refusal?.let { refusal -> { Text(refusal) } },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false, imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { save() }),
            )
        }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (draft.stored == null) "New camera" else "Edit camera") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (!needsUrl) draft.refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
                OutlinedTextField(
                    value = draft.name,
                    onValueChange = { onChange(draft.copy(name = it, refusal = null)) },
                    label = { Text("Name") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words, imeAction = if (needsUrl) ImeAction.Next else ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { save() }),
                )
                common.map { (name, members) -> group(name, members) }
                if (more.isNotEmpty() && !showMore) TextButton(onClick = { showMore = true }) { Text("More types") }
                if (showMore) more.map { (name, members) -> group(name, members) }
            }
        },
        confirmButton = { TextButton(onClick = save, enabled = !busy) { Text("Save") } },
        dismissButton = {
            Row {
                onRemove?.let { remove -> TextButton(onClick = remove, enabled = !busy) { Text("Remove", color = MaterialTheme.colorScheme.error) } }
                TextButton(onClick = onDismiss) { Text("Cancel") }
            }
        },
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun KindGroup(name: String, members: List<CameraKind>, selected: String, onPick: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(name, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            members.map { option ->
                FilterChip(selected = option.raw == selected, onClick = { onPick(option.raw) }, label = { Text(kindLabel(option.label)) })
            }
        }
    }
}
