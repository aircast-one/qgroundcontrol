package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.BuildConfig
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal const val FLASHER_PAGE = "Aircast card"
internal const val FLASHER_VIEW = "view.flasher"
internal const val FLASHER_OPEN = "flasher.open"
internal const val FLASHER_CHANNEL = "flasher.channel"
internal const val FLASHER_SELECT = "flasher.select"
internal const val FLASHER_DOWNLOAD = "flasher.download"
internal const val FLASHER_FORM = "flasher.form"
internal const val FLASHER_DISKS = "flasher.disks"
internal const val FLASHER_CHOOSE_DISK = "flasher.chooseDisk"
internal const val FLASHER_START = "flasher.start"
internal const val FLASHER_CANCEL = "flasher.cancel"
internal const val FLASHER_AGAIN = "flasher.again"
internal const val FLASHER_POLL_MS = 500L
internal const val FLASHER_DISK_POLL_MS = 2_000L
internal const val FLASHER_PASSWORD_MODE = "password"
internal val FLASHER_FINISHED = setOf("done", "failed", "cancelled")
internal val FLASHER_SETUP_FIELDS = setOf("hostname", "ssid", "wifiPassword", "devicePassword", "authorizedKey", "controlServer")
internal val FLASHER_MORE_FIELDS = setOf("authorizedKey", "controlServer")
internal val FLASHER_FIELD_LABELS = mapOf(
    "hostname" to "Drone name",
    "ssid" to "WiFi network",
    "wifiPassword" to "WiFi password",
    "devicePassword" to "Password for pi",
    "authorizedKey" to "SSH public key",
    "controlServer" to "Headscale server",
)
internal val FLASHER_MORE_ROWS = setOf("Image", "Remote access")

internal enum class FlasherStep(val title: String) { Card("Card"), Setup("Setup"), Write("Write") }

internal data class FlasherRelease(val version: String, val label: String, val published: String, val sizeText: String, val recommended: Boolean = false)

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
    val downloadState: String,
    val downloadText: String,
    val phase: String,
    val busy: Boolean,
    val percent: Float,
    val speedText: String,
    val jobError: String,
    val hostname: String,
    val canStart: Boolean,
    val blocked: String,
    val firstProblemField: String = "",
    val firstProblem: String = "",
    val remainingText: String = "",
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
    "waiting" -> "Tap OK on the prompt to let Aircast use the card reader"
    "ready" -> listOf(card.optText("label").ifBlank { "Card reader" }, card.optText("capacityText")).filter { it.isNotBlank() }.joinToString(" · ")
    "failed" -> card.optText("error")
    else -> ""
}

internal fun flasherDownloadText(download: JSONObject?): String = when (download?.optText("state")) {
    "running" -> "Downloading Aircast OS… ${download.optDouble("percent", 0.0).toInt()}%"
    "ready" -> "Aircast OS is downloaded"
    "failed" -> "Download failed: ${download.optText("error")}"
    else -> ""
}

internal fun flasherState(json: JSONObject?): FlasherState? {
    val view = json?.takeIf { it.has("job") } ?: return null
    val releases = view.optJSONObject("releases")
    val identity = view.optJSONObject("keyIdentity")
    val job = view.optJSONObject("job")
    val problems = view.optJSONObject("problems")
    val card = view.optJSONObject("card")
    return FlasherState(
        available = view.optBoolean("available"),
        channel = view.optText("channel"),
        channels = view.optJSONArray("channels").texts(),
        releasesState = releases?.optText("state").orEmpty(),
        releasesError = releases?.optText("error").orEmpty(),
        releases = releases?.optJSONArray("items").objects().map {
            FlasherRelease(it.optText("version"), it.optText("label"), it.optText("published"), it.optText("sizeText"), it.optBoolean("recommended"))
        },
        selected = view.optText("selected"),
        form = flasherForm(view.optJSONObject("form")),
        problems = problems?.keys()?.asSequence()?.associateWith { problems.optText(it) }.orEmpty(),
        keyIdentity = identity?.let { listOf(it.optText("algorithm"), it.optText("fingerprint"), it.optText("comment")).filter(String::isNotBlank).joinToString(" ") }.orEmpty(),
        summary = view.optJSONArray("summary").objects().map { FlasherSummaryRow(it.optText("label"), it.optText("value"), it.optBoolean("warn")) },
        disks = view.optJSONArray("disks").objects().map { FlasherDisk(it.optText("id"), it.optText("name")) },
        cardState = card?.optText("state").orEmpty(),
        cardId = card?.optText("id").orEmpty(),
        cardText = flasherCardText(card),
        downloadState = view.optJSONObject("download")?.optText("state").orEmpty(),
        downloadText = flasherDownloadText(view.optJSONObject("download")),
        phase = job?.optText("phase").orEmpty(),
        busy = job?.optBoolean("busy") == true,
        percent = (job?.optDouble("percent", 0.0) ?: 0.0).toFloat(),
        speedText = job?.optText("speedText").orEmpty(),
        jobError = job?.optText("error").orEmpty(),
        hostname = job?.optText("hostname").orEmpty(),
        canStart = view.optBoolean("canStart"),
        blocked = view.optText("blocked"),
        firstProblemField = view.optJSONObject("firstProblem")?.optText("field").orEmpty(),
        firstProblem = view.optJSONObject("firstProblem")?.optText("message").orEmpty(),
        remainingText = job?.optText("remainingText").orEmpty(),
    )
}

