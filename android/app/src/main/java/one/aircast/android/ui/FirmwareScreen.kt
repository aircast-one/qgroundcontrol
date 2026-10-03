package one.aircast.android.ui

import one.aircast.mapspike.aircast
import androidx.compose.material3.Surface
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
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
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.mapspike.optText
import org.json.JSONObject
import java.io.File

internal const val FIRMWARE_VIEW = "view.firmwareUpgrade"
internal const val FIRMWARE_PORTS_VIEW = "view.firmwarePorts"
internal const val FIRMWARE_FLASH = "firmware.flash"
internal const val FIRMWARE_CANCEL = "firmware.cancel"
internal const val FIRMWARE_CHOOSE = "firmware.choose"
internal const val FIRMWARE_UPGRADE_SETTINGS = "settings.firmwareUpgradeSettings"
internal const val APM_CHIBIOS = "apmChibiOS"
internal const val FIRMWARE_POLL_MS = 500L
internal val FIRMWARE_EXTENSIONS = listOf("px4", "apj", "bin", "ihx")

internal data class FirmwarePort(val port: String, val description: String, val bootloader: Boolean, val boardType: String = "")

internal const val MULTIPLE_DEVICES = "Multiple devices detected. Make sure to select the correct one from the list."

internal fun preselectedPort(ports: List<FirmwarePort>, current: String): String? =
    ports.firstOrNull { it.port == current && it.boardType.isNotBlank() }?.port
        ?: (ports.firstOrNull { it.boardType == "Pixhawk" } ?: ports.firstOrNull { it.boardType == "SiK Radio" })?.port

internal data class FirmwareJob(
    val phase: String,
    val busy: Boolean,
    val cancellable: Boolean,
    val progress: Float,
    val messages: List<String>,
    val error: String,
    val choices: List<Pair<String, String>> = emptyList(),
    val bestChoice: Int = -1,
    val updateAvailable: String = "",
    val px4StableVersion: String = "",
    val px4BetaVersion: String = "",
)

internal fun firmwarePorts(view: JSONObject?): List<FirmwarePort> {
    val listed = view?.optJSONArray("ports") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            entry.optText("port").takeIf { it.isNotBlank() }?.let { port ->
                FirmwarePort(port, entry.optText("description"), entry.optBoolean("bootloader"), entry.optText("boardType"))
            }
        }
    }
}

internal fun firmwareJob(view: JSONObject?): FirmwareJob? = view?.takeIf { it.optText("class") == "FirmwareUpgrade" }?.let {
    val messages = it.optJSONArray("messages")
    FirmwareJob(
        phase = it.optText("phase").ifBlank { "idle" },
        busy = it.optBoolean("busy"),
        cancellable = it.optBoolean("cancellable"),
        progress = it.optDouble("progress", 0.0).toFloat(),
        messages = (0 until (messages?.length() ?: 0)).map { index -> messages!!.optString(index) },
        error = it.optText("error"),
        choices = it.optJSONArray("choices")?.let { listed ->
            (0 until listed.length()).mapNotNull { index -> listed.optJSONObject(index)?.let { c -> c.optText("name") to c.optText("url") } }
        }.orEmpty(),
        updateAvailable = it.optText("updateAvailable"),
        bestChoice = if (it.isNull("bestChoice")) -1 else it.optInt("bestChoice"),
        px4StableVersion = it.optText("px4StableVersion"),
        px4BetaVersion = it.optText("px4BetaVersion"),
    )
}

internal fun firmwarePhaseText(phase: String): String = when (phase) {
    "connecting" -> "Waiting for the bootloader"
    "choosing" -> "Choose the firmware build"
    "erasing" -> "Erasing"
    "programming" -> "Programming"
    "verifying" -> "Verifying"
    "complete" -> "Upgrade complete"
    "failed" -> "Upgrade failed"
    else -> ""
}

internal const val FIRMWARE_FROM_FILE = "file"
internal const val FLASH_FAIL_TEXT = "If upgrade failed, make sure to connect directly to a powered USB port on your computer, not through a USB hub. Also make sure you are only powered via USB not battery."

internal val FIRMWARE_SOURCES: List<Pair<String, String>> =
    listOf(FIRMWARE_FROM_FILE to "A firmware file", "px4:stable" to "PX4 Pro, stable", "px4:beta" to "PX4 Pro, beta", "px4:dev" to "PX4 Pro, dev", "sik:stable" to "SiK radio, stable") +
        listOf("copter", "heli", "plane", "rover", "sub").flatMap { vehicle ->
            listOf("stable", "beta", "dev").map { build ->
                "ardupilot:$vehicle:$build" to "ArduPilot ${vehicle.replaceFirstChar { it.uppercase() }}, $build"
            }
        }

internal const val DEFAULT_FIRMWARE_SOURCE = "px4:stable"
private const val FIRMWARE_TYPE_PX4 = 12
private const val FIRMWARE_TYPE_APM = 3
private val APM_VEHICLES = listOf("copter", "heli", "plane", "rover", "sub")
private const val DEFAULT_FIRMWARE_TYPE = "defaultFirmwareType"
private const val APM_VEHICLE_TYPE = "apmVehicleType"

