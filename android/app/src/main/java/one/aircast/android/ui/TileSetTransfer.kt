package one.aircast.android.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import java.io.File

internal const val OFFLINE_EXPORT = "offlineMaps.export"
internal const val OFFLINE_IMPORT = "offlineMaps.import"
private const val TILESET_EXTENSION = "qgctiledb"
private const val TILESET_MIME = "application/octet-stream"
private const val EXPORT_STAGE = "tile-export.$TILESET_EXTENSION"
private const val IMPORT_STAGE = "tile-import.$TILESET_EXTENSION"

private enum class Transfer(val label: String) { Exporting("Exporting"), Importing("Importing") }

internal fun exportArguments(path: String, chosen: Set<Long>): Array<Any> = (listOf<Any>(path) + chosen.sorted()).toTypedArray()

@Composable
internal fun TileSetTransfer(sets: List<OfflineSet>, onRefusal: (String?) -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var running by remember { mutableStateOf<Transfer?>(null) }
    var choosingSets by remember { mutableStateOf(false) }
    var choosingMode by remember { mutableStateOf(false) }
    var chosen by remember { mutableStateOf(emptySet<Long>()) }
    var replace by remember { mutableStateOf(false) }

    fun finish(refusal: String?) {
        running = null
        onRefusal(refusal)
    }

    val saver = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument(TILESET_MIME)) { uri ->
        val target = uri ?: return@rememberLauncherForActivityResult
        running = Transfer.Exporting
        scope.launch {
            val staged = File(context.cacheDir, EXPORT_STAGE)
            val refusal = withContext(Dispatchers.IO) {
                Qgc.refusalOf(OFFLINE_EXPORT, *exportArguments(staged.absolutePath, chosen))
                    ?: runCatching { context.contentResolver.openOutputStream(target)?.use { out -> staged.inputStream().use { it.copyTo(out) } } }
                        .fold({ if (it == null) "That file could not be written." else null }, { "That file could not be written." })
                        .also { staged.delete() }
            }
            finish(refusal)
        }
    }
    val opener = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val source = uri ?: return@rememberLauncherForActivityResult
        running = Transfer.Importing
        scope.launch {
            val staged = File(context.cacheDir, IMPORT_STAGE)
            val refusal = withContext(Dispatchers.IO) {
                val copied = runCatching { context.contentResolver.openInputStream(source)?.use { input -> staged.outputStream().use { input.copyTo(it) } } != null }.getOrDefault(false)
                (if (copied) Qgc.refusalOf(OFFLINE_IMPORT, staged.absolutePath, replace) else "That file could not be read.").also { staged.delete() }
            }
            finish(refusal)
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        ActionLine("Import Map Tiles", "Import…", running == null) { choosingMode = true }
        ActionLine("Export Map Tiles", "Export…", running == null) {
            chosen = emptySet()
            choosingSets = true
        }
        running?.let {
            Text(it.label)
            LinearProgressIndicator(Modifier.fillMaxWidth())
        }
    }

    if (choosingSets) {
        AlertDialog(
            onDismissRequest = { choosingSets = false },
            title = { Text("Export selected tile sets") },
            text = {
                Column {
                    sets.forEach { set ->
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Checkbox(checked = set.id in chosen, onCheckedChange = { chosen = if (it) chosen + set.id else chosen - set.id })
                            Text(set.name)
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(enabled = chosen.isNotEmpty(), onClick = {
                    choosingSets = false
                    saver.launch("export.$TILESET_EXTENSION")
                }) { Text("Ok") }
            },
            dismissButton = { TextButton(onClick = { choosingSets = false }) { Text("Cancel") } },
        )
    }

    if (choosingMode) {
        AlertDialog(
            onDismissRequest = { choosingMode = false },
            title = { Text("Import TileSets") },
            text = {
                Column {
                    listOf(false to "Append to existing sets", true to "Replace existing sets").forEach { (mode, label) ->
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            RadioButton(selected = replace == mode, onClick = { replace = mode })
                            Text(label)
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    choosingMode = false
                    opener.launch(arrayOf("*/*"))
                }) { Text("Ok") }
            },
            dismissButton = { TextButton(onClick = { choosingMode = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun ActionLine(label: String, button: String, enabled: Boolean, onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        OutlinedButton(enabled = enabled, onClick = onClick) { Text(button) }
    }
}
