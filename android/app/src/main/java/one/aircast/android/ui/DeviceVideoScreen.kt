package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.R
import org.json.JSONObject
import java.io.File

@Composable
internal fun DeviceVideoScreen(props: DeviceScreenProps) {
    val view = deviceView("video", ::parseVideo) ?: return FootNote("Waiting for the device to answer…")
    view.radio?.let { RadioSection(it) }
    SharingSection(view.sharing)
    StatusLine("These apply to every camera on this device. Which camera goes where is set on the camera itself.")
    DetectionSection(view.detection)
    StatusLine(
        "Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. " +
            "Draw-a-box tracking works whichever model is active.",
    )
    SetupRow(title = view.engineering.title, subtitle = view.engineering.summary, icon = R.drawable.ic_terminal, onClick = { props.onOpen(view.engineering.href) })
}

@Composable
private fun RadioSection(radio: RadioSettings) {
    DeviceSection("Packet radio", radio.status) {
        DeviceText("The channel has to match the ground station exactly — a mismatch looks the same as a dead radio. Saving restarts the radio, so the link drops for a few seconds.")
        key("${radio.channel}|${radio.region}|${radio.txPower}") {
            DeviceForm(
                fields = listOf(
                    DeviceField("channel", "Channel", radio.channel, hint = "1-14 on 2.4 GHz, 32-177 on 5 GHz. 165 is 5825 MHz.", keyboard = KeyboardType.Number),
                    DeviceField("region", "Region", radio.region, hint = "Two-letter country code for the regulatory domain. BO permits the highest transmit power."),
                    DeviceField(
                        "tx_power",
                        "Transmit power",
                        radio.txPower,
                        placeholder = "driver default",
                        hint = "0-30. Leave empty for the driver default — the legal ceiling and the heat the adapter can shed are both airframe-specific.",
                        keyboard = KeyboardType.Number,
                    ),
                ),
                method = DeviceMethod.PUT,
                path = "/api/stream/wfb/radio",
                submit = "Save",
                success = "Radio settings applied.",
                confirm = ActionConfirm("Restart the radio?", "Change the radio and restart it? The link drops for a few seconds — do this on the ground.", "Restart"),
                note = "Restarts the radio.",
            )
        }
        DeviceDisclosure("Technical details") { DeviceText(radio.details) }
    }
}

@Composable
private fun SharingSection(sharing: PublicSharing) {
    DeviceSection("Public viewer (Cloudflare)", sharing.status) {
        when (sharing.stage) {
            SharingStage.Live -> {
                DeviceText("Public sharing is on. Every camera that publishes to Cloudflare shows a public share link — hand it to anyone, no login or VPN, and it survives the backend going down.")
                CopyRow("Bridge Worker", sharing.baseUrl)
                ButtonRow {
                    ActionButton(
                        label = "Turn off",
                        request = DeviceRequest(DeviceMethod.PUT, "/api/stream/cfbridge", JSONObject(SHARING_OFF)),
                        success = "Public sharing turned off.",
                        confirm = ActionConfirm("Turn off public sharing?", "Turn off public sharing? Existing share links stop working.", "Turn off"),
                    )
                }
            }
            SharingStage.Deploy -> {
                StepText(1, "Deploy the bridge to your Cloudflare account", "One click — Cloudflare copies the bridge into your account and creates its storage. Sign in when asked.")
                ButtonRow { LinkButton(sharing.deployUrl, "Deploy to Cloudflare", external = true) }
                StepText(2, "Paste these two keys when Cloudflare asks for secrets")
                CopyRow("SIGN_KEY", sharing.signKey, hint = "Signs share links (generated on this device).")
                CopyRow("DEVICE_TOKEN", sharing.deviceToken, hint = "Lets this device report the live session (generated on this device).")
                StatusLine("Your Cloudflare app secret is sent automatically by this device — you don't need to set it.")
                StepText(3, "Paste the Worker URL Cloudflare gives you")
                DeviceForm(
                    fields = listOf(DeviceField("baseUrl", "Worker URL", sharing.baseUrl, placeholder = "https://aircast-cf-bridge.<you>.workers.dev", keyboard = KeyboardType.Uri)),
                    method = DeviceMethod.PUT,
                    path = "/api/stream/cfbridge",
                    submit = "Connect",
                    success = "Public sharing is on.",
                    extra = mapOf("signKey" to sharing.signKey, "deviceToken" to sharing.deviceToken),
                )
                ButtonRow {
                    ActionButton(
                        label = "Start over",
                        request = DeviceRequest(DeviceMethod.PUT, "/api/stream/cfbridge", JSONObject(SHARING_OFF)),
                        success = "Setup cleared.",
                        confirm = ActionConfirm("Start over?", "Discard the generated keys and start over?", "Discard"),
                        tonal = false,
                    )
                }
            }
            else -> {
                DeviceText(
                    "Give a camera that publishes to Cloudflare Realtime a public share link — anyone opens it in a browser, no login and no VPN, and it keeps working even if the cloud backend is down. " +
                        "You host the bridge in your own Cloudflare account (free tier is plenty).",
                )
                ButtonRow { ActionButton("Set up public sharing", DeviceRequest(DeviceMethod.POST, "/api/stream/cfbridge/keys"), "Keys generated — follow the deploy steps.") }
            }
        }
    }
}

