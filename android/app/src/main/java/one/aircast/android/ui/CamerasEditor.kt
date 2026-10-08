package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SheetValue
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
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.android.bridge.CameraCommands
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSheet

private const val CAMERA_UNDO_WINDOW_MS = 6000L
private const val CAMERA_LIST_SETTLE_MS = 2000L
private const val CAMERA_CLASSIFY_SETTLE_MS = 300L

private data class CameraDraft(
    val stored: Int?,
    val title: String,
    val name: String,
    val source: String,
    val url: String,
    val needsUrl: Boolean = true,
    val picked: String? = null,
    val kept: String? = null,
    val refusal: String? = null,
)

private data class RemovedCamera(val slot: Int, val name: String, val source: String, val url: String, val title: String, val active: Boolean)

private fun undoRemoval(gone: RemovedCamera, storedCount: Int): String? {
    val restored = CameraCommands.add(gone.name, gone.source, gone.url)
        ?: if (gone.slot < storedCount) CameraCommands.move(storedCount, gone.slot) else null
    if (restored == null && gone.active) VideoCommands.setActiveSource(gone.slot)
    return restored
}

private fun editDraft(camera: CameraEntry, kinds: List<CameraKind>): CameraDraft? = camera.stored?.let { stored ->
    CameraDraft(
        stored = stored,
        title = camera.title,
        name = camera.name,
        source = camera.source,
        url = camera.url,
        needsUrl = kinds.none { it.raw == camera.source && !it.needsUrl },
        picked = camera.source,
        kept = camera.url.takeIf { camera.problem == null },
    )
}

private fun draftGuess(draft: CameraDraft, guess: CameraGuess?): CameraGuess? = keptGuess(guess, draft.source, draft.url, draft.kept)

private fun saved(draft: CameraDraft, shown: CameraGuess?): String? = when {
    draft.stored != null && !draft.needsUrl -> CameraCommands.update(draft.stored, draft.name, draft.source, draft.url)
    else -> savedAs(draft, shown ?: draftGuess(draft, cameraGuess(CameraCommands.classify(draft.url))))
}

