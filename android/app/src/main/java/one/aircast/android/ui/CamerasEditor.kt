package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.R
import one.aircast.android.bridge.CameraCommands
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSheet
import one.aircast.map.AircastSpace

private const val CAMERA_UNDO_WINDOW_MS = 6000L
private const val CAMERA_LIST_SETTLE_MS = 2000L
private val ROW_PAD_HORIZONTAL = AircastSpace.s4
private val ROW_PAD_VERTICAL = 10.dp
private val ROW_MIN_HEIGHT = 64.dp
private val ROW_GAP = 14.dp
private val ROW_LINE_GAP = 2.dp
private val SHEET_PAD = AircastSpace.s6
private val SHEET_GAP = AircastSpace.s3

internal data class CameraDraft(
    val stored: Int?,
    val title: String,
    val name: String,
    val source: String,
    val url: String,
    val needsUrl: Boolean = true,
    val choosingKind: Boolean = false,
    val refusal: String? = null,
)

private data class RemovedCamera(val slot: Int, val name: String, val source: String, val url: String, val title: String, val active: Boolean)

private fun undoRemoval(gone: RemovedCamera, storedCount: Int): String? {
    val restored = CameraCommands.add(gone.name, gone.source, gone.url)
        ?: if (gone.slot < storedCount) CameraCommands.move(storedCount, gone.slot) else null
    if (restored == null && gone.active) VideoCommands.setActiveSource(gone.slot)
    return restored
}

internal val NEW_CAMERA_DRAFT = CameraDraft(stored = null, title = "", name = "", source = "", url = "", choosingKind = true)

internal fun editDraft(camera: CameraEntry, kinds: List<CameraKind>): CameraDraft? = camera.stored?.let { stored ->
    CameraDraft(
        stored = stored,
        title = camera.title,
        name = camera.name,
        source = camera.source,
        url = camera.url,
        needsUrl = kinds.none { it.raw == camera.source && !it.needsUrl },
    )
}

internal fun pickedKind(draft: CameraDraft, kind: CameraKind): CameraDraft = draft.copy(
    source = kind.raw,
    needsUrl = kind.needsUrl,
    url = if (kind.needsUrl) draft.url.ifBlank { kind.defaultAddress } else "",
    choosingKind = false,
    refusal = null,
)

internal sealed interface CameraSave {
    data class Add(val name: String, val source: String, val url: String) : CameraSave
    data class Update(val slot: Int, val name: String, val source: String, val url: String) : CameraSave
}

internal fun cameraSave(draft: CameraDraft): CameraSave =
    draft.stored?.let { CameraSave.Update(it, draft.name, draft.source, draft.url) } ?: CameraSave.Add(draft.name, draft.source, draft.url)

private fun written(save: CameraSave): String? = when (save) {
    is CameraSave.Add -> CameraCommands.add(save.name, save.source, save.url)
    is CameraSave.Update -> CameraCommands.update(save.slot, save.name, save.source, save.url)
}

internal fun refusedDraft(current: CameraDraft?, refusal: String?): CameraDraft? = refusal?.let { current?.copy(refusal = it) }

