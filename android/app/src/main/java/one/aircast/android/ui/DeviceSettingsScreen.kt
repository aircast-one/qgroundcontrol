package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceSettingsScreen(props: DeviceScreenProps) {
    val view = deviceView("device", ::parseDeviceSettings) ?: return FootNote("Waiting for the device to answer…")
    HealthLine(view.health, props.onOpen)
    view.notice?.let { DeviceNotice(it) }
    DeviceSection("Status") {
        view.stats.map { stat -> key(stat.label) { if (stat.label == SERVICES) ServicesStat(stat, view.services) else StatText(stat, Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) } }
    }
    CloudSection(view.cloud)
    SoftwareSection(view.software)
    DeviceSection("More devices") { ProvisionSteps() }
    ResetSection(view.hostname)
}

@Composable
private fun HealthLine(health: HealthBadge, onOpen: (String) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Box(Modifier.size(10.dp).background(deviceLevelColor(health.level), CircleShape))
        Text(health.text, style = MaterialTheme.typography.titleMedium, color = deviceLevelColor(health.level), modifier = Modifier.weight(1f))
        if (health.href.isNotEmpty()) FilledTonalButton(onClick = { onOpen(health.href) }) { Text("Check") }
    }
}

@Composable
private fun StatText(stat: DeviceStat, modifier: Modifier) {
    Column(modifier) {
        Text(stat.label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(stat.value, style = MaterialTheme.typography.titleMedium, color = if (stat.level == DeviceLevel.Neutral) MaterialTheme.colorScheme.onSurface else deviceLevelColor(stat.level))
        if (stat.hint.isNotEmpty()) Text(stat.hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun ServicesStat(stat: DeviceStat, services: List<ServiceRow>) {
    var open by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().clickable { open = !open }.padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        StatText(stat, Modifier.weight(1f))
        Text(if (open) "Hide" else "Show details", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
        Icon(if (open) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    if (open) {
        Column(Modifier.fillMaxWidth().padding(bottom = 8.dp).semantics { contentDescription = "Supervised units" }) {
            services.map { service ->
                key(service.label) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text(service.label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                        Text(service.state, style = MaterialTheme.typography.labelLarge, color = deviceLevelColor(service.level))
                    }
                }
            }
            StatusLine("Supervised by systemd — a unit that stops is restarted automatically.")
        }
    }
}

@Composable
private fun CloudSection(cloud: CloudAccount) {
    DeviceSection("Cloud account", cloud.status) {
        when (cloud.stage) {
            CloudStage.Linked -> {
                DeviceText("This device is linked to your Aircast cloud account. The ID below only matters if support asks for it.")
                CopyRow("Device ID (for support)", cloud.deviceId)
                if (cloud.watchUrl.isNotEmpty()) ButtonRow { LinkButton(cloud.watchUrl, "Watch in the cloud", look = LinkLook.Text, external = true) }
            }
            CloudStage.SigningIn -> {
                DeviceText("Open the sign-in page, log in (or sign up) with Google, and authorize this device. It connects automatically once you approve.")
                ButtonRow { LinkButton(cloud.signInUrl, "Open sign-in page", external = true) }
                CopyRow("Or go to the link and enter this code", cloud.userCode)
            }
            else -> key(cloud.issuer) { CloudConnect(cloud) }
        }
    }
}

@Composable
private fun CloudConnect(cloud: CloudAccount) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var issuer by remember { mutableStateOf(cloud.issuer) }
    val refusedIssuer = action.refusal?.field == "issuer"
    DeviceText("Not connected. Linking signs this device in to an Aircast cloud account; it is optional — every local and self-hosted path works without it.")
    if (cloud.error.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Warn, cloud.error))
    DeviceDisclosure("Advanced · environment", summary = "Which Aircast cloud to sign in to", initiallyOpen = refusedIssuer) {
        OutlinedTextField(
            value = issuer,
            onValueChange = { issuer = it },
            label = { Text("Cloud API") },
            placeholder = { Text("https://api.aircast.one") },
            singleLine = true,
            isError = refusedIssuer,
            supportingText = { Text("Leave this alone unless you were told to use a different Aircast environment.") },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        )
    }
    RefusalText(action.refusal, action.refusal?.field.orEmpty())
    ButtonRow {
        FilledTonalButton(
            onClick = { scope.launch { action.run(DeviceRequest(DeviceMethod.POST, "/api/cloud/connect", JSONObject(mapOf("issuer" to issuer))), "Sign-in started — the code appears here in a moment.") } },
            enabled = !action.busy,
        ) { Text("Connect") }
    }
}

@Composable
private fun SoftwareNoticeRow(notice: SoftwareNotice) {
    DeviceNotice(DeviceBadge(notice.level, if (notice.text.isEmpty()) notice.title else "${notice.title} ${notice.text}"))
    if (notice.detail.isNotEmpty()) {
        DeviceDisclosure(notice.detailLabel) {
            Text(
                notice.detail,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 8.dp).heightIn(max = 192.dp)
                    .background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(8.dp)).verticalScroll(rememberScrollState()).padding(12.dp),
            )
        }
    }
}

@Composable
private fun SoftwareSection(software: SoftwareStatus) {
    DeviceSection("Software", software.status) {
        StackedFact("Running", software.summary)
        software.notices.map { notice -> key(notice.title) { SoftwareNoticeRow(notice) } }
        software.candidate?.let { candidate ->
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(16.dp)
                    .semantics { contentDescription = "Available update" },
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(candidate.title, style = MaterialTheme.typography.titleMedium)
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    ActionButton(candidate.install, DeviceRequest(DeviceMethod.POST, "/api/update"), candidate.success, ActionConfirm("Install the update?", candidate.confirm, "Install"))
                    LinkButton(candidate.notesUrl, "What's new", look = LinkLook.Text, external = true)
                }
            }
        }
        software.notes.map { note -> key(note) { DeviceText(note) } }
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(software.checked, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            if (software.canCheck) ActionButton("Check now", DeviceRequest(DeviceMethod.POST, "/api/update/check"), "Checked for updates.", tonal = false)
        }
    }
}