internal fun rememberedSource(firmwareType: Int?, apmVehicleType: Int?): String =
    if (firmwareType == FIRMWARE_TYPE_APM) "ardupilot:${APM_VEHICLES.getOrElse(apmVehicleType ?: 0) { APM_VEHICLES.first() }}:stable" else DEFAULT_FIRMWARE_SOURCE

internal fun sourceSettings(source: String): List<Pair<String, Int>> {
    val parts = source.split(':')
    return when (parts.first()) {
        "px4" -> listOf(DEFAULT_FIRMWARE_TYPE to FIRMWARE_TYPE_PX4)
        "ardupilot" -> listOf(DEFAULT_FIRMWARE_TYPE to FIRMWARE_TYPE_APM) + listOfNotNull(APM_VEHICLES.indexOf(parts.getOrNull(1)).takeIf { it >= 0 }?.let { APM_VEHICLE_TYPE to it })
        else -> emptyList()
    }
}
internal const val APM_FIRMWARE = "vehicle.apmFirmware"
internal const val FLASH_BOOTLOADER = "vehicle.flashBootloader"

internal fun bootloaderOffered(advanced: Boolean, apmVehicle: Boolean): Boolean = advanced && apmVehicle

internal fun firmwareSources(advanced: Boolean): List<Pair<String, String>> =
    if (advanced) FIRMWARE_SOURCES else FIRMWARE_SOURCES.filter { it.first.endsWith(":stable") }

internal fun sourceAfterAdvanced(source: String, advanced: Boolean): String =
    source.takeIf { advanced || it.endsWith(":stable") } ?: source.substringBeforeLast(':', "").takeIf { it.isNotEmpty() }?.let { "$it:stable" } ?: DEFAULT_FIRMWARE_SOURCE

internal const val BETA_WARNING = "WARNING: BETA FIRMWARE. This firmware version is ONLY intended for beta testers. Although it has received FLIGHT TESTING, it represents actively changed code. Do NOT use for normal operation."
internal const val DEV_WARNING = "WARNING: CONTINUOUS BUILD FIRMWARE. This firmware has NOT BEEN FLIGHT TESTED. It is only intended for DEVELOPERS. Run bench tests without props first. Do NOT fly this without additional safety precautions. Follow the forums actively when using it."

internal fun firmwareWarning(source: String): String? = when {
    source.endsWith(":beta") -> BETA_WARNING
    source.endsWith(":dev") -> DEV_WARNING
    else -> null
}

internal fun flashingLabel(ports: List<FirmwarePort>, port: String): String =
    "Flashing - ${ports.firstOrNull { it.port == port }?.description?.ifBlank { null } ?: port}"

