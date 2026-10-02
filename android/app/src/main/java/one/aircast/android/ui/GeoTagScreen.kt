package one.aircast.android.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.settingControl
import java.io.File

private const val GEOTAG_POLL_MS = 250L
private const val DEFAULT_OUTPUT = "TAGGED"

@Composable
fun GeoTagScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val logSavePath by qgcString(settingControl("settings.appSettings.logSavePath"))
    var state by remember { mutableStateOf<GeoTagState?>(null) }
    var imageTree by remember { mutableStateOf<Uri?>(null) }
    var outputTree by remember { mutableStateOf<Uri?>(null) }
    var note by remember { mutableStateOf<String?>(null) }
    var offsetText by remember { mutableStateOf<String?>(null) }
    var stagedOutput by remember { mutableStateOf<File?>(null) }

    suspend fun refresh() {
        state = withContext(Dispatchers.Default) { geoTagState(Qgc.get(GEOTAG_ROOT)) }
    }

    fun write(path: String, value: Any?) {
        scope.launch {
            withContext(Dispatchers.Default) { Qgc.set("$GEOTAG_ROOT.$path", value) }
            refresh()
        }
    }

    LaunchedEffect(Unit) { refresh() }
    LaunchedEffect(state?.inProgress) {
        val running = state?.inProgress == true
        if (running) {
            while (state?.inProgress == true) {
                delay(GEOTAG_POLL_MS)
                refresh()
            }
            val finished = state
            val target = outputTree ?: imageTree
            val staged = stagedOutput
            if (finished != null && !finished.previewMode && finished.tagged > 0 && target != null && staged != null) {
                val published = withContext(Dispatchers.IO) { publishTagged(context, staged, target, DEFAULT_OUTPUT.takeIf { outputTree == null }) }
                note = if (published == finished.tagged - finished.failed) null else "Only $published of the tagged images could be written to the chosen folder."
            }
        }
    }

    val logPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = withContext(Dispatchers.IO) { stageLog(context, chosen, documentName(context, chosen)) }
            if (staged == null) note = "That file could not be read." else write("logFile", staged)
        }
    }
    val imagePicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        imageTree = chosen
        scope.launch {
            val (path, count) = withContext(Dispatchers.IO) { stageImages(context, chosen) }
            note = if (count == 0) "That folder holds no JPEG, TIFF or DNG images." else null
            write("imageDirectory", path)
        }
    }
    val outputPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        outputTree = uri ?: return@rememberLauncherForActivityResult
    }

    val current = state ?: return
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            "Tag images from a survey mission with GPS coordinates from your flight log.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (current.inProgress) {
            Text("Geotagging in progress...", style = MaterialTheme.typography.bodyMedium)
            LinearProgressIndicator(progress = { (current.progress / 100.0).toFloat() }, modifier = Modifier.fillMaxWidth())
        }
        listOfNotNull(current.errorMessage.ifBlank { null }, note).forEach { message ->
            Text(message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
        }
        geoTagSummary(current)?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary) }

        GeoTagStepRow(
            mark = geoTagStep(current.logFile.isNotBlank(), 1),
            title = "Select Flight Log",
            detail = current.logFile.ifBlank { null }?.substringAfterLast('/') ?: "No file selected",
            enabled = !current.inProgress,
        ) { logPicker.launch(arrayOf("*/*")) }
        downloadedLogs(logSavePath).takeIf { it.isNotEmpty() && !current.inProgress }?.let { logs ->
            Column(Modifier.padding(start = 40.dp)) {
                logs.forEach { log ->
                    TextButton(onClick = { write("logFile", log.absolutePath) }) { Text(log.name) }
                }
            }
        }
        GeoTagStepRow(
            mark = geoTagStep(current.imageDirectory.isNotBlank(), 2),
            title = "Select Image Folder",
            detail = imageTree?.let(::treeName) ?: "No folder selected",
            enabled = !current.inProgress,
        ) { imagePicker.launch(null) }
        GeoTagStepRow(
            mark = "3",
            title = "Output Folder (Optional)",
            detail = outputTree?.let(::treeName) ?: "Default: /$DEFAULT_OUTPUT subfolder",
            enabled = !current.inProgress,
        ) { outputPicker.launch(null) }

        Text("Advanced Options", style = MaterialTheme.typography.titleSmall)
        OutlinedTextField(
            value = offsetText ?: "%.1f".format(current.timeOffsetSecs),
            onValueChange = { typed ->
                offsetText = typed
                typed.toDoubleOrNull()?.let { write("timeOffsetSecs", it) }
            },
            label = { Text("Time Offset (seconds):") },
            supportingText = { Text("Adjust if camera clock differs from flight log") },
            singleLine = true,
            enabled = !current.inProgress,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Checkbox(checked = current.previewMode, enabled = !current.inProgress, onCheckedChange = { write("previewMode", it) })
            Column {
                Text("Preview mode (don't write files)", style = MaterialTheme.typography.bodyMedium)
                Text("Verify time offset before committing", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Button(
            enabled = current.inProgress || (current.logFile.isNotBlank() && current.imageDirectory.isNotBlank()),
            onClick = {
                scope.launch {
                    withContext(Dispatchers.Default) {
                        if (current.inProgress) {
                            Qgc.invoke("$GEOTAG_ROOT.cancelTagging")
                        } else {
                            val output = withContext(Dispatchers.IO) { taggedOutputDir(context) }
                            stagedOutput = output
                            Qgc.set("$GEOTAG_ROOT.saveDirectory", output.absolutePath)
                            Qgc.invoke("$GEOTAG_ROOT.startTagging")
                        }
                    }
                    note = null
                    refresh()
                }
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text(geoTagButton(current)) }
    }
}

@Composable
private fun GeoTagStepRow(mark: String, title: String, detail: String, enabled: Boolean, onBrowse: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(mark, style = MaterialTheme.typography.titleMedium)
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        OutlinedButton(enabled = enabled, onClick = onBrowse) { Text("Browse...") }
    }
}
