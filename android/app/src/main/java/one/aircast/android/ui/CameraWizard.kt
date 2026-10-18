package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject
import java.io.File

private const val VERIFY_ATTEMPTS = 10
private const val VERIFY_POLL_MS = 1500L
private const val PORT_UP_ATTEMPTS = 15
private const val PORT_UP_POLL_MS = 1000L
private const val SCAN_PORTS = "/api/streams/scan/ports"
private val BOX_SHAPE = RoundedCornerShape(12.dp)
private val CABLE_CHECKS = listOf(
    "The cable goes from the camera to this device's camera port",
    "The camera has power — its light is on",
    "Give it 30 seconds after power-on",
)

private data class CameraSignIn(val shown: Boolean, val asked: Boolean)

private data class TemplateRun(val busy: String, val note: String)

private data class ApplyStatus(val error: Boolean, val text: String)

private val APPLYING = ApplyStatus(false, "Applying…")
private val NO_FIXER = PortFixer(open = false, input = "", suggest = "", status = "", busy = false)
private val NO_SIGN_IN = CameraSignIn(shown = false, asked = false)
private val NO_TEMPLATE = TemplateRun("", "")

private fun errorOf(json: JSONObject?, fallback: String): String = json?.optText("error").orEmpty().ifEmpty { fallback }

private suspend fun fetchPorts(): List<ScanPort> = deviceRead(SCAN_PORTS, ::parseScanPorts).orEmpty()

private suspend fun waitForPort(iface: String, want: String, attempt: Int = 0) {
    if (attempt >= PORT_UP_ATTEMPTS) return
    val port = fetchPorts().firstOrNull { it.iface == iface }
    if (port == null || (if (want.isNotEmpty()) portHasSubnet(port, want) else port.state != "configuring")) return
    delay(PORT_UP_POLL_MS)
    waitForPort(iface, want, attempt + 1)
}

private suspend fun setPortAddress(iface: String, address: String): String {
    val sent = deviceSend(DeviceMethod.POST, "/api/setup/camera/port", JSONObject(mapOf("interface" to iface, "address" to address)))
    return when (sent.refusal) {
        null -> ""
        NO_ANSWER -> NO_ANSWER.message
        else -> errorOf(sent.result, "Could not set up this port.")
    }
}

private fun stagedVideo(context: Context, uri: Uri): File? = runCatching {
    val name = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }.orEmpty().replace('/', '_').ifEmpty { "video.mp4" }
    context.contentResolver.openInputStream(uri)?.use { source -> File(context.cacheDir, name).also { target -> target.outputStream().use { source.copyTo(it) } } }
}.getOrNull()

private fun Modifier.boxed(border: Color, onClick: (() -> Unit)? = null, enabled: Boolean = true, fill: Color = Color.Transparent): Modifier =
    fillMaxWidth().clip(BOX_SHAPE).background(fill).border(1.dp, border, BOX_SHAPE)
        .then(if (onClick != null) Modifier.clickable(enabled = enabled, onClick = onClick) else Modifier)
        .padding(horizontal = 16.dp, vertical = 12.dp)

