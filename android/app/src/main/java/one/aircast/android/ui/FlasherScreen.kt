package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal const val FLASHER_VIEW = "view.flasher"
internal const val FLASHER_OPEN = "flasher.open"
internal const val FLASHER_CHANNEL = "flasher.channel"
internal const val FLASHER_SELECT = "flasher.select"
internal const val FLASHER_FORM = "flasher.form"
internal const val FLASHER_DISKS = "flasher.disks"
internal const val FLASHER_CHOOSE_DISK = "flasher.chooseDisk"
internal const val FLASHER_START = "flasher.start"
internal const val FLASHER_CANCEL = "flasher.cancel"
internal const val FLASHER_AGAIN = "flasher.again"
internal const val FLASHER_POLL_MS = 500L
internal const val FLASHER_DISK_POLL_MS = 2_000L

internal data class FlasherRelease(val version: String, val label: String, val published: String, val sizeText: String)

internal data class FlasherDisk(val id: String, val name: String)

internal data class FlasherSummaryRow(val label: String, val value: String, val warn: Boolean)

internal data class FlasherForm(
    val hostname: String = "",
    val ssid: String = "",
    val wifiPassword: String = "",
    val noWifi: Boolean = false,
    val country: String = "",
    val sshMode: String = "key-only",
    val authorizedKey: String = "",
    val devicePassword: String = "",
    val authKey: String = "",
    val controlServer: String = "",
)

internal data class FlasherState(
    val available: Boolean,
    val channel: String,
    val channels: List<String>,
    val releasesState: String,
    val releasesError: String,
    val releases: List<FlasherRelease>,
    val selected: String,
    val form: FlasherForm,
    val problems: Map<String, String>,
    val keyIdentity: String,
    val summary: List<FlasherSummaryRow>,
    val disks: List<FlasherDisk>,
    val cardState: String,
    val cardId: String,
    val cardText: String,
    val downloadText: String,
    val phase: String,
    val busy: Boolean,
    val percent: Float,
    val speedText: String,
    val jobError: String,
    val hostname: String,
    val canStart: Boolean,
    val blocked: String,
)

private fun JSONArray?.objects(): List<JSONObject> = this?.let { array -> (0 until array.length()).mapNotNull(array::optJSONObject) }.orEmpty()

private fun JSONArray?.texts(): List<String> = this?.let { array -> (0 until array.length()).map(array::optString) }.orEmpty()

internal fun flasherForm(json: JSONObject?): FlasherForm = json?.let {
    FlasherForm(
        hostname = it.optText("hostname"),
        ssid = it.optText("ssid"),
        wifiPassword = it.optText("wifiPassword"),
        noWifi = it.optBoolean("noWifi"),
        country = it.optText("country"),
        sshMode = it.optText("sshMode").ifBlank { "key-only" },
        authorizedKey = it.optText("authorizedKey"),
        devicePassword = it.optText("devicePassword"),
        authKey = it.optText("authKey"),
        controlServer = it.optText("controlServer"),
    )
} ?: FlasherForm()

internal fun flasherCardText(card: JSONObject?): String = when (card?.optText("state")) {
    "opening" -> "Checking the card reader…"
    "waiting" -> "Allow access to the card reader in the dialog"
    "ready" -> listOf(card.optText("label").ifBlank { "Card reader" }, card.optText("capacityText")).filter { it.isNotBlank() }.joinToString(" · ")
    "failed" -> card.optText("error")
    else -> ""
}

internal fun flasherDownloadText(download: JSONObject?): String = when (download?.optText("state")) {
    "running" -> "Downloading… ${download.optDouble("percent", 0.0).toInt()}%"
    "ready" -> "Downloaded · ${download.optText("imageSizeText")} when written"
    "failed" -> "Download failed: ${download.optText("error")}"
    else -> ""
}

