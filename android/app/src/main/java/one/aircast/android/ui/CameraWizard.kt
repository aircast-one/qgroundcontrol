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
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
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
import kotlinx.coroutines.flow.fold
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject
import java.io.File

private val BOX_SHAPE = RoundedCornerShape(12.dp)
private val CABLE_CHECKS = listOf(
    "The cable goes from the camera to this device's camera port",
    "The camera has power — its light is on",
    "Give it 30 seconds after power-on",
)

private data class CameraSignIn(val shown: Boolean, val asked: Boolean)

private data class PortInput(val open: Boolean, val text: String)

private val NO_SIGN_IN = CameraSignIn(shown = false, asked = false)
private val NO_WIZARD = parseDeviceWizard(null)

private fun errorOf(json: JSONObject?, fallback: String): String = json?.optText("error").orEmpty().ifEmpty { fallback }

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
    val done by rememberUpdatedState(onDone)
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
    var inputs by remember { mutableStateOf(emptyMap<String, PortInput>()) }
    var picked by remember { mutableStateOf<WizardCamera?>(null) }
    var editRtsp by remember { mutableStateOf(false) }
    var base by remember { mutableStateOf("") }
    var signIn by remember { mutableStateOf(NO_SIGN_IN) }
    var user by remember { mutableStateOf("") }
    var pass by remember { mutableStateOf("") }
    var iface by remember { mutableStateOf(view.iface) }
    var ifaceTouched by remember { mutableStateOf(false) }
    var ip by remember { mutableStateOf("") }
    var transport by remember { mutableStateOf(view.transport) }
    var form by remember { mutableStateOf(NO_FORM) }
    var session by remember { mutableLongStateOf(-1L) }
    var file by remember { mutableStateOf<File?>(null) }
    var uploading by remember { mutableStateOf(false) }
    val errorLine = remember { BringIntoViewRequester() }
    val shown by qgcPath(DEVICE_WIZARD)
    val wizard = remember(shown, session) { parseDeviceWizard(shown).takeIf { it.session == session } ?: NO_WIZARD }

    val locked = picked?.pathFound == true && !editRtsp
    val cameraName = if (view.isNew) name.trim().lowercase() else view.name
    val current = view.interfaces.firstOrNull { it.value == iface }?.address.orEmpty()
    val applyField = if (wizard.apply.state == "failed") wizard.apply.field else ""

    LaunchedEffect(refusal) {
        if (refusal != null) errorLine.bringIntoView()
    }

    LaunchedEffect(step) { onStep(step) }

    LaunchedEffect(rtsp, iface, ip, current, wizard.scans, wizard.scanning) { form = wizardForm(rtsp, iface, ip, current) }

    suspend fun pick(cam: WizardCamera): String {
        picked = cam
        editRtsp = false
        if (!cam.pathFound) addressOpen = true
        base = cam.url
        signIn = CameraSignIn(shown = cam.needsAuth, asked = false)
        if (!cam.needsAuth) {
            user = ""
            pass = ""
        }
        val url = signedInUrl(cam.url, user, pass)
        if (cam.url.isNotEmpty()) rtsp = url
        return url
    }

    fun askSignIn(url: String, reason: String) {
        base = url
        signIn = CameraSignIn(shown = true, asked = true)
        step = 1
        refusal = WizardRefusal(reason, false, null)
    }

    fun applySetup(url: String, camera: String, port: String, portIp: String, how: String) {
        step = 3
        scope.launch { applyCamera(url, camera, port, portIp, how) }
    }

    suspend fun advance(typed: String, typedName: String) {
        val out = wizardNext(typed, typedName, dupAck) ?: return
        rtsp = out.rtsp
        out.name?.let { name = it }
        dupAck = out.dupAck
        val refused = out.refusal
        when {
            refused != null -> {
                refusal = refused
                if (refused.field == "rtsp") {
                    editRtsp = true
                    addressOpen = true
                }
                if (refused.field == "name") namingOpen = true
            }
            out.kind == "apply" -> {
                refusal = null
                iface = ""
                ip = ""
                applySetup(out.rtsp, out.camera, "", "", transport)
            }
            else -> {
                refusal = null
                if (out.iface.isNotEmpty() && !ifaceTouched) iface = out.iface
                if (ip.isEmpty()) ip = out.ip
                step = 2
            }
        }
    }

    LaunchedEffect(Unit) {
        session = openWizard(view, scan)
        snapshotFlow { parseDeviceWizard(shown).takeIf { it.session == session } ?: NO_WIZARD }.fold(NO_WIZARD) { seen, now ->
            if (now.scans > seen.scans) {
                picked = null
                base = ""
                signIn = NO_SIGN_IN
                inputs = emptyMap()
                now.adopt?.let { pick(it) }
            }
            if (now.template.n > seen.template.n) {
                val found = now.template.camera
                if (found == null) {
                    refusal = WizardRefusal(now.template.refusal, false, null)
                } else {
                    val url = pick(found)
                    val camera = now.template.name.ifEmpty { name }
                    name = camera
                    advance(url, camera)
                }
            }
            if (now.notice.n > seen.notice.n) notice.show(now.notice.text, null)
            if (now.apply.n > seen.apply.n) {
                when (now.apply.state) {
                    "live" -> done(now.apply.camera)
                    "signIn" -> askSignIn(now.apply.url, now.apply.text)
                }
            }
            now
        }
    }

    fun restart() {
        scope.launch { restartWizard() }
        rtsp = ""
        ip = ""
        transport = view.transports.firstOrNull()?.value ?: view.transport
        refusal = null
        picked = null
        base = ""
        signIn = NO_SIGN_IN
        editRtsp = false
        step = 1
    }

    fun credentials(nextUser: String, nextPass: String) {
        user = nextUser
        pass = nextPass
        val from = base
        if (from.isNotEmpty()) {
            scope.launch {
                val url = signedInUrl(from, nextUser, nextPass)
                if (user == nextUser && pass == nextPass) rtsp = url
            }
        }
    }

    fun input(port: WizardPort): PortInput = inputs[port.iface] ?: PortInput(port.open, port.suggest)

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

    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).semantics { contentDescription = if (view.isNew) "New camera" else "Set up ${view.name}" },
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StepNames(step, skipsPort = step == 1 && form.skipsPort)
        if (step == 1) {
            view.piCamera?.let { pi ->
                Row(
                    Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, BOX_SHAPE).padding(horizontal = 16.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(pi.note, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Button(onClick = { scope.launch { advance(PI_CAMERA, name) } }) { Text(pi.action) }
                }
            }
            if (wizard.templates.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    wizard.templates.map { t ->
                        key(t.id) {
                            val idle = !wizard.templateBusy
                            Row(
                                Modifier.alpha(if (idle) 1f else 0.6f).boxed(
                                    MaterialTheme.colorScheme.outlineVariant,
                                    onClick = {
                                        refusal = null
                                        scope.launch { setUpCameraMake(t.id) }
                                    },
                                    enabled = idle,
                                ),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(t.label, style = MaterialTheme.typography.bodyLarge)
                                    Text(t.sub, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                Text(t.action, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                            }
                        }
                    }
                    if (wizard.templateNote.isNotEmpty()) DeviceHint(wizard.templateNote)
                }
            }
            if (view.lookingFor.isNotEmpty()) Text(view.lookingFor, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.aircast.warning)
            if (wizard.checklist.isEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    DeviceHint("Plug the camera in and look for it.")
                    LeadButton(scanLeads(rtsp, picked != null), onClick = { scope.launch { scanForCameras() } }, enabled = !wizard.scanning) {
                        Text(if (wizard.scanning) "Looking…" else "Scan for cameras")
                    }
                }
            }
            if (wizard.note.isNotEmpty()) DeviceHint(wizard.note)
            if (wizard.checklist.isNotEmpty()) {
                Column(Modifier.boxed(MaterialTheme.colorScheme.outlineVariant), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(wizard.checklist, style = MaterialTheme.typography.titleSmall)
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        CABLE_CHECKS.map { Text("☐ $it", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    }
                    Button(onClick = { scope.launch { scanForCameras() } }, enabled = !wizard.scanning) { Text(if (wizard.scanning) "Looking…" else "Scan again") }
                }
            }
            if (wizard.cams.isNotEmpty()) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(wizard.camHeading, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    wizard.cams.map { cam ->
                        key("${cam.host} ${cam.url}") {
                            val selected = picked == cam
                            val primary = MaterialTheme.colorScheme.primary
                            Row(
                                Modifier.boxed(
                                    if (selected) primary else MaterialTheme.colorScheme.outlineVariant,
                                    onClick = { scope.launch { pick(cam) } },
                                    fill = if (selected) primary.copy(alpha = 0.08f) else Color.Transparent,
                                ),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(cam.host, style = MaterialTheme.typography.bodyLarge, fontFamily = FontFamily.Monospace)
                                    Text(cam.note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                Text(if (selected) "Selected" else cam.action, style = MaterialTheme.typography.labelLarge, color = primary)
                            }
                        }
                    }
                    if (wizard.camsHint.isNotEmpty()) DeviceHint(wizard.camsHint)
                }
            }
            if (wizard.details.isNotEmpty()) {
                Fold(wizard.details, techOpen, { techOpen = it }) {
                    ScanDetails(wizard, view.coverage, ::input, { port, change -> inputs = inputs + (port.iface to change) }) { port ->
                        scope.launch { fixWizardPort(port.iface, input(port).text.trim()) }
                    }
                }
            }
            if (signIn.shown) {
                val userFocus = remember { FocusRequester() }
                LaunchedEffect(signIn) {
                    if (signIn.asked) userFocus.requestFocus()
                }
                Column(Modifier.boxed(MaterialTheme.colorScheme.outlineVariant), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    DeviceHint("This camera asked for a sign-in. Enter the username and password set in the camera's own app.")
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
                LeadButton(continueLeads(rtsp), onClick = { scope.launch { advance(rtsp, name) } }) { Text(form.next) }
                onCancel?.let { TextButton(onClick = it) { Text("Cancel") } }
            }
            Fold("No camera? Use a video file", fileOpen, { fileOpen = it }) {
                DeviceHint("Upload a video and the device loops it as a stream — a stand-in camera for trying out detection and the overlays.")
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
            DeviceHint("Usually the Ethernet port your camera's cable connects to. Each port shows its own address — the camera has to be on that same subnet to be reachable.")
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
                DeviceHint("Leave on Automatic; switch to TCP if the video stays black or choppy.")
            }
            FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { step = 1 }) { Text("← Back") }
                Button(onClick = { applySetup(rtsp, cameraName, iface, ip, transport) }) { Text("Set up & verify") }
            }
            if (form.applyNote.isNotEmpty()) DeviceHint(form.applyNote)
        }
        if (step == 3) {
            val failed = wizard.apply.state == "failed"
            Row(Modifier.semantics { liveRegion = LiveRegionMode.Polite }, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if (failed) {
                    Icon(painterResource(R.drawable.ic_error), null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
                } else {
                    CircularProgressIndicator(Modifier.size(20.dp).semantics { contentDescription = "Working" }, strokeWidth = 3.dp)
                }
                Text(wizard.apply.text.ifEmpty { "Applying…" }, style = MaterialTheme.typography.bodyLarge, color = if (failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            }
            FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { restart() }) { Text("Start over") }
                if (failed) {
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
    wizard: DeviceWizard,
    coverage: String,
    input: (WizardPort) -> PortInput,
    onInput: (WizardPort, PortInput) -> Unit,
    onFix: (WizardPort) -> Unit,
) {
    var restOpen by rememberSaveable { mutableStateOf(false) }
    var checkedOpen by rememberSaveable { mutableStateOf(false) }
    val rest: @Composable () -> Unit = {
        wizard.rest.map { key(it.iface) { PortRow(it.label, it.detail) } }
    }
    if (wizard.todo.isNotEmpty()) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(wizard.todoHeading, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            wizard.todo.map {
                key(it.iface) {
                    PortFix(it, input(it), { change -> onInput(it, change) }) { onFix(it) }
                }
            }
        }
    }
    if (wizard.rest.isNotEmpty() && wizard.todo.isNotEmpty()) {
        Fold(wizard.restTitle, restOpen, { restOpen = it }) { rest() }
    }
    if (wizard.rest.isNotEmpty() && wizard.todo.isEmpty() && wizard.cams.isNotEmpty()) {
        rest()
        DeviceHint(coverage)
    }
    if (wizard.rest.isNotEmpty() && wizard.todo.isEmpty() && wizard.cams.isEmpty()) {
        Text("Ports checked", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        rest()
    }
    if (wizard.cams.isEmpty()) Fold("What was checked?", checkedOpen, { checkedOpen = it }) { DeviceHint(coverage) }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PortFix(port: WizardPort, input: PortInput, onInput: (PortInput) -> Unit, onFix: () -> Unit) {
    Column(Modifier.boxed(MaterialTheme.colorScheme.primary), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(port.label, style = MaterialTheme.typography.bodyLarge)
        Text(port.detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (input.open) {
            WizardField("This port's address", input.text, { onInput(input.copy(text = it)) }, placeholder = "192.168.144.2/24", keyboard = KeyboardType.Uri)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                port.options.map { option ->
                    key(option.cidr) {
                        AssistChip(onClick = { onInput(input.copy(text = option.cidr)) }, label = { Text(option.text) })
                    }
                }
            }
            DeviceHint(port.hint)
            LeadButton(port.suggest.isNotEmpty(), onClick = onFix, enabled = !port.busy && input.text.isNotBlank()) { Text(port.go) }
            if (port.status.isNotEmpty()) DeviceHint(port.status)
        } else {
            OutlinedButton(onClick = { onInput(input.copy(open = true)) }) { Text("Change this port's address") }
        }
    }
}