internal fun flasherPhaseLabel(phase: String): String = when (phase) {
    "downloading" -> "Downloading Aircast OS"
    "preparing" -> "Preparing the card"
    "writing" -> "Writing the card"
    "verifying" -> "Checking what was written"
    "customizing" -> "Setting up WiFi and access"
    "done" -> "Card ready"
    "failed" -> "Writing failed"
    "cancelled" -> "Cancelled"
    else -> ""
}

internal fun flasherStepFor(state: FlasherState?): FlasherStep? = state?.takeIf { it.busy || it.phase in FLASHER_FINISHED }?.let { FlasherStep.Write }

internal fun readerToOpen(state: FlasherState): String? =
    state.disks.singleOrNull()?.id?.takeIf { state.cardState == "none" && !state.busy && state.phase !in FLASHER_FINISHED }

internal fun visibleProblems(problems: Map<String, String>, touched: Set<String>, showAll: Boolean): Map<String, String> =
    problems.filterKeys { showAll || it in touched }

internal fun setupComplete(problems: Map<String, String>): Boolean = problems.keys.none { it in FLASHER_SETUP_FIELDS }

internal fun defaultSshMode(form: FlasherForm): String? = FLASHER_PASSWORD_MODE.takeIf { form.sshMode == "key-only" && form.authorizedKey.isBlank() }

internal fun meteredNote(release: FlasherRelease?, downloadState: String, metered: Boolean): String? =
    release?.takeIf { metered && downloadState != "ready" && downloadState != "running" }?.let { "Aircast OS is a ${it.sizeText} download and this phone is on mobile data." }

internal val FLASHER_SSH_MODES = listOf("password" to "Password", "key-only" to "SSH key", "disabled" to "Off")

internal fun problemNote(state: FlasherState): String =
    state.firstProblem.takeIf { it.isNotBlank() }?.let { message -> FLASHER_FIELD_LABELS[state.firstProblemField]?.let { "$it: $message" } ?: message }.orEmpty()

internal fun problemInMoreOptions(state: FlasherState): Boolean = state.firstProblemField in FLASHER_MORE_FIELDS

internal fun previousStep(step: FlasherStep, busy: Boolean, phase: String): FlasherStep? =
    step.takeIf { it != FlasherStep.Card && !busy && phase !in FLASHER_FINISHED }?.let { FlasherStep.entries[it.ordinal - 1] }

internal fun showsChannels(debug: Boolean, channel: String): Boolean = debug || channel != "stable"

internal fun keepsScreenOn(state: FlasherState?): Boolean = state != null && (state.busy || state.downloadState == "running")

internal fun nextSteps(ssid: String, hostname: String, noWifi: Boolean): List<String> = listOf(
    "Unplug the reader, put the card in the drone and power it on.",
    when {
        noWifi -> "Give it about 3 minutes for its first boot on Ethernet or cellular."
        else -> "Give it about 3 minutes for its first boot. It joins $ssid by itself."
    },
    hostname.takeIf { it.isNotBlank() }?.let { "Open http://$it.local on a phone or computer on the same network to see the drone and its links." }
        ?: "Find it on your network under the image's default name.",
)

private fun patch(key: String, value: Any) = offMainInOrder { Qgc.refusalOf(FLASHER_FORM, JSONObject().put(key, value)) }

