package one.aircast.android.ui

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceEngineeringScreen(props: DeviceScreenProps) {
    val view = deviceView("engineering", ::parseEngineering) ?: return FootNote("Waiting for the device to answer…")
    BackendSection(view.backend)
    PublishSection(view.publish, props.onOpen)
    DeviceSection("Device logs") {
        DeviceText("Everything every service on this device has logged, gzipped.")
        ButtonRow { LinkButton(deviceUrl("/api/logs"), "Download logs", icon = R.drawable.ic_download) }
    }
    DiagnosticsSection(view.diagnostics)
}

@Composable
private fun BackendSection(backend: BackendChoice) {
    DeviceSection("What carries video", backend.status) {
        StatusLine("aircastd's own engine, or mediamtx as a separate service — switching restarts the device.")
        DeviceText(backend.text)
        ButtonRow {
            ActionButton(
                label = backend.label,
                request = DeviceRequest(DeviceMethod.PUT, "/api/stream/media-backend", JSONObject(mapOf("backend" to backend.switchTo))),
                success = backend.success,
                confirm = ActionConfirm("Restart the device?", backend.confirm, "Switch"),
            )
        }
        DeviceDisclosure("Technical details") {
            DeviceText(
                "Both backends bind the same ports — 8554, 8888, 8889, 1935, 8890 — so only one can run, and the choice is read once at startup. Switching writes the setting and restarts aircastd into it; " +
                    "if the restart fails the setting still stands and the next boot lands on it. Camera paths carry across in both directions. Only the engine can send a camera to a wfb-ng packet radio.",
            )
        }
    }
}

@Composable
private fun PublishSection(publish: PublishSignIn, onOpen: (String) -> Unit) {
    DeviceSection("Publishing sign-in", publish.status) {
        DeviceText(
            "Devices on the Aircast cloud sign in automatically once connected to a cloud account. Custom servers are set up under Advanced; publishing to a specific server — LiveKit, Cloudflare — " +
                "is configured per camera on the Cameras page.",
        )
        if (publish.awaitingCloud) {
            ButtonRow { FilledTonalButton(onClick = { onOpen("/settings/device") }) { Text("Connect the cloud account") } }
            StatusLine("Nothing to fill in here — the sign-in arrives with the cloud account.")
        }
        ButtonRow { TextButton(onClick = { onOpen("/streaming") }) { Text("Choose where cameras publish") } }
        DeviceDisclosure("Advanced · sign-in credentials", summary = "How this device signs in to video servers.") {
            key(signInKey(publish)) { SignInEditor(publish) }
        }
    }
}