internal fun sourceLabel(source: String, label: String, stable: String, beta: String): String = when {
    source == "px4:stable" && stable.isNotBlank() -> "PX4 Pro $stable"
    source == "px4:beta" && beta.isNotBlank() -> "PX4 Pro $beta"
    else -> label
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
    var advanced by remember { mutableStateOf(false) }
    val apmVehicle by qgcBool(APM_FIRMWARE)
    val upgradeSettings by one.aircast.android.bridge.qgcFacts(FIRMWARE_UPGRADE_SETTINGS)
    var source by remember { mutableStateOf(DEFAULT_FIRMWARE_SOURCE) }
    var sourceRestored by remember { mutableStateOf(false) }
    LaunchedEffect(upgradeSettings) {
        if (sourceRestored || upgradeSettings.isEmpty()) return@LaunchedEffect
        val setting = { name: String -> (upgradeSettings.firstOrNull { it.name == name }?.value as? Number)?.toInt() }
        source = rememberedSource(setting(DEFAULT_FIRMWARE_TYPE), setting(APM_VEHICLE_TYPE))
        sourceRestored = true
    }
    val chooseSource: (String) -> Unit = { chosen ->
        source = chosen
        sourceRestored = true
        one.aircast.android.bridge.offMainDetached {
            sourceSettings(chosen).forEach { (name, value) -> Qgc.writeRefusal("$FIRMWARE_UPGRADE_SETTINGS.$name", value) }
        }
    }
    var refusal by remember { mutableStateOf("") }
    var flashingName by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        while (true) {
            val (readJob, readPorts) = withContext(Dispatchers.Default) {
                firmwareJob(Qgc.get(FIRMWARE_VIEW)) to firmwarePorts(Qgc.get(FIRMWARE_PORTS_VIEW))
            }
            job = readJob
            if (readPorts != ports && readJob?.busy != true) port = preselectedPort(readPorts, port) ?: port.takeIf { chosen -> readPorts.any { it.port == chosen } }.orEmpty()
            ports = readPorts
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
            Surface(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), color = MaterialTheme.colorScheme.primaryContainer, shape = MaterialTheme.shapes.medium) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    job?.updateAvailable?.takeIf { it.isNotBlank() }?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning) }
                    Text(if (ports.isEmpty()) "No USB serial devices attached" else "Flash a board over USB", style = MaterialTheme.typography.titleSmall)
                    Text(
                        "Plug in your device via USB, choose its port and either a release, downloaded once the board is " +
                            "identified, or a firmware file, then press Flash. A board " +
                            "running its firmware is asked to be unplugged and plugged back in so its bootloader starts.",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
        }
        if (!busy && ports.count { it.boardType.isNotBlank() } > 1) item(key = "multiple") {
            Text(MULTIPLE_DEVICES, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning, modifier = Modifier.padding(horizontal = 16.dp))
        }
        items(ports.size, key = { ports[it].port }) { index ->
            val entry = ports[index]
            ListItem(
                headlineContent = { Text(entry.description.ifBlank { entry.port }) },
                supportingContent = { Text(if (entry.bootloader) "In its bootloader" else listOf(entry.boardType, entry.port).filter { it.isNotBlank() }.joinToString(" \u00b7 ")) },
                leadingContent = { RadioButton(selected = entry.port == port, onClick = null, enabled = !busy) },
                modifier = Modifier.selectable(selected = entry.port == port, enabled = !busy) { port = entry.port },
            )
        }
        item(key = "actions") {
            Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                val offered = firmwareSources(advanced)
                ChoiceField(
                    "Firmware",
                    FIRMWARE_SOURCES.firstOrNull { it.first == source }?.let { sourceLabel(it.first, it.second, job?.px4StableVersion.orEmpty(), job?.px4BetaVersion.orEmpty()) } ?: source,
                    offered.map { sourceLabel(it.first, it.second, job?.px4StableVersion.orEmpty(), job?.px4BetaVersion.orEmpty()) },
                    Modifier.fillMaxWidth(),
                ) { index -> if (!busy) chooseSource(offered[index].first) }
                if (source.startsWith("ardupilot:")) upgradeSettings.firstOrNull { it.name == APM_CHIBIOS }?.let { FactRow(it, fieldModifier = Modifier.fillMaxWidth()) }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(checked = advanced, enabled = !busy, onCheckedChange = {
                        advanced = it
                        source = sourceAfterAdvanced(source, it)
                    })
                    Text("Advanced settings", style = MaterialTheme.typography.bodyMedium)
                }
                if (bootloaderOffered(advanced, apmVehicle)) {
                    OutlinedButton(enabled = !busy, onClick = { offMainDetached { Qgc.invoke(FLASH_BOOTLOADER) } }) { Text("Flash ChibiOS bootloader") }
                }
                if (source == FIRMWARE_FROM_FILE) {
                    Text(file?.name?.removePrefix("firmware-") ?: "No firmware file chosen", style = MaterialTheme.typography.bodyMedium)
                }
                firmwareWarning(source)?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error) }
                val choice = firmwareChoice(source, file?.absolutePath)
                if (busy) Text(flashingName.ifBlank { flashingLabel(ports, port) }, style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End), verticalAlignment = Alignment.CenterVertically) {
                    if (source == FIRMWARE_FROM_FILE) {
                        OutlinedButton(enabled = !busy, onClick = { picker.launch(arrayOf("*/*")) }) { Text("Choose file") }
                    }
                    Button(
                        enabled = !busy && port.isNotBlank() && choice != null,
                        onClick = {
                            val chosen = choice ?: return@Button
                            flashingName = flashingLabel(ports, port)
                            scope.launch {
                                refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(FIRMWARE_FLASH, port, chosen) }.orEmpty()
                            }
                        },
                    ) { Text("Flash") }
                    if (busy) {
                        OutlinedButton(
                            enabled = job?.cancellable == true,
                            onClick = { scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(FIRMWARE_CANCEL) }.orEmpty() } },
                        ) { Text("Cancel") }
                    }
                }
                job?.let { current ->
                    if (current.busy) LinearProgressIndicator(progress = { current.progress }, modifier = Modifier.fillMaxWidth())
                    firmwarePhaseText(current.phase).takeIf { it.isNotBlank() }?.let { Text(it) }
                    if (current.phase == "choosing") current.choices.forEachIndexed { index, (name, url) ->
                        val choose = { scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(FIRMWARE_CHOOSE, url) }.orEmpty() } }
                        if (index == current.bestChoice) Button(onClick = { choose() }, modifier = Modifier.fillMaxWidth()) { Text(name) }
                        else OutlinedButton(onClick = { choose() }, modifier = Modifier.fillMaxWidth()) { Text(name) }
                    }
                    current.error.takeIf { it.isNotBlank() }?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                    if (current.phase == "failed") Text(FLASH_FAIL_TEXT, style = MaterialTheme.typography.bodyMedium)
                }
                refusal.takeIf { it.isNotBlank() }?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                job?.messages?.takeIf { it.isNotEmpty() }?.let {
                    Text(it.joinToString("\n"), fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
}
