package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject
import java.text.NumberFormat

private const val MAVLINK_LOG_VIEW = "view.mavlinkLog"
private const val MAVLINK_LOG_POLL_MS = 1000L
private const val TEXT_SAVE_DELAY_MS = 600L

internal val WIND_SPEEDS = listOf("Not Set" to "-1", "Calm" to "0", "Breeze" to "5", "Gale" to "8", "Storm" to "10")
internal val FLIGHT_RATINGS = listOf(
    "Not Set" to "notset",
    "Crashed (Pilot Error)" to "crash_pilot",
    "Crashed (Software or Hardware Issue)" to "crash_sw_hw",
    "Unsatisfactory" to "unsatisfactory",
    "Good" to "good",
    "Great" to "great",
)

internal data class LogFile(val name: String, val size: Long, val uploaded: Boolean, val writing: Boolean = false)

internal data class MavlinkLog(
    val px4: Boolean,
    val running: Boolean,
    val canStart: Boolean,
    val persistence: Boolean,
    val settings: JSONObject,
    val files: List<LogFile>,
    val uploading: Boolean,
    val uploadingFile: String,
    val message: String,
)

internal fun mavlinkLog(view: JSONObject?): MavlinkLog? = view?.let {
    val listed = it.optJSONArray("files")
    MavlinkLog(
        px4 = it.optBoolean("vehiclePx4"),
        running = it.optBoolean("logRunning"),
        canStart = it.optBoolean("canStartLog"),
        persistence = it.optBoolean("persistence", true),
        settings = it.optJSONObject("settings") ?: JSONObject(),
        files = (0 until (listed?.length() ?: 0)).mapNotNull { index -> listed?.optJSONObject(index) }.map { file ->
            LogFile(file.optText("name"), file.optLong("size"), file.optBoolean("uploaded"), file.optBoolean("writing"))
        },
        uploading = it.optBoolean("uploading"),
        uploadingFile = it.optText("uploadingFile"),
        message = it.optText("message"),
    )
}

internal fun logSizeText(size: Long): String = NumberFormat.getIntegerInstance().format(size)