private val PROVISION_STEPS = listOf(
    "Export this device's settings.",
    "Add your tailnet auth key to the file. It is not exported, because this page answers anyone who can reach the device.",
    "Copy the file onto the new card's boot partition as aircast-provision.json.",
    "Boot it. It joins, picks up the camera and the data caps, then deletes the file so the key does not stay on the card.",
)

@Composable
private fun ProvisionSteps() {
    DeviceDisclosure("Set up another device like this one", summary = "Export these settings for the next card.") {
        PROVISION_STEPS.mapIndexed { index, step ->
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("${index + 1}.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(step, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            }
        }
        DeviceText("The file carries the camera's own address, sign-in included — that is what makes it worth copying. Treat it like the password it contains.")
        ButtonRow { LinkButton(deviceUrl("/api/provision/export"), "Export these settings", icon = R.drawable.ic_download) }
        StatusLine("Airframes two through five are this one with a different serial.")
    }
}

@Composable
private fun ResetSection(hostname: String) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var asking by remember { mutableStateOf(false) }
    DeviceSection("Reset device") {
        DeviceText("Reboots and reverts the device to factory settings — camera, telemetry and all Wi-Fi and added network settings are wiped, and the setup wizard runs again. Tailscale stays connected so the device remains reachable.")
        ButtonRow {
            FilledTonalButton(
                onClick = { asking = true },
                enabled = !action.busy,
                colors = ButtonDefaults.filledTonalButtonColors(containerColor = MaterialTheme.colorScheme.errorContainer, contentColor = MaterialTheme.colorScheme.onErrorContainer),
            ) { Text("Reset to defaults") }
        }
        RefusalText(action.refusal, action.refusal?.field.orEmpty())
    }
    if (asking) {
        TypedConfirmDialog(
            title = "Reset to factory settings?",
            text = "It reboots and wipes the camera, telemetry and all Wi-Fi and added network settings, then runs setup again. Tailscale is preserved.",
            label = "Type $hostname to confirm",
            expected = hostname,
            confirm = "Reset",
            onConfirm = { typed ->
                scope.launch {
                    action.run(DeviceRequest(DeviceMethod.POST, "/api/setup/reset", JSONObject(mapOf("confirm" to typed))), "Resetting — the device is rebooting to factory settings; reconnect in a minute.")
                }
            },
            onDismiss = { asking = false },
        )
    }
}