@OptIn(ExperimentalLayoutApi::class, ExperimentalFoundationApi::class)
@Composable
internal fun CameraWizard(view: CameraWizardView, scan: Boolean = false, onCancel: (() -> Unit)? = null, onStep: (Int) -> Unit = {}, onDone: (String) -> Unit) {
    val notice = LocalChangeNotice.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var step by remember { mutableIntStateOf(1) }
    var rtsp by remember { mutableStateOf(view.rtsp) }
    var name by remember { mutableStateOf(view.suggestedName) }
    var addressOpen by rememberSaveable { mutableStateOf(view.addressOpen) }
    var namingOpen by rememberSaveable { mutableStateOf(false) }
    var techOpen by rememberSaveable { mutableStateOf(false) }
    var portOpen by rememberSaveable { mutableStateOf(false) }
    var fileOpen by rememberSaveable { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<WizardRefusal?>(null) }
    var dupAck by remember { mutableStateOf("") }
    var scanning by remember { mutableStateOf(false) }
    var scanNote by remember { mutableStateOf("") }
    var outcome by remember { mutableStateOf<ScanOutcome?>(null) }
    var fixers by remember { mutableStateOf(emptyMap<String, PortFixer>()) }
    var picked by remember { mutableStateOf<FoundCamera?>(null) }
    var editRtsp by remember { mutableStateOf(false) }
    var base by remember { mutableStateOf("") }
    var signIn by remember { mutableStateOf(NO_SIGN_IN) }
    var user by remember { mutableStateOf("") }
    var pass by remember { mutableStateOf("") }
    var iface by remember { mutableStateOf(view.iface) }
    var ifaceTouched by remember { mutableStateOf(false) }
    var ip by remember { mutableStateOf("") }
    var transport by remember { mutableStateOf(view.transport) }
    var status by remember { mutableStateOf(APPLYING) }
    var applyField by remember { mutableStateOf("") }
    var added by remember { mutableStateOf(false) }
    var template by remember { mutableStateOf(NO_TEMPLATE) }
    var noPort by remember { mutableStateOf(false) }
    var file by remember { mutableStateOf<File?>(null) }
    var uploading by remember { mutableStateOf(false) }
    var applying by remember { mutableStateOf<Job?>(null) }
    val errorLine = remember { BringIntoViewRequester() }

    val ports = outcome?.ports.orEmpty()
    val cams = outcome?.cams.orEmpty()
    val locked = picked?.pathFound == true && !editRtsp
    val cameraName = if (view.isNew) name.trim().lowercase() else view.name

    LaunchedEffect(refusal) {
        if (refusal != null) errorLine.bringIntoView()
    }

    LaunchedEffect(step) { onStep(step) }

    fun pick(cam: FoundCamera): String {
        picked = cam
        editRtsp = false
        if (!cam.pathFound) addressOpen = true
        base = cam.url
        signIn = CameraSignIn(shown = cam.needsAuth, asked = false)
        if (!cam.needsAuth) {
            user = ""
            pass = ""
        }
        val url = camURL(cam.url, user, pass)
        if (cam.url.isNotEmpty()) rtsp = url
        return url
    }

    suspend fun runScan(auto: Boolean, typed: String): ScanOutcome? {
        scanning = true
        scanNote = SCANNING_NOTE
        outcome = null
        val sent = deviceSend(DeviceMethod.POST, "/api/streams/scan")
        scanning = false
        if (sent.refusal != null) {
            scanNote = SCAN_FAILED_NOTE
            return null
        }
        val result = scanOutcome(parseFoundCameras(sent.result, view.name), parseScanPorts(sent.result), rtspHost(view.source))
        outcome = result
        fixers = fixersFor(result, view.subnets)
        scanNote = result.note
        picked = null
        base = ""
        signIn = NO_SIGN_IN
        adoptedCamera(result, auto, typed)?.let { pick(it) }
        return result
    }

    LaunchedEffect(Unit) {
        if (view.isNew) {
            launch { noPort = fetchPorts().let { found -> found.isNotEmpty() && found.none { !it.isUplink && it.type == "ethernet" } } }
            runScan(true, view.rtsp)
        } else if (scan) {
            runScan(false, view.rtsp)
        }
    }

    fun askSignIn(url: String, reason: String) {
        base = url
        signIn = CameraSignIn(shown = true, asked = true)
        step = 1
        refusal = WizardRefusal(reason, false, null)
    }

    suspend fun verify(url: String, camera: String, attempt: Int, reason: String) {
        if (attempt >= VERIFY_ATTEMPTS) {
            status = ApplyStatus(true, reason.ifEmpty { "No video yet — check the cable, IP and RTSP URL." })
            return
        }
        val answer = deviceRead("/api/setup/camera/status?source=${uriComponent(url)}&name=${uriComponent(camera)}", ::parseCameraStatus)
        when (answer?.state) {
            "live" -> {
                notice.show(if (view.isNew) "Camera $camera added." else "Stream is live — receiving video from the camera.", null)
                onDone(camera)
            }
            "needs_auth" -> askSignIn(url, answer?.reason.orEmpty())
            else -> {
                status = ApplyStatus(false, "Verifying the stream is live…")
                delay(VERIFY_POLL_MS)
                verify(url, camera, attempt + 1, answer?.reason ?: reason)
            }
        }
    }

    fun applySetup(url: String, camera: String, port: String, portIp: String, how: String) {
        applying?.cancel()
        step = 3
        applyField = ""
        status = ApplyStatus(false, "Applying network and stream settings…")
        applying = scope.launch {
            val body = JSONObject(mapOf("name" to camera, "new" to (view.isNew && !added), "rtsp" to url, "interface" to port, "address" to portIp.trim(), "transport" to how))
            val sent = deviceSend(DeviceMethod.POST, "/api/setup/camera", body)
            val refused = sent.refusal
            if (refused != null) {
                status = ApplyStatus(true, if (refused == NO_ANSWER) "Network error — try again." else errorOf(sent.result, "Setup failed."))
                applyField = refused.field
                return@launch
            }
            added = true
            status = ApplyStatus(false, "Verifying the stream is live…")
            verify(url, camera, 0, "")
        }
    }

    fun advance(typed: String, typedName: String, known: List<ScanPort>) {
        val out = nextStep(NextInput(typed, if (view.isNew) typedName else null, view.others, known, dupAck))
        rtsp = out.rtsp
        out.name?.let { name = it }
        when (out) {
            is NextOutcome.Refuse -> {
                refusal = out.refusal
                dupAck = out.dupAck
                if (out.refusal.field == "rtsp") {
                    editRtsp = true
                    addressOpen = true
                }
                if (out.refusal.field == "name") namingOpen = true
            }
            is NextOutcome.Apply -> {
                refusal = null
                iface = ""
                ip = ""
                applySetup(out.rtsp, out.name ?: view.name, "", "", transport)
            }
            is NextOutcome.Port -> {
                refusal = null
                val suggested = portToSuggest(known)
                if (suggested.isNotEmpty() && !ifaceTouched) iface = suggested
                if (ip.isEmpty()) ip = suggestIP(out.rtsp)
                step = 2
            }
        }
    }

    fun restart() {
        applying?.cancel()
        rtsp = ""
        ip = ""
        transport = view.transports.firstOrNull()?.value ?: view.transport
        refusal = null
        added = false
        picked = null
        base = ""
        signIn = NO_SIGN_IN
        editRtsp = false
        status = APPLYING
        step = 1
    }

    fun credentials(nextUser: String, nextPass: String) {
        user = nextUser
        pass = nextPass
        if (base.isNotEmpty()) rtsp = camURL(base, nextUser, nextPass)
    }

    suspend fun setUpTemplate(t: CameraTemplate) {
        if (template.busy.isNotEmpty()) return
        fun fail(text: String) {
            template = NO_TEMPLATE
            refusal = WizardRefusal(text, false, null)
        }
        refusal = null
        template = TemplateRun(t.id, "Finding the Ethernet port for the ${t.label}…")
        val known = fetchPorts()
        val port = templatePort(known, t) ?: return fail("This device has no Ethernet port to plug the ${t.label} into.")
        if (!portHasSubnet(port, t.cidr)) {
            val holder = known.firstOrNull { it.iface != port.iface && portHasSubnet(it, t.cidr) }
            if (holder != null) return fail(templateHolderError(holder, t))
            template = TemplateRun(t.id, "Putting ${port.iface} on the ${t.label}'s network (${t.cidr})…")
            val refused = setPortAddress(port.iface, t.cidr)
            if (refused.isNotEmpty()) return fail(refused)
            waitForPort(port.iface, t.cidr)
        }
        template = TemplateRun(t.id, "Looking for the ${t.label} at ${t.host}…")
        val result = runScan(false, rtsp)
        val found = result?.cams?.firstOrNull { it.host == t.host && it.pathFound }
        if (result == null || found == null) return fail("No ${t.label} answered at ${t.host} — check its Ethernet cable and that it has power, then try again.")
        val url = pick(found)
        val camera = if (view.others.any { it.name.lowercase() == t.name }) name else t.name
        name = camera
        if (t.gimbal.isNotEmpty()) {
            scope.launch {
                deviceCall(DeviceMethod.PUT, "/api/gimbal/config", JSONObject(mapOf("vendor" to t.gimbal, "cameraAddr" to t.gimbalAddr)))?.let {
                    notice.show("Gimbal control was not switched to this camera — ${it.message}", null)
                }
            }
        }
        template = NO_TEMPLATE
        advance(url, camera, result.ports)
    }

    fun fixer(port: ScanPort): PortFixer = fixers[port.iface] ?: NO_FIXER

    fun changeFixer(port: ScanPort, change: (PortFixer) -> PortFixer) {
        fixers = fixers + (port.iface to change(fixer(port)))
    }

    suspend fun fixPort(port: ScanPort) {
        val want = fixer(port).input.trim()
        changeFixer(port) { it.copy(busy = true, status = "Setting up ${port.iface}…") }
        val refused = setPortAddress(port.iface, want)
        if (refused.isNotEmpty()) return changeFixer(port) { it.copy(busy = false, status = refused) }
        changeFixer(port) { it.copy(status = "Waiting for ${port.iface} to come up on $want…") }
        waitForPort(port.iface, want)
        runScan(false, rtsp)
    }

    fun upload() {
        val chosen = file
        when {
            cameraName.isEmpty() -> notice.show("Name the camera first.", null)
            chosen == null -> notice.show("Choose a video file.", null)
            else -> scope.launch {
                uploading = true
                val sent = deviceUpload(DeviceMethod.PUT, "/api/streams/${uriComponent(cameraName)}/file", listOf(DevicePart("file", chosen)))
                uploading = false
                when (sent.refusal) {
                    null -> {
                        notice.show("Video source added.", null)
                        onDone(cameraName)
                    }
                    NO_ANSWER -> notice.show("Upload failed — check the file size and try again.", null)
                    else -> notice.show(errorOf(sent.result, "Upload failed."), null)
                }
            }
        }
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch { file = withContext(Dispatchers.IO) { stagedVideo(context, chosen) } ?: file }
    }

    val commits = commitsNow(rtsp, ports)
    val fresh = cams.filter { it.used.isEmpty() }
    val checklist = scanNote.isNotEmpty() && cams.isEmpty() && ports.isNotEmpty()
    val current = view.interfaces.firstOrNull { it.value == iface }?.address.orEmpty()

    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).semantics { contentDescription = if (view.isNew) "New camera" else "Set up ${view.name}" },
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StepNames(step, skipsPort = step == 1 && commits)
        if (step == 1) {
            view.piCamera?.let { pi ->
                Row(
                    Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, BOX_SHAPE).padding(horizontal = 16.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(pi.note, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Button(onClick = { advance(PI_CAMERA, name, ports) }) { Text(pi.action) }
                }
            }
            if (view.templates.isNotEmpty() && !noPort) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    view.templates.map { t ->
                        key(t.id) {
                            val ready = templateReadyPort(t, cams, ports)
                            val idle = template.busy.isEmpty()
                            Row(
                                Modifier.alpha(if (idle) 1f else 0.6f).boxed(MaterialTheme.colorScheme.outlineVariant, onClick = { scope.launch { setUpTemplate(t) } }, enabled = idle),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(t.label, style = MaterialTheme.typography.bodyLarge)
                                    Text(templateSub(t, ready), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                Text(templateAction(template.busy == t.id, ready), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                            }
                        }
                    }
                    if (template.note.isNotEmpty()) Hint(template.note)
                }
            }
            if (view.lookingFor.isNotEmpty()) Text(view.lookingFor, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.aircast.warning)
            if (!checklist) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Hint("Plug the camera in and look for it.")
                    LeadButton(scanLeads(rtsp, picked != null), onClick = { scope.launch { runScan(false, rtsp) } }, enabled = !scanning) {
                        Text(if (scanning) "Looking…" else "Scan for cameras")
                    }
                }
            }
            if (scanNote.isNotEmpty() && !(ports.isNotEmpty() && cams.isEmpty())) Hint(scanNote)
            if (checklist) {
                Column(Modifier.boxed(MaterialTheme.colorScheme.outlineVariant), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(checklistHint(ports, view.subnets).ifEmpty { NO_ANSWER_CHECKLIST }, style = MaterialTheme.typography.titleSmall)
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        CABLE_CHECKS.map { Text("☐ $it", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    }
                    Button(onClick = { scope.launch { runScan(false, rtsp) } }, enabled = !scanning) { Text(if (scanning) "Looking…" else "Scan again") }
                }
            }
            if (cams.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(camHeading(cams), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    cams.map { cam ->
                        key("${cam.host} ${cam.url}") {
                            val selected = picked == cam
                            val primary = MaterialTheme.colorScheme.primary
                            Row(
                                Modifier.boxed(
                                    if (selected) primary else MaterialTheme.colorScheme.outlineVariant,
                                    onClick = { pick(cam) },
                                    fill = if (selected) primary.copy(alpha = 0.08f) else Color.Transparent,
                                ),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(cam.host, style = MaterialTheme.typography.bodyLarge, fontFamily = FontFamily.Monospace)
                                    Text(rowNote(cam), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                Text(
                                    when {
                                        selected -> "Selected"
                                        cam.used.isNotEmpty() -> "Add again"
                                        else -> "Use"
                                    },
                                    style = MaterialTheme.typography.labelLarge,
                                    color = primary,
                                )
                            }
                        }
                    }
                    if (fresh.isEmpty()) Hint("Every camera that answered is already set up on this device — a newly plugged-in one has not answered yet.")
                }
            }
            outcome?.takeIf { ports.isNotEmpty() }?.let { shown ->
                Fold(if (cams.isNotEmpty()) "Scan details" else "Details for a technician", techOpen, { techOpen = it }) {
                    ScanDetails(shown, view, ::fixer, ::changeFixer) { port -> scope.launch { fixPort(port) } }
                }
            }
            if (signIn.shown) {
                val userFocus = remember { FocusRequester() }
                LaunchedEffect(signIn) {
                    if (signIn.asked) userFocus.requestFocus()
                }
                Column(Modifier.boxed(MaterialTheme.colorScheme.outlineVariant), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Hint("This camera asked for a sign-in. Enter the username and password set in the camera's own app.")
                    WizardField("Camera username", user, { credentials(it, pass) }, Modifier.focusRequester(userFocus))
                    SecretField("Camera password", pass, { credentials(user, it) })
                }
            }
            if (locked) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Stream", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(base, style = MaterialTheme.typography.bodyMedium, fontFamily = FontFamily.Monospace, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                    TextButton(onClick = {
                        editRtsp = true
                        addressOpen = true
                    }) { Text("Change") }
                }
            } else {
                Fold("I know my camera's address", addressOpen, { addressOpen = it }) {
                    WizardField(
                        "Camera RTSP URL",
                        rtsp,
                        { rtsp = it },
                        placeholder = view.rtspHint,
                        keyboard = KeyboardType.Uri,
                        isError = refusal?.field == "rtsp" || applyField == "rtsp",
                        supportingText = if (picked == null) "Find it in the camera's app or manual — for example rtsp://192.168.8.10:554/stream" else "",
                    )
                }
            }
            if (view.isNew) {
                Fold("Named ${name.trim().lowercase().ifEmpty { "…" }} — change", namingOpen, { namingOpen = it }) {
                    WizardField(
                        "Camera name",
                        name,
                        { name = it },
                        placeholder = "cam",
                        isError = refusal?.field == "name" || applyField == "name",
                        supportingText = "Used in the stream URLs (rtsp://…/${view.suggestedName}) — lowercase letters, numbers, dashes.",
                    )
                }
            }
            refusal?.let { shown ->
                val tint = if (shown.warn) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.error
                Row(Modifier.bringIntoViewRequester(errorLine).semantics { liveRegion = LiveRegionMode.Assertive }, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Icon(painterResource(R.drawable.ic_warning), null, tint = tint, modifier = Modifier.size(18.dp))
                    Text(shown.text, style = MaterialTheme.typography.bodyMedium, color = tint)
                }
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                LeadButton(continueLeads(rtsp), onClick = { advance(rtsp, name, ports) }) { Text(nextLabel(rtsp, ports, view.isNew)) }
                onCancel?.let { TextButton(onClick = it) { Text("Cancel") } }
            }
            Fold("No camera? Use a video file", fileOpen, { fileOpen = it }) {
                Hint("Upload a video and the device loops it as a stream — a stand-in camera for trying out detection and the overlays.")
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick = { picker.launch(arrayOf("video/*")) }) { Text("Choose a video file") }
                    Text(file?.name ?: "H.264 .mp4 works best", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                OutlinedButton(onClick = { upload() }, enabled = !uploading) { Text(if (uploading) "Uploading…" else "Upload & start") }
            }
        }
        if (step == 2) {
            DeviceSelect(
                "Which port is the camera plugged into?",
                iface,
                listOf(DeviceOption("", "Select a port")) + view.interfaces.map { DeviceOption(it.value, it.label) },
                {
                    iface = it
                    ifaceTouched = true
                },
                Modifier.fillMaxWidth(),
            )
            Hint("Usually the Ethernet port your camera's cable connects to. Each port shows its own address — the camera has to be on that same subnet to be reachable.")
            Fold("Port address & transport", portOpen, { portOpen = it }, summary = "This device's address must be on the same subnet as the camera's URL above — change it only if yours differs.") {
                WizardField(
                    "This device's IP on the camera subnet",
                    ip,
                    { ip = it },
                    placeholder = "192.168.8.2/24",
                    keyboard = KeyboardType.Uri,
                    isError = applyField == "address",
                    supportingText = "Suggested from the camera's address — edit if your subnet differs.",
                )
                DeviceSelect("RTSP transport", transport, view.transports, { transport = it }, Modifier.fillMaxWidth())
                Hint("Leave on Automatic; switch to TCP if the video stays black or choppy.")
            }
            FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { step = 1 }) { Text("← Back") }
                Button(onClick = { applySetup(rtsp, cameraName, iface, ip, transport) }) { Text("Set up & verify") }
            }
            applyNote(iface, ip, current).takeIf { it.isNotEmpty() }?.let { Hint(it) }
        }
        if (step == 3) {
            Row(Modifier.semantics { liveRegion = LiveRegionMode.Polite }, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if (status.error) {
                    Icon(painterResource(R.drawable.ic_error), null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
                } else {
                    CircularProgressIndicator(Modifier.size(20.dp).semantics { contentDescription = "Working" }, strokeWidth = 3.dp)
                }
                Text(status.text, style = MaterialTheme.typography.bodyLarge, color = if (status.error) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            }
            FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { restart() }) { Text("Start over") }
                if (status.error) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = { step = 2 }) { Text("Change settings") }
                        Button(onClick = { applySetup(rtsp, cameraName, iface, ip, transport) }) { Text("Try again") }
                    }
                }
            }
        }
    }
}