@Composable
fun FlasherScreen(modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val phoneWifi by rememberPhoneWifi()
    val phoneSsid = phoneWifi.ssid
    val view = LocalView.current
    var more by remember { mutableStateOf(false) }
    var state by remember { mutableStateOf<FlasherState?>(null) }
    var form by remember { mutableStateOf<FlasherForm?>(null) }
    var step by remember { mutableStateOf(FlasherStep.Card) }
    var touched by remember { mutableStateOf(emptySet<String>()) }
    var showAllProblems by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf("") }
    var metered by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) { Qgc.invoke(FLASHER_OPEN) }
        while (true) {
            val read = withContext(Dispatchers.Default) { flasherState(Qgc.get(FLASHER_VIEW)) }
            state = read
            metered = phoneIsOnMeteredNetwork(context)
            flasherStepFor(read)?.let { step = it }
            if (form == null && read != null) {
                val country = read.form.country.ifBlank { Locale.getDefault().country }
                val sshMode = defaultSshMode(read.form) ?: read.form.sshMode
                if (country != read.form.country) patch("country", country)
                if (sshMode != read.form.sshMode) patch("sshMode", sshMode)
                form = read.form.copy(country = country, sshMode = sshMode)
            }
            read?.let(::readerToOpen)?.let { id -> withContext(Dispatchers.Default) { Qgc.invoke(FLASHER_CHOOSE_DISK, id) } }
            delay(FLASHER_POLL_MS)
        }
    }
    LaunchedEffect(Unit) {
        while (true) {
            delay(FLASHER_DISK_POLL_MS)
            withContext(Dispatchers.Default) { Qgc.invoke(FLASHER_DISKS) }
        }
    }
    LaunchedEffect(phoneSsid, form == null) {
        val current = form ?: return@LaunchedEffect
        val ssid = phoneSsid ?: return@LaunchedEffect
        if (current.ssid.isBlank() && !current.noWifi) {
            form = current.copy(ssid = ssid)
            patch("ssid", ssid)
        }
    }
    LaunchedEffect(phoneWifi, form == null) {
        if (form != null) patch("securedSsid", phoneWifi.ssid?.takeIf { phoneWifi.secured == true }.orEmpty())
    }
    val awake = keepsScreenOn(state)
    DisposableEffect(awake) {
        view.keepScreenOn = awake
        onDispose { view.keepScreenOn = false }
    }
    val busy by rememberUpdatedState(state?.busy == true)
    DisposableEffect(Unit) {
        onDispose { if (busy) offMainInOrder { Qgc.invoke(FLASHER_CANCEL) } }
    }
    BlocksNavigation(state?.busy == true, "Wait for the card to finish writing or cancel it first")
    val back = previousStep(step, state?.busy == true, state?.phase.orEmpty())
    BackHandler(enabled = back != null) { back?.let { step = it } }
    if (back != null) OverridePageHeading(FLASHER_PAGE) { step = back }

    val command: (String, Array<Any>) -> Unit = { path, args ->
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }.orEmpty() }
    }
    val edit: (FlasherForm, String, Any) -> Unit = { next, key, value ->
        form = next
        touched = touched + key
        patch(key, value)
    }
    val startDownload = { command(FLASHER_DOWNLOAD, emptyArray()) }

    val current = state
    val fields = form
    Column(modifier.fillMaxSize()) {
        StepHeader(step, onStep = { chosen -> if (back != null && chosen.ordinal < step.ordinal) step = chosen })
        HorizontalDivider()
        if (current == null || fields == null) {
            Text("Loading…", Modifier.padding(16.dp))
            return@Column
        }
        if (!current.available) {
            Text("This device cannot write cards.", Modifier.padding(16.dp), color = MaterialTheme.aircast.warning)
            return@Column
        }
        val release = current.releases.firstOrNull { it.version == current.selected }
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(vertical = 8.dp)) {
            when (step) {
                FlasherStep.Card -> CardStep(current, onChoose = { command(FLASHER_CHOOSE_DISK, arrayOf(it)) })
                FlasherStep.Setup -> SetupStep(
                    state = current,
                    form = fields,
                    phoneSsid = phoneSsid,
                    more = more,
                    onMore = { more = !more },
                    problems = visibleProblems(current.problems, touched, showAllProblems),
                    meteredNote = meteredNote(release, current.downloadState, metered),
                    edit = edit,
                    onSelect = { command(FLASHER_SELECT, arrayOf(it)) },
                    onChannel = { command(FLASHER_CHANNEL, arrayOf(it)) },
                    onDownload = startDownload,
                )
                FlasherStep.Write -> WriteStep(current, fields, meteredNote(release, current.downloadState, metered), onChange = { row ->
                    if (row in FLASHER_MORE_ROWS) more = true
                    step = if (row == "Card") FlasherStep.Card else FlasherStep.Setup
                })
            }
        }
        HorizontalDivider()
        StepActions(
            step = step,
            state = current,
            refusal = refusal,
            setupNote = if (showAllProblems) problemNote(current) else "",
            onBack = {
                refusal = ""
                step = FlasherStep.entries[step.ordinal - 1]
            },
            onContinue = {
                refusal = ""
                when (step) {
                    FlasherStep.Card -> {
                        step = FlasherStep.Setup
                        if (!metered && current.downloadState in listOf("idle", "failed", "")) startDownload()
                    }
                    FlasherStep.Setup -> {
                        showAllProblems = true
                        if (problemInMoreOptions(current)) more = true
                        if (setupComplete(current.problems)) step = FlasherStep.Write
                    }
                    FlasherStep.Write -> command(FLASHER_START, emptyArray())
                }
            },
            onCancel = { command(FLASHER_CANCEL, emptyArray()) },
            onAgain = {
                val finished = current.phase
                scope.launch {
                    val again = withContext(Dispatchers.Default) {
                        Qgc.refusalOf(FLASHER_AGAIN)
                        flasherState(Qgc.get(FLASHER_VIEW))
                    }
                    again?.let {
                        form = it.form
                        state = it
                    }
                    touched = emptySet()
                    showAllProblems = false
                    step = if (finished == "done") FlasherStep.Card else FlasherStep.Write
                }
            },
        )
    }
}