internal fun flasherState(json: JSONObject?): FlasherState? {
    val view = json?.takeIf { it.has("job") } ?: return null
    val releases = view.optJSONObject("releases")
    val identity = view.optJSONObject("keyIdentity")
    val job = view.optJSONObject("job")
    val problems = view.optJSONObject("problems")
    return FlasherState(
        available = view.optBoolean("available"),
        channel = view.optText("channel"),
        channels = view.optJSONArray("channels").texts(),
        releasesState = releases?.optText("state").orEmpty(),
        releasesError = releases?.optText("error").orEmpty(),
        releases = releases?.optJSONArray("items").objects().map { FlasherRelease(it.optText("version"), it.optText("label"), it.optText("published"), it.optText("sizeText")) },
        selected = view.optText("selected"),
        form = flasherForm(view.optJSONObject("form")),
        problems = problems?.keys()?.asSequence()?.associateWith { problems.optText(it) }.orEmpty(),
        keyIdentity = identity?.let { listOf(it.optText("algorithm"), it.optText("fingerprint"), it.optText("comment")).filter(String::isNotBlank).joinToString(" ") }.orEmpty(),
        summary = view.optJSONArray("summary").objects().map { FlasherSummaryRow(it.optText("label"), it.optText("value"), it.optBoolean("warn")) },
        disks = view.optJSONArray("disks").objects().map { FlasherDisk(it.optText("id"), it.optText("name")) },
        cardState = view.optJSONObject("card")?.optText("state").orEmpty(),
        cardId = view.optJSONObject("card")?.optText("id").orEmpty(),
        cardText = flasherCardText(view.optJSONObject("card")),
        downloadText = flasherDownloadText(view.optJSONObject("download")),
        phase = job?.optText("phase").orEmpty(),
        busy = job?.optBoolean("busy") == true,
        percent = (job?.optDouble("percent", 0.0) ?: 0.0).toFloat(),
        speedText = job?.optText("speedText").orEmpty(),
        jobError = job?.optText("error").orEmpty(),
        hostname = job?.optText("hostname").orEmpty(),
        canStart = view.optBoolean("canStart"),
        blocked = view.optText("blocked"),
    )
}

internal fun flasherPhaseLabel(phase: String): String = when (phase) {
    "downloading" -> "Downloading Aircast OS"
    "preparing" -> "Preparing the card"
    "writing" -> "Writing"
    "verifying" -> "Verifying"
    "customizing" -> "Setting up WiFi and access"
    "done" -> "Card ready"
    "failed" -> "Writing failed"
    "cancelled" -> "Cancelled"
    else -> ""
}

internal val FLASHER_SSH_MODES = listOf("key-only" to "Key only", "password" to "Password", "disabled" to "Disabled")

private fun patch(key: String, value: Any) = offMainInOrder { Qgc.refusalOf(FLASHER_FORM, JSONObject().put(key, value)) }

@Composable
fun FlasherScreen(modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    var state by remember { mutableStateOf<FlasherState?>(null) }
    var form by remember { mutableStateOf<FlasherForm?>(null) }
    var refusal by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) { Qgc.invoke(FLASHER_OPEN) }
        while (true) {
            val read = withContext(Dispatchers.Default) { flasherState(Qgc.get(FLASHER_VIEW)) }
            state = read
            if (form == null && read != null) {
                val country = read.form.country.ifBlank { Locale.getDefault().country }
                if (country != read.form.country) patch("country", country)
                form = read.form.copy(country = country)
            }
            delay(FLASHER_POLL_MS)
        }
    }
    LaunchedEffect(Unit) {
        while (true) {
            delay(FLASHER_DISK_POLL_MS)
            withContext(Dispatchers.Default) { Qgc.invoke(FLASHER_DISKS) }
        }
    }
    val busy by rememberUpdatedState(state?.busy == true)
    DisposableEffect(Unit) {
        onDispose { if (busy) offMainInOrder { Qgc.invoke(FLASHER_CANCEL) } }
    }
    BlocksNavigation(state?.busy == true, "Wait for the card to finish writing or cancel it first")

    val command: (String, Array<Any>) -> Unit = { path, args ->
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }.orEmpty() }
    }
    val edit: (FlasherForm, String, Any) -> Unit = { next, key, value ->
        form = next
        patch(key, value)
    }

    val current = state
    val fields = form
    LazyColumn(modifier.fillMaxSize()) {
        item(key = "intro") { FlasherIntro(current) }
        if (current != null && fields != null) {
            if (current.busy || current.phase in listOf("done", "failed", "cancelled")) {
                item(key = "job") {
                    FlasherJob(current, onCancel = { command(FLASHER_CANCEL, emptyArray()) }, onAgain = {
                        scope.launch {
                            val again = withContext(Dispatchers.Default) {
                                Qgc.refusalOf(FLASHER_AGAIN)
                                flasherState(Qgc.get(FLASHER_VIEW))
                            }
                            again?.let { form = it.form; state = it }
                        }
                    })
                }
            } else {
                releaseItems(current, onChannel = { command(FLASHER_CHANNEL, arrayOf(it)) }, onSelect = { command(FLASHER_SELECT, arrayOf(it)) })
                networkItems(fields, current.problems, edit)
                accessItems(fields, current, edit)
                cardItems(current, onChoose = { command(FLASHER_CHOOSE_DISK, arrayOf(it)) })
                item(key = "summary") { FlasherSummary(current.summary) }
                item(key = "write") {
                    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(enabled = current.canStart, onClick = { command(FLASHER_START, emptyArray()) }, modifier = Modifier.fillMaxWidth()) { Text("Write card") }
                        (refusal.ifBlank { current.blocked }).takeIf { it.isNotBlank() }?.let {
                            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FlasherIntro(state: FlasherState?) {
    Surface(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), color = MaterialTheme.colorScheme.primaryContainer, shape = MaterialTheme.shapes.medium) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Flash an Aircast card", style = MaterialTheme.typography.titleSmall)
            Text(
                "Write Aircast OS to an SD card through a USB card reader. WiFi, the hostname and how you get in are set up for the first boot, so the drone comes online on its own.",
                style = MaterialTheme.typography.bodyMedium,
            )
            if (state != null && !state.available) Text("This device cannot write cards.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning)
        }
    }
}

@Composable
private fun SectionTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp))
}

