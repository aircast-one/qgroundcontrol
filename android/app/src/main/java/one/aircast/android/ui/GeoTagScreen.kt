package one.aircast.android.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.produceState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.settingControl
import java.io.File

private val GEOTAG_BLUE = Color(0xFF2196F3)
private val GEOTAG_GREEN = Color(0xFF00C853)
private val GEOTAG_ORANGE = Color(0xFFFF9800)

@Composable
fun GeoTagScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val logSavePath by qgcString(settingControl("settings.appSettings.logSavePath"))
    val state by GeoTagRun.state.collectAsState()
    val note by GeoTagRun.note.collectAsState()
    val busy by GeoTagRun.busy.collectAsState()
    val imageTree by GeoTagRun.imageTree.collectAsState()
    val outputTree by GeoTagRun.outputTree.collectAsState()
    var offsetText by remember { mutableStateOf<String?>(null) }
    val logs by produceState(emptyList<File>(), logSavePath) {
        value = withContext(Dispatchers.IO) { downloadedLogs(logSavePath) }
    }

    LaunchedEffect(Unit) { withContext(Dispatchers.Default) { GeoTagRun.refresh() } }

    val logPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri?.let { GeoTagRun.pickLog(context, it) }
    }
    val imagePicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        uri?.let { GeoTagRun.pickImages(context, it) }
    }
    val outputPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        uri?.let { GeoTagRun.pickOutput(context, it) }
    }

    val current = state ?: return
    val editable = !current.inProgress && !busy
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
        listOfNotNull(geoTagError(current), note).forEach { message ->
            Text(message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
        }
        geoTagSummary(current)?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary) }

        GeoTagStepRow(
            mark = geoTagStep(current.logFile.isNotBlank(), 1),
            title = "Select flight log",
            detail = current.logFile.ifBlank { null }?.substringAfterLast('/') ?: "No file selected",
            enabled = editable,
        ) { logPicker.launch(arrayOf("*/*")) }
        logs.takeIf { it.isNotEmpty() && editable }?.let { downloaded ->
            Column(Modifier.padding(start = 40.dp)) {
                downloaded.forEach { log ->
                    TextButton(onClick = { GeoTagRun.pickDownloadedLog(log.absolutePath) }) { Text(log.name) }
                }
            }
        }
        GeoTagStepRow(
            mark = geoTagStep(current.imageDirectory.isNotBlank(), 2),
            title = "Select image folder",
            detail = imageTree?.let(::treeName) ?: "No folder selected",
            enabled = editable,
        ) { imagePicker.launch(null) }
        GeoTagStepRow(
            mark = "3",
            title = "Output folder (optional)",
            detail = outputTree?.let(::treeName) ?: "Default: /$DEFAULT_GEOTAG_OUTPUT subfolder",
            enabled = editable,
        ) { outputPicker.launch(null) }

        Text("Advanced options", style = MaterialTheme.typography.titleSmall)
        OutlinedTextField(
            value = offsetText ?: shownOffset(current.timeOffsetSecs),
            onValueChange = { typed ->
                offsetText = typed
                parsedOffset(typed)?.let { GeoTagRun.set("timeOffsetSecs", it) }
            },
            label = { Text("Time offset (seconds):") },
            supportingText = { Text("Adjust if camera clock differs from flight log") },
            singleLine = true,
            enabled = editable,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Checkbox(checked = current.previewMode, enabled = editable, onCheckedChange = { GeoTagRun.set("previewMode", it) })
            Column {
                Text("Preview mode (don't write files)", style = MaterialTheme.typography.bodyMedium)
                Text("Verify time offset before committing", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        Button(
            enabled = current.inProgress || (!busy && current.logFile.isNotBlank() && current.imageDirectory.isNotBlank()),
            onClick = { if (current.inProgress) GeoTagRun.cancel() else GeoTagRun.start(context) },
            modifier = Modifier.fillMaxWidth(),
        ) { Text(geoTagButton(current)) }
        if (current.images.isNotEmpty()) {
            Text("Images (${current.images.size})", style = MaterialTheme.typography.titleSmall)
            GeoTagLegend()
            current.images.forEach { GeoTagImageRow(it) }
        }
    }
}

@Composable
private fun geoTagStatusColour(status: Int): Color = when (status) {
    1 -> GEOTAG_BLUE
    GEOTAG_TAGGED -> GEOTAG_GREEN
    3 -> GEOTAG_ORANGE
    4 -> MaterialTheme.colorScheme.error
    else -> MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
}

internal val GEOTAG_LEGEND = listOf(0 to "Pending", 1 to "Processing", GEOTAG_TAGGED to "Tagged", 3 to "Skipped", 4 to "Failed")

@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
private fun GeoTagLegend() {
    androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        GEOTAG_LEGEND.forEach { (status, label) ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                Box(Modifier.size(10.dp).background(geoTagStatusColour(status), RoundedCornerShape(2.dp)))
                Text(label, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

@Composable
private fun GeoTagImageRow(image: GeoTagImage) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Box(Modifier.size(12.dp).background(geoTagStatusColour(image.status), RoundedCornerShape(2.dp)))
        Column(Modifier.weight(1f)) {
            Text(image.fileName, style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
            geoTagCoordinate(image)?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
        Text(
            geoTagImageText(image),
            style = MaterialTheme.typography.bodySmall,
            color = if (image.status == 3 || image.status == 4) geoTagStatusColour(image.status) else MaterialTheme.colorScheme.onSurface,
        )
    }
}

@Composable
private fun GeoTagStepRow(mark: String, title: String, detail: String, enabled: Boolean, onBrowse: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        val done = !mark.all { it.isDigit() }
        Box(
            Modifier.size(32.dp).background(if (done) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.secondaryContainer, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(mark, style = MaterialTheme.typography.labelLarge, color = if (done) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSecondaryContainer)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        OutlinedButton(enabled = enabled, onClick = onBrowse) { Text("Browse...") }
    }
}