@Composable
private fun StepHeader(step: FlasherStep, onStep: (FlasherStep) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), horizontalArrangement = Arrangement.SpaceBetween) {
        FlasherStep.entries.forEach { each ->
            Text(
                "${if (each.ordinal < step.ordinal) "✓" else "${each.ordinal + 1}"}  ${each.title}",
                modifier = Modifier.clickable(enabled = each.ordinal < step.ordinal) { onStep(each) }.padding(horizontal = 8.dp, vertical = 8.dp),
                style = MaterialTheme.typography.labelLarge,
                fontWeight = if (each == step) FontWeight.Bold else FontWeight.Normal,
                color = if (each.ordinal <= step.ordinal) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun StepActions(step: FlasherStep, state: FlasherState, refusal: String, setupNote: String, onBack: () -> Unit, onContinue: () -> Unit, onCancel: () -> Unit, onAgain: () -> Unit) {
    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        val note = when {
            refusal.isNotBlank() -> refusal
            step == FlasherStep.Setup && setupNote.isNotBlank() -> setupNote
            step == FlasherStep.Write && !state.busy && state.phase !in FLASHER_FINISHED -> state.blocked
            else -> ""
        }
        if (note.isNotBlank()) Text(note, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            when {
                state.busy -> OutlinedButton(onClick = onCancel, modifier = Modifier.weight(1f)) { Text("Cancel") }
                state.phase in FLASHER_FINISHED -> Button(onClick = onAgain, modifier = Modifier.weight(1f)) { Text(if (state.phase == "done") "Flash another card" else "Try again") }
                else -> {
                    if (step != FlasherStep.Card) OutlinedButton(onClick = onBack, modifier = Modifier.weight(1f)) { Text("Back") }
                    Button(
                        onClick = onContinue,
                        enabled = when (step) {
                            FlasherStep.Card, FlasherStep.Setup -> true
                            FlasherStep.Write -> state.canStart
                        },
                        modifier = Modifier.weight(1f),
                    ) { Text(if (step == FlasherStep.Write) "Write card" else "Continue") }
                }
            }
        }
    }
}

@Composable
private fun Heading(title: String, body: String) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        Text(body, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun CardStep(state: FlasherState, onChoose: (String) -> Unit) {
    Heading("Insert the card", "Put the drone's SD card in a USB card reader and plug it into this phone. Use an OTG adapter if the reader has a full-size USB plug.")
    if (state.disks.isEmpty()) {
        ListItem(headlineContent = { Text("Waiting for a card reader…") }, supportingContent = { Text("You can continue and plug it in before writing") })
    }
    state.disks.forEach { disk ->
        val chosen = disk.id == state.cardId
        val detail = state.cardText.takeIf { chosen && it.isNotBlank() }
        ListItem(
            headlineContent = { Text(disk.name) },
            supportingContent = detail?.let { text -> { Text(text, color = if (state.cardState == "failed") MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurfaceVariant) } },
            leadingContent = { RadioButton(selected = chosen, onClick = null) },
            trailingContent = if (chosen && state.cardState == "ready") ({ Text("✓", color = MaterialTheme.colorScheme.primary) }) else null,
            modifier = Modifier.selectable(selected = chosen, onClick = { onChoose(disk.id) }),
        )
    }
    if (state.cardState == "failed" && state.cardId.isNotBlank()) {
        TextButton(onClick = { onChoose(state.cardId) }, modifier = Modifier.padding(horizontal = 8.dp)) { Text("Try the reader again") }
    }
}

@Composable
private fun Field(label: String, value: String, problem: String?, help: String? = null, secret: Boolean = false, singleLine: Boolean = true, keyboard: KeyboardType = KeyboardType.Text, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = singleLine,
        isError = problem != null,
        supportingText = (problem ?: help)?.let { { Text(it) } },
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(keyboardType = if (secret) KeyboardType.Password else keyboard),
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
    )
}

@Composable
private fun SetupStep(
    state: FlasherState,
    form: FlasherForm,
    phoneSsid: String?,
    more: Boolean,
    onMore: () -> Unit,
    problems: Map<String, String>,
    meteredNote: String?,
    edit: (FlasherForm, String, Any) -> Unit,
    onSelect: (String) -> Unit,
    onChannel: (String) -> Unit,
    onDownload: () -> Unit,
) {
    Heading("Set up the drone", "These are applied on the drone's first boot, so it joins your network by itself.")
    Field("Drone name", form.hostname, problems["hostname"], help = form.hostname.takeIf { it.isNotBlank() }?.let { "It answers at $it.local" }) { edit(form.copy(hostname = it), "hostname", it) }
    if (!form.noWifi) {
        Field("WiFi network", form.ssid, problems["ssid"], help = "This phone's current network".takeIf { phoneSsid != null && form.ssid == phoneSsid }) { edit(form.copy(ssid = it), "ssid", it) }
        phoneSsid?.takeIf { it != form.ssid }?.let { ssid ->
            AssistChip(onClick = { edit(form.copy(ssid = ssid), "ssid", ssid) }, label = { Text("Use $ssid") }, modifier = Modifier.padding(horizontal = 16.dp))
        }
        Field("WiFi password", form.wifiPassword, problems["wifiPassword"], secret = true) { edit(form.copy(wifiPassword = it), "wifiPassword", it) }
    }
    ListItem(
        headlineContent = { Text("No WiFi") },
        supportingContent = { Text("The drone goes online over Ethernet or cellular") },
        trailingContent = { Checkbox(checked = form.noWifi, onCheckedChange = null) },
        modifier = Modifier.selectable(selected = form.noWifi, onClick = { edit(form.copy(noWifi = !form.noWifi), "noWifi", !form.noWifi) }),
    )
    if (form.sshMode == FLASHER_PASSWORD_MODE) {
        Field("Password for pi", form.devicePassword, problems["devicePassword"], help = "At least 8 characters. You sign in to the drone as pi with it.", secret = true) { edit(form.copy(devicePassword = it), "devicePassword", it) }
    }
    DownloadLine(state, meteredNote, onDownload)
    TextButton(onClick = onMore, modifier = Modifier.padding(horizontal = 8.dp)) { Text(if (more) "Fewer options" else "More options") }
    if (more) MoreOptions(state, form, problems, edit, onSelect, onChannel)
}

@Composable
private fun DownloadLine(state: FlasherState, meteredNote: String?, onDownload: () -> Unit) {
    if (meteredNote != null) {
        Column(Modifier.padding(horizontal = 16.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(meteredNote, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning)
            TextButton(onClick = onDownload) { Text("Download now anyway") }
        }
    } else if (state.downloadText.isNotBlank()) {
        Text(state.downloadText, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
    }
}

@Composable
private fun MoreOptions(state: FlasherState, form: FlasherForm, problems: Map<String, String>, edit: (FlasherForm, String, Any) -> Unit, onSelect: (String) -> Unit, onChannel: (String) -> Unit) {
    SubHeading("Aircast OS version")
    state.releases.forEach { release ->
        ListItem(
            headlineContent = { Text(release.label) },
            supportingContent = { Text(listOfNotNull(release.published, "${release.sizeText} download", "Recommended".takeIf { release.recommended }).joinToString(" · ")) },
            leadingContent = { RadioButton(selected = release.version == state.selected, onClick = null) },
            modifier = Modifier.selectable(selected = release.version == state.selected, onClick = { onSelect(release.version) }),
        )
    }
    if (state.releasesState == "failed") Text(state.releasesError, color = MaterialTheme.aircast.warning, modifier = Modifier.padding(horizontal = 16.dp))
    if (showsChannels(BuildConfig.DEBUG, state.channel)) Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        state.channels.forEach { channel ->
            FilterChip(selected = channel == state.channel, onClick = { onChannel(channel) }, label = { Text(channel.replaceFirstChar { it.titlecase(Locale.ROOT) }) })
        }
    }
    SubHeading("Signing in to the drone")
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FLASHER_SSH_MODES.forEach { (mode, label) ->
            FilterChip(selected = form.sshMode == mode, onClick = { edit(form.copy(sshMode = mode), "sshMode", mode) }, label = { Text(label) })
        }
    }
    if (form.sshMode == "key-only") {
        Field("SSH public key", form.authorizedKey, problems["authorizedKey"], singleLine = false) { edit(form.copy(authorizedKey = it), "authorizedKey", it) }
        if (state.keyIdentity.isNotBlank()) Text(state.keyIdentity, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(horizontal = 16.dp))
    }
    SubHeading("Remote access")
    Field("Tailscale auth key", form.authKey, null, help = "The drone joins your tailnet on first boot", secret = true) { edit(form.copy(authKey = it), "authKey", it) }
    if (form.authKey.isNotBlank()) {
        Field("Headscale server", form.controlServer, problems["controlServer"], help = "Leave empty for Tailscale", keyboard = KeyboardType.Uri) { edit(form.copy(controlServer = it), "controlServer", it) }
    }
    SubHeading("WiFi region")
    Field("Country code", form.country, null, help = "Sets which WiFi channels the drone may use") { value -> value.uppercase(Locale.ROOT).take(2).let { edit(form.copy(country = it), "country", it) } }
    Spacer(Modifier.padding(4.dp))
}

@Composable
private fun SubHeading(text: String) {
    Text(text, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp))
}

@Composable
private fun WriteStep(state: FlasherState, form: FlasherForm, meteredNote: String?, onChange: (String) -> Unit) {
    when {
        state.busy -> Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(flasherPhaseLabel(state.phase), style = MaterialTheme.typography.titleLarge)
            LinearProgressIndicator(progress = { state.percent / 100f }, modifier = Modifier.fillMaxWidth())
            Text(listOf("${state.percent.toInt()}%", state.speedText, state.remainingText).filter { it.isNotBlank() }.joinToString(" · "), style = MaterialTheme.typography.bodyMedium)
            Text("Keep the card reader plugged in until this finishes.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        state.phase == "done" -> Column(Modifier.fillMaxWidth()) {
            Heading("Card ready", "The card is written and checked. It's safe to unplug the reader.")
            SubHeading("What happens next")
            nextSteps(form.ssid.trim(), state.hostname, form.noWifi).forEachIndexed { index, line ->
                Text("${index + 1}. $line", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
            }
        }
        state.phase == "failed" -> Column(Modifier.fillMaxWidth()) {
            Heading("Writing failed", "The card is not usable as it is. Fix the problem below and write it again.")
            Text(state.jobError, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning, modifier = Modifier.padding(horizontal = 16.dp))
        }
        state.phase == "cancelled" -> Heading("Cancelled", "The card is incomplete. Write it again before putting it in the drone.")
        else -> {
            Heading("Check and write", "Everything on the card is replaced.")
            state.summary.forEach { row ->
                Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(row.label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(row.value, style = MaterialTheme.typography.bodyMedium, color = if (row.warn) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurface)
                    TextButton(onClick = { onChange(row.label) }) { Text("Change") }
                }
            }
            Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Card", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                Text(state.cardText.ifBlank { "Not connected" }, style = MaterialTheme.typography.bodyMedium, color = if (state.cardState == "ready") MaterialTheme.colorScheme.onSurface else MaterialTheme.aircast.warning)
                TextButton(onClick = { onChange("Card") }) { Text("Change") }
            }
            state.downloadText.takeIf { state.downloadState != "ready" && it.isNotBlank() }?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
            }
            meteredNote?.let { Text("$it It downloads when you write the card.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)) }
        }
    }
}