@Composable
private fun DetectionSection(detection: DetectionSettings) {
    DeviceSection("Detection model") {
        StackedFact("In use", detection.status.text)
        if (detection.unavailable) {
            DeviceNotice(DeviceBadge(DeviceLevel.Warn, "Detection is unavailable — this device has no AI accelerator, so the models below can't run. Draw-a-box tracking still works on every camera."))
        }
        if (detection.models.isEmpty()) DeviceText("No models installed yet.") else detection.models.map { model -> key(model.name) { ModelRow(model) } }
        DeviceDisclosure("Add a model", summary = "A .hef compiled for Hailo-8, with its class labels.") { ModelUploadForm() }
    }
}

@Composable
private fun ModelRow(model: DetectModel) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).semantics { contentDescription = model.name },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(model.name, style = MaterialTheme.typography.bodyLarge)
            if (model.about.isNotEmpty()) Text(model.about, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(model.size, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                DeviceBadgeText(model.classes)
            }
        }
        if (model.active) {
            DeviceBadgeText(DeviceBadge(DeviceLevel.Ready, "Active"))
        } else {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                ActionButton("Use", DeviceRequest(DeviceMethod.PUT, "/api/stream/detect/settings", JSONObject(mapOf("hef" to model.path))), "Model switched — detection restarts on the new model.")
                ActionButton(
                    label = "Delete",
                    request = DeviceRequest(DeviceMethod.DELETE, modelPath(model.name)),
                    success = "Model deleted.",
                    confirm = ActionConfirm("Delete ${model.name}?", "Delete ${model.name}? Anyone relying on it loses it from this device.", "Delete"),
                    tonal = false,
                )
            }
        }
    }
}

private fun staged(context: Context, uri: Uri): File? = runCatching {
    val name = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }.orEmpty().replace('/', '_')
    File(context.cacheDir, name).takeIf { target -> context.contentResolver.openInputStream(uri)?.use { source -> target.outputStream().use { source.copyTo(it) } } != null }
}.getOrNull()

@Composable
private fun FilePick(label: String, description: String, file: File?, onPick: (File?) -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch { onPick(withContext(Dispatchers.IO) { staged(context, chosen) }) }
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedButton(onClick = { picker.launch(arrayOf("*/*")) }, modifier = Modifier.semantics { contentDescription = description }) { Text(file?.name ?: "Choose a file") }
    }
}

@Composable
private fun ModelUploadForm() {
    val notice = LocalChangeNotice.current
    val scope = rememberCoroutineScope()
    var hef by remember { mutableStateOf<File?>(null) }
    var labels by remember { mutableStateOf<File?>(null) }
    var about by remember { mutableStateOf("") }
    var use by remember { mutableStateOf(true) }
    var busy by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<DeviceRefusal?>(null) }
    val upload = { file: File ->
        busy = true
        refusal = null
        scope.launch {
            val outcome = uploadModel(ModelUpload(file, labels, about, use))
            busy = false
            refusal = outcome.refusal
            if (outcome.message.isNotEmpty()) notice.show(outcome.message, null)
        }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        FilePick("Model (.hef, compiled for Hailo-8)", "Model file", hef) { hef = it }
        FilePick("Class labels (.labels, one per line — optional). Without labels, boxes are named by class number.", "Class labels file", labels) { labels = it }
        OutlinedTextField(
            value = about,
            onValueChange = { about = it },
            label = { Text("What it's for (shown next to the model — optional)") },
            placeholder = { Text("Aerial — vehicles at 400 ft") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Checkbox(checked = use, onCheckedChange = { use = it }, modifier = Modifier.semantics { contentDescription = "Switch to this model after upload" })
            Text("Switch to this model after upload", style = MaterialTheme.typography.bodyMedium)
        }
        RefusalText(refusal)
        FilledTonalButton(onClick = { hef?.let(upload) ?: run { refusal = DeviceRefusal("Choose a .hef model file.", "") } }, enabled = !busy) {
            Text(if (busy) "Uploading…" else "Upload model")
        }
    }
}