@Composable
private fun FormField(label: String, value: String, problem: String?, secret: Boolean = false, singleLine: Boolean = true, keyboard: KeyboardType = KeyboardType.Text, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = singleLine,
        isError = problem != null,
        supportingText = problem?.let { { Text(it) } },
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(keyboardType = if (secret) KeyboardType.Password else keyboard),
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
    )
}

private fun LazyListScope.releaseItems(state: FlasherState, onChannel: (String) -> Unit, onSelect: (String) -> Unit) {
    item(key = "os-title") { SectionTitle("Operating system") }
    item(key = "channels") {
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            state.channels.forEach { channel ->
                FilterChip(selected = channel == state.channel, onClick = { onChannel(channel) }, label = { Text(channel.replaceFirstChar { it.titlecase(Locale.ROOT) }) })
            }
        }
    }
    when (state.releasesState) {
        "loading" -> item(key = "releases-loading") { ListItem(headlineContent = { Text("Loading releases…") }) }
        "failed" -> item(key = "releases-failed") { ListItem(headlineContent = { Text("Could not load releases") }, supportingContent = { Text(state.releasesError) }) }
        else -> state.releases.forEach { release ->
            item(key = "release-${release.version}") {
                ListItem(
                    headlineContent = { Text(release.label) },
                    supportingContent = { Text("${release.published} · ${release.sizeText} download") },
                    leadingContent = { RadioButton(selected = release.version == state.selected, onClick = null) },
                    modifier = Modifier.selectable(selected = release.version == state.selected, onClick = { onSelect(release.version) }),
                )
            }
        }
    }
    if (state.downloadText.isNotBlank()) item(key = "download") { Text(state.downloadText, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 16.dp)) }
}

private fun LazyListScope.networkItems(form: FlasherForm, problems: Map<String, String>, edit: (FlasherForm, String, Any) -> Unit) {
    item(key = "network-title") { SectionTitle("Network") }
    item(key = "hostname") { FormField("Hostname", form.hostname, problems["hostname"]) { edit(form.copy(hostname = it), "hostname", it) } }
    item(key = "no-wifi") {
        ListItem(
            headlineContent = { Text("No WiFi") },
            supportingContent = { Text("The drone uses Ethernet or cellular") },
            trailingContent = { Checkbox(checked = form.noWifi, onCheckedChange = null) },
            modifier = Modifier.selectable(selected = form.noWifi, onClick = { edit(form.copy(noWifi = !form.noWifi), "noWifi", !form.noWifi) }),
        )
    }
    if (!form.noWifi) {
        item(key = "ssid") { FormField("WiFi network", form.ssid, problems["ssid"]) { edit(form.copy(ssid = it), "ssid", it) } }
        item(key = "wifi-password") { FormField("WiFi password", form.wifiPassword, problems["wifiPassword"], secret = true) { edit(form.copy(wifiPassword = it), "wifiPassword", it) } }
        item(key = "country") { FormField("Country code", form.country, null) { value -> value.uppercase(Locale.ROOT).take(2).let { edit(form.copy(country = it), "country", it) } } }
    }
}

