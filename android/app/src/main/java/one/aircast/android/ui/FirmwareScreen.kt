package one.aircast.android.ui

import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject
import java.io.File

internal const val FIRMWARE_VIEW = "view.firmwareUpgrade"
internal const val FIRMWARE_PORTS_VIEW = "view.firmwarePorts"
internal const val FIRMWARE_FLASH = "firmware.flash"
internal const val FIRMWARE_POLL_MS = 500L
internal val FIRMWARE_EXTENSIONS = listOf("px4", "apj", "bin", "ihx")

internal data class FirmwarePort(val port: String, val description: String, val bootloader: Boolean)

internal data class FirmwareJob(
    val phase: String,
    val busy: Boolean,
    val progress: Float,
    val messages: List<String>,
    val error: String,
)

internal fun firmwarePorts(view: JSONObject?): List<FirmwarePort> {
    val listed = view?.optJSONArray("ports") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            entry.optText("port").takeIf { it.isNotBlank() }?.let { port ->
                FirmwarePort(port, entry.optText("description"), entry.optBoolean("bootloader"))
            }
        }
    }
}

internal fun firmwareJob(view: JSONObject?): FirmwareJob? = view?.takeIf { it.optText("class") == "FirmwareUpgrade" }?.let {
    val messages = it.optJSONArray("messages")
    FirmwareJob(
        phase = it.optText("phase").ifBlank { "idle" },
        busy = it.optBoolean("busy"),
        progress = it.optDouble("progress", 0.0).toFloat(),
        messages = (0 until (messages?.length() ?: 0)).map { index -> messages!!.optString(index) },
        error = it.optText("error"),
    )
}

internal fun firmwarePhaseText(phase: String): String = when (phase) {
    "connecting" -> "Waiting for the bootloader"
    "erasing" -> "Erasing"
    "programming" -> "Programming"
    "verifying" -> "Verifying"
    "complete" -> "Upgrade complete"
    "failed" -> "Upgrade failed"
    else -> ""
}

internal const val FIRMWARE_FROM_FILE = "file"

internal val FIRMWARE_SOURCES: List<Pair<String, String>> =
    listOf(FIRMWARE_FROM_FILE to "A firmware file", "px4:stable" to "PX4 Pro, stable", "px4:beta" to "PX4 Pro, beta", "sik:stable" to "SiK radio, stable") +
        listOf("copter", "heli", "plane", "rover", "sub").flatMap { vehicle ->
            listOf("stable", "beta", "dev").map { build ->
                "ardupilot:$vehicle:$build" to "ArduPilot ${vehicle.replaceFirstChar { it.uppercase() }}, $build"
            }
        }

internal fun firmwareChoice(source: String, file: String?): String? =
    if (source == FIRMWARE_FROM_FILE) file else source

internal fun firmwareFileAccepted(name: String): Boolean = FIRMWARE_EXTENSIONS.any { name.lowercase().endsWith(".$it") }

@Composable
fun FirmwareScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var job by remember { mutableStateOf<FirmwareJob?>(null) }
    var ports by remember { mutableStateOf<List<FirmwarePort>>(emptyList()) }
    var port by remember { mutableStateOf("") }
    var file by remember { mutableStateOf<File?>(null) }
    var source by remember { mutableStateOf(FIRMWARE_FROM_FILE) }
    var sourceMenu by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        while (true) {
            val (readJob, readPorts) = withContext(Dispatchers.Default) {
                firmwareJob(Qgc.get(FIRMWARE_VIEW)) to firmwarePorts(Qgc.get(FIRMWARE_PORTS_VIEW))
            }
            job = readJob
            ports = readPorts
            if (port.isBlank()) readPorts.firstOrNull { it.bootloader }?.let { port = it.port }
            delay(FIRMWARE_POLL_MS)
        }
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = withContext(Dispatchers.IO) {
                val name = context.contentResolver.query(chosen, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
                    if (cursor.moveToFirst()) cursor.getString(0) else null
                }.orEmpty()
                if (!firmwareFileAccepted(name)) return@withContext null
                val target = File(context.cacheDir, "firmware-$name")
                runCatching {
                    context.contentResolver.openInputStream(chosen)?.use { source -> target.outputStream().use { source.copyTo(it) } }
                }.getOrNull()?.let { target }
            }
            refusal = if (staged == null) "Choose a .px4, .apj, .bin or .ihx firmware file." else ""
            file = staged ?: file
        }
    }

    val busy = job?.busy == true
    LazyColumn(modifier.fillMaxSize()) {
        item(key = "note") {
            FootNote(
                "Plug in your device via USB, choose its port and either a release, downloaded once the board is " +
                    "identified, or a firmware file, then press Flash. A board " +
                    "running its firmware is asked to be unplugged and plugged back in so its bootloader starts.",
            )
        }
        if (ports.isEmpty()) {
            item(key = "noPorts") { ListItem(headlineContent = { Text("No USB serial devices attached") }) }
        }
        items(ports.size, key = { ports[it].port }) { index ->
            val entry = ports[index]
            ListItem(
                headlineContent = { Text(entry.description.ifBlank { entry.port }) },
                supportingContent = { Text(if (entry.bootloader) "In its bootloader" else entry.port) },
                leadingContent = { RadioButton(selected = entry.port == port, onClick = null, enabled = !busy) },
                modifier = Modifier.selectable(selected = entry.port == port, enabled = !busy) { port = entry.port },
            )
        }
        item(key = "actions") {
            Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Box {
                    OutlinedButton(enabled = !busy, onClick = { sourceMenu = true }) {
                        Text(FIRMWARE_SOURCES.firstOrNull { it.first == source }?.second ?: source)
                    }
                    DropdownMenu(expanded = sourceMenu, onDismissRequest = { sourceMenu = false }) {
                        FIRMWARE_SOURCES.forEach { (token, title) ->
                            DropdownMenuItem(text = { Text(title) }, onClick = {
                                source = token
                                sourceMenu = false
                            })
                        }
                    }
                }
                if (source == FIRMWARE_FROM_FILE) {
                    Text(file?.name?.removePrefix("firmware-") ?: "No firmware file chosen", style = MaterialTheme.typography.bodyMedium)
                }
                val choice = firmwareChoice(source, file?.absolutePath)
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (source == FIRMWARE_FROM_FILE) {
                        OutlinedButton(enabled = !busy, onClick = { picker.launch(arrayOf("*/*")) }) { Text("Choose file") }
                    }
                    Button(
                        enabled = !busy && port.isNotBlank() && choice != null,
                        onClick = {
                            val chosen = choice ?: return@Button
                            scope.launch {
                                refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(FIRMWARE_FLASH, port, chosen) }.orEmpty()
                            }
                        },
                    ) { Text("Flash") }
                }
                job?.let { current ->
                    if (current.busy) LinearProgressIndicator(progress = { current.progress }, modifier = Modifier.fillMaxWidth())
                    firmwarePhaseText(current.phase).takeIf { it.isNotBlank() }?.let { Text(it) }
                    current.error.takeIf { it.isNotBlank() }?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                }
                refusal.takeIf { it.isNotBlank() }?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                job?.messages?.takeIf { it.isNotEmpty() }?.let {
                    Text(it.joinToString("\n"), fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
}