@Composable
private fun SignInInput(field: SignInField, value: String, refusal: DeviceRefusal?, onValueChange: (String) -> Unit) {
    val refused = refusal?.field == field.name
    val supporting = if (refused) refusal?.message.orEmpty() else ""
    if (field.secret) {
        SecretField(field.label, value, onValueChange, placeholder = field.placeholder, isError = refused, supportingText = supporting)
    } else {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            label = { Text(field.label) },
            placeholder = field.placeholder.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
            singleLine = true,
            isError = refused,
            supportingText = supporting.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun SignInEditor(publish: PublishSignIn) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var mode by remember { mutableStateOf(publish.mode) }
    var edits by remember { mutableStateOf(emptyMap<String, String>()) }
    var asking by remember { mutableStateOf(false) }
    val picked = modeNamed(publish, mode)
    val dirty = mode != publish.mode
    val confirm = if (dirty) picked?.confirm.orEmpty() else ""
    val fields = signInFields(publish, mode)
    val save = { scope.launch { action.run(DeviceRequest(DeviceMethod.PUT, "/api/stream/sfu/oauth", JSONObject(signInBody(publish, mode, edits))), "Publishing sign-in saved.") } }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.selectableGroup().semantics { contentDescription = "Sign-in method" }, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            publish.modes.map { option ->
                key(option.value) {
                    Row(
                        Modifier.fillMaxWidth().border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(12.dp))
                            .selectable(selected = option.value == mode, role = Role.RadioButton) { mode = option.value }.padding(12.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        RadioButton(selected = option.value == mode, onClick = null)
                        Column(Modifier.weight(1f)) {
                            Text(option.title, style = MaterialTheme.typography.bodyLarge)
                            Text(option.desc, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
        }
        if (mode == "aircast" && publish.environment.isNotEmpty()) {
            Column {
                Text("Environment", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(publish.environment, style = MaterialTheme.typography.bodyMedium, fontFamily = FontFamily.Monospace)
            }
        }
        fields.map { field ->
            key(field.name) { SignInInput(field, edits[field.name] ?: field.value, action.refusal) { edits = edits + (field.name to it) } }
        }
        when {
            dirty -> Text(picked?.unsaved.orEmpty(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            publish.verdict.level == DeviceLevel.Blocked -> DeviceNotice(publish.verdict)
            else -> Text(publish.verdict.text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        action.refusal?.takeIf { refusal -> fields.none { it.name == refusal.field } }?.let { RefusalText(it, it.field) }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Stored on the device only.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            FilledTonalButton(onClick = { if (confirm.isEmpty()) save() else asking = true }, enabled = !action.busy) { Text(picked?.submit ?: "Save") }
        }
    }
    if (asking) ConfirmDialog(ActionConfirm("Clear the sign-in?", confirm, "Clear"), { save() }) { asking = false }
}

@Composable
private fun DiagnosticsSection(diagnostics: DiagnosticsSettings) {
    DeviceSection("Remote diagnostics", diagnostics.status) {
        DeviceText(diagnostics.hint)
        key("${diagnostics.endpoint}|${diagnostics.token}") { DiagnosticsForm(diagnostics) }
        DeviceSwitchRow(
            title = "Stream this device's logs",
            checked = diagnostics.streaming,
            on = DeviceRequest(DeviceMethod.PUT, "/api/diagnostics", JSONObject(mapOf("enabled" to "true"))),
            off = DeviceRequest(DeviceMethod.PUT, "/api/diagnostics", JSONObject(mapOf("enabled" to "false"))),
            success = "Streaming logs.",
            offSuccess = "Stopped streaming logs.",
            enabled = diagnostics.canStream,
            confirmOn = ActionConfirm("Stream the logs?", diagnostics.confirm, "Stream"),
        )
        StatusLine(diagnostics.state)
        StatusLine("Ships this device's logs to an endpoint you name, so support can read them without the device in front of them.")
    }
}

@Composable
private fun DiagnosticsForm(diagnostics: DiagnosticsSettings) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var endpoint by remember { mutableStateOf(diagnostics.endpoint) }
    var token by remember { mutableStateOf(diagnostics.token) }
    val refused = { field: String -> action.refusal?.field == field }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).semantics { contentDescription = "Diagnostics endpoint" }, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        OutlinedTextField(
            value = endpoint,
            onValueChange = { endpoint = it },
            label = { Text("OTLP endpoint (host:port)") },
            placeholder = { Text("otel.example.com:443") },
            singleLine = true,
            isError = refused("endpoint"),
            supportingText = action.refusal?.takeIf { refused("endpoint") }?.let { { Text(it.message) } },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth(),
        )
        SecretField(
            diagnostics.tokenLabel,
            token,
            { token = it },
            placeholder = "none",
            isError = refused("token"),
            supportingText = action.refusal?.takeIf { refused("token") }?.message.orEmpty(),
        )
        action.refusal?.takeIf { !refused("endpoint") && !refused("token") }?.let { RefusalText(it, it.field) }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Stored on the device only.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            FilledTonalButton(
                onClick = { scope.launch { action.run(DeviceRequest(DeviceMethod.POST, "/api/diagnostics/config", JSONObject(mapOf("endpoint" to endpoint, "token" to token))), "Diagnostics endpoint saved.") } },
                enabled = !action.busy,
            ) { Text("Save") }
        }
    }
}