@Composable
private fun Hint(text: String) {
    Text(text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

@Composable
private fun LeadButton(leads: Boolean, onClick: () -> Unit, enabled: Boolean = true, content: @Composable RowScope.() -> Unit) {
    if (leads) Button(onClick = onClick, enabled = enabled, content = content) else OutlinedButton(onClick = onClick, enabled = enabled, content = content)
}

@Composable
private fun WizardField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    keyboard: KeyboardType = KeyboardType.Text,
    isError: Boolean = false,
    supportingText: String = "",
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        placeholder = placeholder.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
        singleLine = true,
        isError = isError,
        supportingText = supportingText.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
        keyboardOptions = KeyboardOptions(keyboardType = keyboard, autoCorrectEnabled = false),
        modifier = modifier.fillMaxWidth(),
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun StepNames(step: Int, skipsPort: Boolean) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        STEP_NAMES.mapIndexed { index, label ->
            val skipped = index == 1 && skipsPort
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(
                    Modifier.size(width = 24.dp, height = 6.dp).alpha(if (skipped) 0.4f else 1f)
                        .background(if (index + 1 <= step) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant, CircleShape),
                )
                Text(
                    label,
                    style = MaterialTheme.typography.labelMedium,
                    color = if (index + 1 == step) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
                    textDecoration = if (skipped) TextDecoration.LineThrough else null,
                )
            }
        }
    }
}