@Composable
internal fun Px4LogTransferPage(modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    var read by remember { mutableStateOf<MavlinkLog?>(null) }
    var polls by remember { mutableIntStateOf(0) }
    var selected by remember { mutableStateOf(emptySet<String>()) }
    var confirming by remember { mutableStateOf<Pair<String, () -> Unit>?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(polls) {
        while (isActive) {
            read = withContext(Dispatchers.Default) { mavlinkLog(Qgc.get(MAVLINK_LOG_VIEW)) }
            delay(MAVLINK_LOG_POLL_MS)
        }
    }

    fun act(path: String, vararg args: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            polls++
        }
    }

    val log = read ?: return
    val editable = log.persistence
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (!log.px4) Text("Connect a PX4 vehicle to manage logs", style = MaterialTheme.typography.bodyMedium)
        SectionHeader("MAVLink 2.0 Logging")
        FootNote("PX4 Pro only")
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Logging", modifier = Modifier.weight(1f))
            OutlinedButton(enabled = !log.running && log.canStart && editable, onClick = { act("mavlinkLog.start") }) { Text("Start") }
            OutlinedButton(enabled = log.running && editable, onClick = { act("mavlinkLog.stop") }) { Text("Stop") }
        }
        LogFlag("Start logging automatically", log.settings.optBoolean("enableAutoStart"), editable) { act("mavlinkLog.set", "enableAutoStart", it) }

        SectionHeader("Log Upload")
        LogText("Email address", log.settings.optText("emailAddress"), editable) { act("mavlinkLog.set", "emailAddress", it) }
        LogText("Default description", log.settings.optText("description"), editable) { act("mavlinkLog.set", "description", it) }
        LogText("Upload URL", log.settings.optText("uploadURL"), editable) { act("mavlinkLog.set", "uploadURL", it) }
        LogText("Video URL", log.settings.optText("videoURL"), editable) { act("mavlinkLog.set", "videoURL", it) }
        LogChoice("Wind Speed", WIND_SPEEDS, log.settings.optText("windSpeed"), editable) { act("mavlinkLog.set", "windSpeed", it) }
        LogChoice("Flight Rating", FLIGHT_RATINGS, log.settings.optText("rating"), editable) { act("mavlinkLog.set", "rating", it) }
        LogText("Additional feedback", log.settings.optText("feedback"), editable) { act("mavlinkLog.set", "feedback", it) }
        LogFlag("Make logs public", log.settings.optBoolean("publicLog"), editable) { act("mavlinkLog.set", "publicLog", it) }
        LogFlag("Upload logs automatically", log.settings.optBoolean("enableAutoUpload"), editable) { act("mavlinkLog.set", "enableAutoUpload", it) }
        LogFlag("Delete logs after upload", log.settings.optBoolean("deleteAfterUpload"), editable && log.settings.optBoolean("enableAutoUpload")) {
            act("mavlinkLog.set", "deleteAfterUpload", it)
        }

        SectionHeader("Saved Log Files")
        if (log.files.isEmpty()) Text("No log files", style = MaterialTheme.typography.bodySmall)
        log.files.forEach { file ->
            val uploadingThis = log.uploading && log.uploadingFile == file.name
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(
                    checked = file.name in selected,
                    enabled = !uploadingThis && !file.writing,
                    onCheckedChange = { selected = if (it) selected + file.name else selected - file.name },
                )
                Text(file.name, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodySmall)
                Text(
                    when {
                        file.uploaded -> "Uploaded"
                        uploadingThis -> "Uploading"
                        else -> logSizeText(file.size)
                    },
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        val idle = !log.uploading
        val uploadedSelected = log.files.any { it.name in selected && it.uploaded }
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            TextButton(enabled = idle, onClick = { selected = log.files.map { it.name }.toSet() }) { Text("Select all") }
            TextButton(enabled = idle, onClick = { selected = emptySet() }) { Text("Select none") }
            TextButton(enabled = selected.isNotEmpty() && idle, onClick = {
                confirming = "Delete the selected log files?" to {
                    act("mavlinkLog.delete", *selected.toTypedArray())
                    selected = emptySet()
                }
            }) { Text("Delete…") }
            if (log.uploading) {
                TextButton(onClick = { confirming = "Cancel the upload in progress?" to { act("mavlinkLog.cancelUpload") } }) { Text("Cancel upload…") }
            } else {
                TextButton(enabled = selected.isNotEmpty() && !uploadedSelected, onClick = {
                    if (log.settings.optText("emailAddress").isBlank()) {
                        refusal = "Please enter an email address before uploading MAVLink log files."
                    } else {
                        confirming = "Upload the selected log files?" to { act("mavlinkLog.upload", *selected.toTypedArray()) }
                    }
                }) { Text("Upload…") }
            }
        }
        listOfNotNull(refusal, log.message.ifBlank { null }).forEach { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
    }

    confirming?.let { (question, run) ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            text = { Text(question) },
            confirmButton = { TextButton(onClick = { confirming = null; run() }) { Text("Ok") } },
            dismissButton = { TextButton(onClick = { confirming = null }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun LogFlag(label: String, checked: Boolean, enabled: Boolean, onChange: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, enabled = enabled, onCheckedChange = onChange)
        Text(label)
    }
}

@Composable
private fun LogText(label: String, value: String, enabled: Boolean, onSave: (String) -> Unit) {
    var typed by remember(value) { mutableStateOf(value) }
    LaunchedEffect(typed) {
        if (typed != value) {
            delay(TEXT_SAVE_DELAY_MS)
            onSave(typed)
        }
    }
    OutlinedTextField(value = typed, onValueChange = { typed = it }, enabled = enabled, label = { Text(label) }, singleLine = true, modifier = Modifier.fillMaxWidth())
}

@Composable
private fun LogChoice(label: String, choices: List<Pair<String, String>>, value: String, enabled: Boolean, onPick: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(enabled = enabled, onClick = { open = true }) { Text(choices.firstOrNull { it.second == value }?.first ?: choices.first().first) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                choices.forEach { (shown, raw) -> DropdownMenuItem(text = { Text(shown) }, onClick = { open = false; onPick(raw) }) }
            }
        }
    }
}