@Composable
fun CamerasEditor(modifier: Modifier = Modifier) {
    val view by qgcPath(CAMERAS_VIEW)
    val reading = remember(view) { camerasReading(view) }
    val cameras = reading?.cameras.orEmpty()
    val kinds = reading?.kinds.orEmpty()
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

    Column(modifier.verticalScroll(rememberScrollState())) {
        Row(Modifier.fillMaxWidth().padding(end = AircastSpace.s1, top = AircastSpace.s2), horizontalArrangement = Arrangement.End) {
            IconButton(onClick = { draft = NEW_CAMERA_DRAFT }, enabled = editable && !busy) {
                Icon(painterResource(R.drawable.ic_add), "Add a video source")
            }
        }
        reading?.takeIf { !it.readable }?.let { ErrorLine(it.reason) }
        notice?.let { ErrorLine(it) }
        if (reading != null && cameras.isEmpty()) {
            FootNote("No video sources yet. Cameras on the drone show up here by themselves. Tap + to add one.")
        }
        cameras.map { camera ->
            CameraRow(camera, onEdit = editDraft(camera, kinds)?.takeIf { editable }?.let { opened -> { draft = opened } })
            HorizontalDivider()
        }
        removed?.let { gone ->
            Row(Modifier.fillMaxWidth().padding(horizontal = ROW_PAD_HORIZONTAL, vertical = AircastSpace.s1), verticalAlignment = Alignment.CenterVertically) {
                Text("Removed ${gone.title}.", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                TextButton(enabled = !busy, onClick = {
                    removed = null
                    change({ undoRemoval(gone, storedCount) }) { notice = it }
                }) { Text("Undo") }
            }
        }
    }

    draft?.let { current ->
        val closed: (String?) -> Unit = { refusal -> draft = refusedDraft(draft, refusal) }
        CameraSheet(
            draft = current,
            kinds = addableKinds(reading, keeping = current.source.takeIf { current.stored != null }),
            busy = busy,
            onChange = { draft = it },
            onDismiss = { draft = null },
            onSave = { change({ written(cameraSave(current)) }, closed) },
            onAddFixed = { kind -> change({ CameraCommands.add(addedName(kind), kind.raw, "") }, closed) },
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
    Text(text, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = ROW_PAD_HORIZONTAL, vertical = AircastSpace.s1))
}

@Composable
private fun CameraRow(camera: CameraEntry, onEdit: (() -> Unit)?) {
    Row(
        Modifier
            .fillMaxWidth()
            .then(if (onEdit != null) Modifier.clickable(onClick = onEdit) else Modifier)
            .heightIn(min = ROW_MIN_HEIGHT)
            .padding(horizontal = ROW_PAD_HORIZONTAL, vertical = ROW_PAD_VERTICAL),
        horizontalArrangement = Arrangement.spacedBy(ROW_GAP),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CameraStatusDot(camera.status)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(ROW_LINE_GAP)) {
            Text(camera.title, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(cameraDetail(camera), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            camera.problem?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        }
        if (camera.active) Text("On screen", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (onEdit != null) Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CameraSheet(
    draft: CameraDraft,
    kinds: List<CameraKind>,
    busy: Boolean,
    onChange: (CameraDraft) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit,
    onAddFixed: (CameraKind) -> Unit,
    onRemove: (() -> Unit)?,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    AircastSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier.verticalScroll(rememberScrollState()).imePadding().padding(start = SHEET_PAD, end = SHEET_PAD, bottom = SHEET_PAD),
            verticalArrangement = Arrangement.spacedBy(SHEET_GAP),
        ) {
            if (draft.choosingKind) {
                KindChooser(
                    title = if (draft.stored == null) "Add video source" else "Type",
                    kinds = kinds,
                    busy = busy,
                    onPick = { kind -> if (draft.stored == null && !kind.needsUrl) onAddFixed(kind) else onChange(pickedKind(draft, kind)) },
                    onCancel = if (draft.stored == null) onDismiss else { { onChange(draft.copy(choosingKind = false)) } },
                )
            } else {
                CameraForm(draft, kinds.firstOrNull { it.raw == draft.source }, busy, onChange, onDismiss, onSave, onRemove)
            }
        }
    }
}

@Composable
private fun KindChooser(title: String, kinds: List<CameraKind>, busy: Boolean, onPick: (CameraKind) -> Unit, onCancel: () -> Unit) {
    Text(title, style = MaterialTheme.typography.titleMedium)
    kinds.groupBy { it.group }.map { (group, members) ->
        Column {
            Text(group, Modifier.padding(top = AircastSpace.s2, bottom = AircastSpace.s1), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            members.map { kind ->
                Column(
                    Modifier.fillMaxWidth().clickable(enabled = !busy, role = Role.Button) { onPick(kind) }.padding(vertical = AircastSpace.s2),
                    verticalArrangement = Arrangement.spacedBy(ROW_LINE_GAP),
                ) {
                    Text(kindTitle(kind), style = MaterialTheme.typography.bodyLarge)
                    kind.description.ifBlank { null }?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                }
            }
        }
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        TextButton(onClick = onCancel) { Text("Cancel") }
    }
}

@Composable
private fun CameraForm(
    draft: CameraDraft,
    kind: CameraKind?,
    busy: Boolean,
    onChange: (CameraDraft) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit,
    onRemove: (() -> Unit)?,
) {
    var naming by remember { mutableStateOf(draft.stored != null) }
    val typeTitle = kind?.let(::kindTitle) ?: kindLabel(draft.source)
    val canSave = !busy && (draft.url.isNotBlank() || !draft.needsUrl)
    val save = { if (canSave) onSave() }
    Text(if (draft.stored == null) typeTitle else draft.title, style = MaterialTheme.typography.titleMedium)
    Row(
        Modifier.fillMaxWidth().clickable(enabled = !busy, role = Role.Button) { onChange(draft.copy(choosingKind = true)) }.padding(vertical = AircastSpace.s2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(ROW_LINE_GAP)) {
            Text("Type", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(typeTitle, style = MaterialTheme.typography.bodyLarge)
        }
        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, "Change type", tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    if (naming) {
        OutlinedTextField(
            value = draft.name,
            onValueChange = { onChange(draft.copy(name = it, refusal = null)) },
            label = { Text("Name") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { save() }),
            modifier = Modifier.fillMaxWidth(),
        )
    } else {
        TextButton(onClick = { naming = true }) { Text("Add a name") }
    }
    if (draft.needsUrl) {
        OutlinedTextField(
            value = draft.url,
            onValueChange = { typed -> onChange(draft.copy(url = typed, refusal = null)) },
            label = { Text("Address") },
            placeholder = { Text(kind?.hint.orEmpty()) },
            singleLine = true,
            isError = draft.refusal != null,
            supportingText = (draft.refusal ?: kind?.description)?.takeIf { it.isNotBlank() }?.let { line -> { Text(line) } },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { save() }),
            modifier = Modifier.fillMaxWidth(),
        )
    } else {
        kind?.description?.ifBlank { null }?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        draft.refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        onRemove?.let { remove -> TextButton(onClick = remove, enabled = !busy) { Text("Remove", color = MaterialTheme.colorScheme.error) } }
        Spacer(Modifier.weight(1f))
        TextButton(onClick = onDismiss) { Text("Cancel") }
        Button(onClick = save, enabled = canSave) { Text("Save") }
    }
}