@Composable
private fun PortRow(label: String, detail: String) {
    Column(Modifier.boxed(MaterialTheme.colorScheme.outlineVariant)) {
        Text(label, style = MaterialTheme.typography.bodyLarge)
        Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun ScanDetails(
    outcome: ScanOutcome,
    view: CameraWizardView,
    fixer: (ScanPort) -> PortFixer,
    onFixer: (ScanPort, (PortFixer) -> PortFixer) -> Unit,
    onFix: (ScanPort) -> Unit,
) {
    var restOpen by rememberSaveable { mutableStateOf(false) }
    var checkedOpen by rememberSaveable { mutableStateOf(false) }
    val host = rtspHost(view.source)
    val rest: @Composable () -> Unit = {
        outcome.rest.map { key(it.port.iface) { PortRow(it.port.label, portDetail(it, view.subnets, host)) } }
    }
    if (outcome.todo.isNotEmpty()) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(todoHeading(outcome.todo, view.subnets), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            outcome.todo.map {
                key(it.port.iface) {
                    PortFix(it.port, portDetail(it, view.subnets, host), outcome.ports, view, fixer(it.port), { change -> onFixer(it.port, change) }) { onFix(it.port) }
                }
            }
        }
    }
    if (outcome.rest.isNotEmpty() && outcome.todo.isNotEmpty()) {
        Fold("${outcome.rest.joinToString(", ") { it.port.label }} — nothing answered", restOpen, { restOpen = it }) { rest() }
    }
    if (outcome.rest.isNotEmpty() && outcome.todo.isEmpty() && outcome.cams.isNotEmpty()) {
        rest()
        Hint(view.coverage)
    }
    if (outcome.rest.isNotEmpty() && outcome.todo.isEmpty() && outcome.cams.isEmpty()) {
        Text("Ports checked", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        rest()
    }
    if (outcome.cams.isEmpty()) Fold("What was checked?", checkedOpen, { checkedOpen = it }) { Hint(view.coverage) }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PortFix(
    port: ScanPort,
    detail: String,
    ports: List<ScanPort>,
    view: CameraWizardView,
    fixer: PortFixer,
    onFixer: ((PortFixer) -> PortFixer) -> Unit,
    onFix: () -> Unit,
) {
    Column(Modifier.boxed(MaterialTheme.colorScheme.primary), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(port.label, style = MaterialTheme.typography.bodyLarge)
        Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (fixer.open) {
            WizardField("This port's address", fixer.input, { input -> onFixer { it.copy(input = input) } }, placeholder = "192.168.144.2/24", keyboard = KeyboardType.Uri)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                subnetOptions(port, ports, view.subnets).map { option ->
                    key(option.cidr) {
                        AssistChip(onClick = { onFixer { it.copy(input = option.cidr) } }, label = { Text(if (option.label.isNotEmpty()) "${option.cidr} · ${option.label}" else option.cidr) })
                    }
                }
            }
            Hint(fixerHint(fixer))
            LeadButton(fixer.suggest.isNotEmpty(), onClick = onFix, enabled = !fixer.busy && fixer.input.isNotBlank()) { Text(fixerGoLabel(port, fixer)) }
            if (fixer.status.isNotEmpty()) Hint(fixer.status)
        } else {
            OutlinedButton(onClick = { onFixer { it.copy(open = true) } }) { Text("Change this port's address") }
        }
    }
}