private fun LazyListScope.accessItems(form: FlasherForm, state: FlasherState, edit: (FlasherForm, String, Any) -> Unit) {
    item(key = "access-title") { SectionTitle("Access") }
    item(key = "ssh-mode") {
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FLASHER_SSH_MODES.forEach { (mode, label) ->
                FilterChip(selected = form.sshMode == mode, onClick = { edit(form.copy(sshMode = mode), "sshMode", mode) }, label = { Text(label) })
            }
        }
    }
    when (form.sshMode) {
        "key-only" -> {
            item(key = "ssh-key") { FormField("SSH public key", form.authorizedKey, state.problems["authorizedKey"], singleLine = false) { edit(form.copy(authorizedKey = it), "authorizedKey", it) } }
            if (state.keyIdentity.isNotBlank()) item(key = "ssh-identity") { Text(state.keyIdentity, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(horizontal = 16.dp)) }
        }
        "password" -> item(key = "device-password") { FormField("Password for pi", form.devicePassword, state.problems["devicePassword"], secret = true) { edit(form.copy(devicePassword = it), "devicePassword", it) } }
    }
    item(key = "auth-key") { FormField("Tailscale auth key (optional)", form.authKey, null, secret = true) { edit(form.copy(authKey = it), "authKey", it) } }
    if (form.authKey.isNotBlank()) {
        item(key = "control-server") { FormField("Headscale server (blank for Tailscale)", form.controlServer, state.problems["controlServer"], keyboard = KeyboardType.Uri) { edit(form.copy(controlServer = it), "controlServer", it) } }
    }
}

private fun LazyListScope.cardItems(state: FlasherState, onChoose: (String) -> Unit) {
    item(key = "card-title") { SectionTitle("Card") }
    if (state.disks.isEmpty()) item(key = "no-disks") { ListItem(headlineContent = { Text("No card reader connected") }, supportingContent = { Text("Plug a USB card reader with the SD card in it into this phone") }) }
    state.disks.forEach { disk ->
        item(key = "disk-${disk.id}") {
            val chosen = disk.id == state.cardId
            ListItem(
                headlineContent = { Text(disk.name) },
                supportingContent = state.cardText.takeIf { chosen && it.isNotBlank() }?.let { text -> { Text(text) } },
                leadingContent = { RadioButton(selected = chosen, onClick = null) },
                modifier = Modifier.selectable(selected = chosen, onClick = { onChoose(disk.id) }),
            )
        }
    }
}

@Composable
private fun FlasherSummary(rows: List<FlasherSummaryRow>) {
    SectionTitle("Summary")
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        rows.forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(row.label, style = MaterialTheme.typography.bodyMedium)
                Text(row.value, style = MaterialTheme.typography.bodyMedium, color = if (row.warn) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurface)
            }
        }
    }
}

@Composable
private fun FlasherJob(state: FlasherState, onCancel: () -> Unit, onAgain: () -> Unit) {
    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(flasherPhaseLabel(state.phase), style = MaterialTheme.typography.titleMedium)
        if (state.busy) {
            LinearProgressIndicator(progress = { state.percent / 100f }, modifier = Modifier.fillMaxWidth())
            Text(listOf("${state.percent.toInt()}%", state.speedText).filter { it.isNotBlank() }.joinToString(" · "), style = MaterialTheme.typography.bodyMedium)
            OutlinedButton(onClick = onCancel) { Text("Cancel") }
        } else {
            when (state.phase) {
                "done" -> Text(
                    "Put the card in the drone and power it on. It joins the network as ${state.hostname.ifBlank { "the image default name" }}${if (state.hostname.isBlank()) "" else ".local"}.",
                    style = MaterialTheme.typography.bodyMedium,
                )
                "failed" -> Text(state.jobError, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning)
                else -> Text("The card is incomplete; write it again before using it.", style = MaterialTheme.typography.bodyMedium)
            }
            Button(onClick = onAgain) { Text(if (state.phase == "done") "Flash another" else "Back") }
        }
    }
}