private fun savedAs(draft: CameraDraft, guess: CameraGuess?): String? = when (draft.stored) {
    null -> CameraCommands.add(draft.name, chosenKind(guess, draft.picked, ""), draft.url)
    else -> guess?.takeIf { it.kind == null }?.problem
        ?: CameraCommands.update(draft.stored, draft.name, chosenKind(guess, draft.picked, draft.source), draft.url)
}

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
        Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp, top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(VIDEO_SOURCES_PAGE, Modifier.weight(1f), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            IconButton(onClick = { draft = CameraDraft(null, "", "", "", "") }, enabled = editable && !busy) {
                Icon(painterResource(R.drawable.ic_add), "Add a video source")
            }
        }
        reading?.takeIf { !it.readable }?.let { ErrorLine(it.reason) }
        notice?.let { ErrorLine(it) }
        if (reading != null && cameras.isEmpty()) {
            FootNote("No video sources yet. Cameras on the drone show up here by themselves. Tap + to add a stream by its address.")
        }
        cameras.map { camera ->
            CameraRow(camera, onEdit = editDraft(camera, kinds)?.takeIf { editable }?.let { opened -> { draft = opened } })
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
    }

    draft?.let { current ->
        val closed: (String?) -> Unit = { refusal -> draft = refusal?.let { current.copy(refusal = it) } }
        CameraSheet(
            draft = current,
            hint = kinds.firstOrNull { it.needsUrl }?.hint.orEmpty(),
            others = if (current.stored == null) otherSources(reading) else emptyList(),
            busy = busy,
            onChange = { draft = it },
            onDismiss = { draft = null },
            onSave = { guess -> change({ saved(current, guess) }, closed) },
            onPick = { kind -> change({ CameraCommands.add("", kind.raw, "") }, closed) },
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
private fun CameraRow(camera: CameraEntry, onEdit: (() -> Unit)?) {
    Row(
        Modifier
            .fillMaxWidth()
            .then(if (onEdit != null) Modifier.clickable(onClick = onEdit) else Modifier)
            .heightIn(min = 64.dp)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CameraStatusDot(camera.status)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(camera.title, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(cameraDetail(camera), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            camera.problem?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        }
        if (camera.active) Text("On screen", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (onEdit != null) Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun CameraSheet(
    draft: CameraDraft,
    hint: String,
    others: List<CameraKind>,
    busy: Boolean,
    onChange: (CameraDraft) -> Unit,
    onDismiss: () -> Unit,
    onSave: (CameraGuess?) -> Unit,
    onPick: (CameraKind) -> Unit,
    onRemove: (() -> Unit)?,
) {
    var guessed by remember { mutableStateOf<Pair<String, CameraGuess?>?>(null) }
    var naming by remember { mutableStateOf(draft.stored != null) }
    LaunchedEffect(draft.url, draft.needsUrl) {
        if (draft.url.isBlank() || !draft.needsUrl) return@LaunchedEffect
        delay(CAMERA_CLASSIFY_SETTLE_MS)
        val typed = draft.url
        guessed = typed to withContext(Dispatchers.Default) { cameraGuess(CameraCommands.classify(typed)) }
    }
    val guess = draftGuess(draft, guessed?.takeIf { it.first == draft.url }?.second)
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    LaunchedEffect(guess != null, guess?.ambiguous) {
        if (guess != null && sheetState.targetValue != SheetValue.Hidden) sheetState.expand()
    }
    val canSave = !busy && (draft.url.isNotBlank() || !draft.needsUrl)
    val save = { if (canSave) onSave(guess) }
    val nameField: @Composable () -> Unit = {
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
    }
    AircastSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            Modifier.verticalScroll(rememberScrollState()).imePadding().padding(start = 24.dp, end = 24.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(if (draft.stored == null) "Add video source" else draft.title, style = MaterialTheme.typography.titleMedium)
            if (draft.stored != null) nameField()
            if (draft.needsUrl) {
                OutlinedTextField(
                    value = draft.url,
                    onValueChange = { typed -> onChange(draft.copy(url = typed, refusal = null)) },
                    label = { Text("Address") },
                    placeholder = { Text(hint) },
                    singleLine = true,
                    isError = draft.refusal != null,
                    supportingText = (draft.refusal ?: guessText(guess).takeIf { draft.url.isNotBlank() })?.takeIf { it.isNotBlank() }?.let { line -> { Text(line) } },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { save() }),
                    modifier = Modifier.fillMaxWidth(),
                )
                guess?.takeIf { it.ambiguous }?.let { ambiguous ->
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        ambiguous.choices.map { choice ->
                            FilterChip(
                                selected = choice == chosenKind(ambiguous, draft.picked, ""),
                                onClick = { onChange(draft.copy(picked = choice, refusal = null)) },
                                label = { Text(kindLabel(choice)) },
                            )
                        }
                    }
                }
            } else {
                Text(kindLabel(draft.source), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                draft.refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
            }
            if (draft.stored == null) nameField()
            Row(verticalAlignment = Alignment.CenterVertically) {
                onRemove?.let { remove -> TextButton(onClick = remove, enabled = !busy) { Text("Remove", color = MaterialTheme.colorScheme.error) } }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = onDismiss) { Text("Cancel") }
                Button(onClick = save, enabled = canSave) { Text("Save") }
            }
            if (others.isNotEmpty()) {
                Text("Other sources", Modifier.padding(top = 8.dp), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Column {
                    others.map { kind ->
                        Text(
                            otherSourceLabel(kind),
                            Modifier.fillMaxWidth().clickable(enabled = !busy) { onPick(kind) }.padding(vertical = 12.dp),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    }
                }
            }
        }
    }
}
