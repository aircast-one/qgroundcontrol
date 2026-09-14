package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private const val SOURCE_UNDO_WINDOW_MS = 6000L

private data class SourceDraft(val index: Int, val name: String, val source: String, val url: String)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ExtraVideoSourcesEditor(modifier: Modifier = Modifier) {
    val view by qgcPath(VIDEO_VIEW)
    val reading = remember(view) { extraSourcesReading(view) }
    val json = reading?.stored.orEmpty()
    val sources = reading?.sources.orEmpty()
    val editable = reading?.readable != false
    var kinds by remember { mutableStateOf(emptyList<VideoKind>()) }
    var draft by remember { mutableStateOf<SourceDraft?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var undo by remember { mutableStateOf<Pair<String, String>?>(null) }

    LaunchedEffect(undo) {
        if (undo != null) {
            kotlinx.coroutines.delay(SOURCE_UNDO_WINDOW_MS)
            undo = null
        }
    }

    LaunchedEffect(Unit) {
        kinds = withContext(Dispatchers.Default) {
            val fact = Qgc.factAt(VIDEO_SOURCE_FACT, Qgc.get(VIDEO_SOURCE_FACT))
            videoKinds(fact.enumValues, fact.enumStrings)
        }
    }

    fun save(next: String) {
        if (!editable) {
            notice = reading?.reason
            return
        }
        notice = null
        offMainDetached {
            if (!Qgc.set(EXTRA_SOURCES_FACT, next)) {
                notice = "The setting would not take that list."
            }
        }
    }

    Column(modifier) {
        SectionHeader("Extra cameras")
        if (!editable) {
            Text(
                reading?.reason.orEmpty(),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }
        notice?.let { message ->
            Text(
                message,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(16.dp),
            )
        }

        if (sources.isEmpty()) {
            Text(
                "Add another camera to switch between them on the Fly view.",
                Modifier.padding(16.dp),
            )
        }

        sources.forEachIndexed { index, entry ->
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(
                        entry.name.ifBlank { "Camera ${index + 2}" },
                        style = MaterialTheme.typography.bodyLarge,
                    )
                    Text(extraSourceSummary(entry), style = MaterialTheme.typography.bodySmall)
                }
                TextButton(onClick = {
                    draft = SourceDraft(index, entry.name, entry.source, entry.url)
                }) { Text("Edit") }
                TextButton(onClick = {
                    undo = entry.name.ifBlank { "Camera ${index + 2}" } to json
                    save(extraSourceRemoved(json, index))
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
            onClick = { draft = SourceDraft(-1, "", kinds.firstOrNull()?.raw.orEmpty(), "") },
            modifier = Modifier.padding(16.dp),
        ) { Text("Add camera") }
    }

    draft?.let { current ->
        val problem = extraSourceProblem(current.source, current.url)
        AlertDialog(
            onDismissRequest = { draft = null },
            title = { Text(if (current.index < 0) "New camera" else "Edit camera") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = current.name,
                        onValueChange = { draft = current.copy(name = it) },
                        label = { Text("Name") },
                        singleLine = true,
                    )
                    FlowRow(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        kinds.forEach { kind ->
                            FilterChip(
                                selected = kind.raw == current.source,
                                onClick = { draft = current.copy(source = kind.raw) },
                                label = {
                                    Text(kind.label.removeSuffix(" Video Stream").ifBlank { kind.label })
                                },
                            )
                        }
                    }
                    if (sourceNeedsUrl(current.source)) {
                        OutlinedTextField(
                            value = current.url,
                            onValueChange = { draft = current.copy(url = it) },
                            label = { Text("Address, without the scheme") },
                            singleLine = true,
                        )
                    }
                    problem?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = problem == null,
                    onClick = {
                        val name = current.name.ifBlank { "Camera ${maxOf(current.index, sources.size) + 2}" }
                        save(
                            if (current.index < 0) {
                                extraSourceAdded(json, name, current.source, current.url)
                            } else {
                                extraSourcePatched(json, current.index, name, current.source, current.url)
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
