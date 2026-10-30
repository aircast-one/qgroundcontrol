package one.aircast.android.ui

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.json.JSONObject

private val DEST_SHAPE = RoundedCornerShape(12.dp)

@Composable
internal fun DeviceCameraSetupScreen(props: DeviceScreenProps) {
    val view = deviceView("cameras", ::parseCameras)
    val camera = view?.cameras?.firstOrNull { it.name == props.query["camera"].orEmpty() }
    when {
        view == null -> FootNote("Waiting for the device to answer…")
        camera == null -> {
            DeviceNotice(DeviceBadge(DeviceLevel.Neutral, "Pick a camera on the Cameras page to set it up."))
            TextButton(onClick = { props.onOpen("/streaming") }, modifier = Modifier.padding(horizontal = 8.dp)) { Text("Cameras →") }
        }
        else -> key(camera.name) { CameraSetupParts(camera, openedSection(props.href), props.onOpen, props.onOpenFrom, props.onClose) }
    }
}

@Composable
private fun Sub(text: String) {
    Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp, bottom = 4.dp))
}

@Composable
private fun CameraSetupParts(camera: CameraView, opened: String, onOpen: (String) -> Unit, onOpenFrom: (String, String) -> Unit, onClose: () -> Unit) {
    val name = camera.name
    val setup = camera.setup
    PartDisclosure(setup.watch.part, CameraSetupSection.Watch, opened) { WatchBody(camera, onOpen) }
    PartDisclosure(setup.radio.part, CameraSetupSection.Radio, opened) {
        LinkedNoticeView(setup.radio.notice, onOpen)
        if (setup.radio.form) {
            key(setup.radio.address) {
                DeviceForm(
                    fields = listOf(
                        DeviceField(
                            name = "address",
                            label = "Address wfb_tx reads from",
                            value = setup.radio.address,
                            keyboard = KeyboardType.Uri,
                            hint = "The default is where a stock wfb-ng install reads from.",
                        ),
                    ),
                    method = DeviceMethod.PUT,
                    path = "/api/streams/$name/wfb",
                    submit = "Save",
                    success = "$name is now going over the radio.",
                    note = "Takes effect immediately — no restart.",
                )
            }
        }
        if (setup.radio.stop) {
            ButtonRow {
                ActionButton(
                    label = "Stop",
                    tonal = false,
                    request = DeviceRequest(DeviceMethod.DELETE, "/api/streams/$name/wfb"),
                    success = "Stopped sending $name over the radio.",
                    confirm = ActionConfirm("Stop the radio", "Stop sending $name over the radio link?", "Stop"),
                )
            }
        }
    }
    PartDisclosure(setup.servers.part, CameraSetupSection.Servers, opened) {
        Column(Modifier.padding(horizontal = 16.dp).fillMaxWidth().clip(DEST_SHAPE).border(1.dp, MaterialTheme.colorScheme.outlineVariant, DEST_SHAPE)) {
            setup.servers.dests.map { dest ->
                key(dest.kind) { PartDisclosure(dest.part, "cam-dest-${dest.kind}", opened) { ServerBody(camera, dest, onOpen) } }
            }
        }
    }
    PartDisclosure(setup.shares.part, CameraSetupSection.Shares, opened) {
        setup.shares.links.map { link -> key(link.id) { ShareRow(link) } }
        DeviceForm(
            fields = listOf(
                DeviceField(name = "label", label = "Who is it for", value = "", placeholder = "e.g. command post"),
                DeviceField(name = "hours", label = "Stops working after", value = setup.shares.hours.firstOrNull()?.value ?: "24", options = setup.shares.hours),
            ),
            method = DeviceMethod.POST,
            path = "/api/share",
            submit = "Create link",
            success = "Share link created.",
            note = "The link opens on a page that is the picture and nothing else — no settings, no addresses.",
            extra = mapOf("camera" to name),
        )
        DeviceText("A link stops working when it expires or when you turn it off. It is not a lock: anyone who can already reach this device on your network can watch the camera anyway.")
    }
    PartDisclosure(setup.detect.part, CameraSetupSection.Detect, opened) {
        val detectHere = cameraSectionHref(name, CameraSetupSection.Detect)
        DeviceSwitchRow(
            title = "Draw detection boxes for $name",
            checked = setup.detect.on,
            on = DeviceRequest(DeviceMethod.PUT, "/api/streams/$name/detect"),
            off = DeviceRequest(DeviceMethod.DELETE, "/api/streams/$name/detect"),
            success = "Detection turned on for $name.",
            offSuccess = "Detection turned off for $name.",
        )
        DeviceText(
            "Draw object-detection boxes over this camera. The video is untouched — boxes are sent as data and drawn by viewers, so there's no added latency. " +
                "The model, and whether it runs on the CPU or the Hailo-8, is set on the Detection page.",
        )
        if (setup.detect.model.isNotEmpty()) NoteLink(setup.detect.model, "Change model →", MaterialTheme.typography.bodyMedium) { onOpenFrom("/settings/detection", detectHere) }
        setup.detect.notice?.let { notice -> LinkedNoticeView(notice) { onOpenFrom(it, detectHere) } }
    }
    PartDisclosure(setup.record.part, CameraSetupSection.Record, opened) {
        DeviceSwitchRow(
            title = "Record $name to disk",
            checked = setup.record.on,
            on = DeviceRequest(DeviceMethod.PUT, "/api/streams/$name/record"),
            off = DeviceRequest(DeviceMethod.DELETE, "/api/streams/$name/record"),
            success = "Recording turned on for $name.",
            offSuccess = "Recording turned off for $name — download it under Settings → Recordings.",
        )
        DeviceText("Recorded in ten-minute segments to this device's storage, and playable as soon as a segment closes. The oldest go when the size limit is reached, so recording can never fill the card and take the device down with it.")
        if (setup.record.full) DeviceNotice(DeviceBadge(DeviceLevel.Warn, "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones."))
        if (setup.record.usage.isNotEmpty()) NoteLink(setup.record.usage, "Open recordings →", MaterialTheme.typography.bodyMedium) { onOpen("/settings/recordings") }
    }
    setup.gimbal?.let { gimbal ->
        PartDisclosure(gimbal.part, CameraSetupSection.Gimbal, opened) {
            if (gimbal.owner) {
                DeviceText("Tracking from QGroundControl steers the gimbal from this camera's picture. Only one camera can be on the gimbal — naming another one moves tracking there.")
                ButtonRow { ActionButton("Not this camera", DeviceRequest(DeviceMethod.DELETE, "/api/streams/$name/gimbal/owner"), "Tracking turned off for $name.", tonal = false) }
            } else {
                DeviceText("This device has a gimbal, but it is not this camera's, so QGroundControl cannot track from this picture. Say it is the gimbal camera to track from it.")
                ButtonRow { ActionButton("This is the gimbal camera", DeviceRequest(DeviceMethod.PUT, "/api/streams/$name/gimbal/owner"), "$name is now the gimbal camera.") }
            }
        }
    }
    Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.End) {
        ActionButton(
            label = "Remove this camera",
            tonal = false,
            request = DeviceRequest(DeviceMethod.DELETE, "/api/streams/$name"),
            success = "Camera $name removed.",
            confirm = ActionConfirm("Remove camera", "Remove camera $name? Its stream URLs stop working, and once no other camera uses its port the port returns to the rescue network.", "Remove"),
            onDone = onClose,
        )
    }
}

@Composable
private fun WatchBody(camera: CameraView, onOpen: (String) -> Unit) {
    val name = camera.name
    val watch = camera.setup.watch
    Sub("From anywhere")
    when {
        watch.cloud -> {
            EndpointRows(watch.cloudLinks)
            DeviceText(watch.cloudCopy)
            if (watch.signIn) {
                LinkedNoticeView(
                    LinkedNotice(DeviceLevel.Warn, "Add cloud credentials in Engineering → Publishing sign-in to finish authorizing the stream.", NoticeLink("Engineering", "/settings/engineering")),
                    onOpen,
                )
            }
            ButtonRow {
                ActionButton(
                    label = "Disconnect",
                    tonal = false,
                    request = DeviceRequest(DeviceMethod.DELETE, "/api/stream/sfu/$name/cloud"),
                    success = "$name disconnected from the cloud.",
                    confirm = ActionConfirm(
                        "Disconnect from the cloud",
                        "Disconnect $name from the Aircast cloud? Its cloud watch page stops working. Share links keep working on this network.",
                        "Disconnect",
                    ),
                )
            }
        }
        watch.canConnect -> {
            DeviceText("Optional: a watch page on the Aircast cloud, reachable from outside this network.")
            ButtonRow { ActionButton("Connect to cloud", DeviceRequest(DeviceMethod.POST, "/api/stream/sfu/$name/cloud"), "Connecting $name to the Aircast cloud…") }
        }
        else -> LinkedNoticeView(LinkedNotice(DeviceLevel.Neutral, "Needs a cloud account on this device — Device → Cloud account.", NoticeLink("Device", "/settings/device")), onOpen)
    }
    Sub("On this network")
    EndpointRows(watch.links)
    NoteLink("Didn't play, or the ground station is another machine?", "Ground station links →") { onOpen("/connect") }
}

private val STOP_LABELS = mapOf("whip" to "the SFU", "livekit" to "LiveKit", "cloudflare" to "Cloudflare")

@Composable
private fun ServerBody(camera: CameraView, dest: ServerDest, onOpen: (String) -> Unit) {
    val name = camera.name
    val path = "/api/stream/sfu/$name/${dest.kind}"
    val label = STOP_LABELS[dest.kind] ?: dest.kind
    when (dest.kind) {
        "whip" -> {
            key(dest.url) {
                DeviceForm(
                    fields = listOf(DeviceField(name = "url", label = "WHIP URL", value = dest.url, placeholder = "https://sfu.example.com/whip/device", keyboard = KeyboardType.Uri)),
                    method = DeviceMethod.PUT,
                    path = path,
                    submit = "Save",
                    success = "Publishing $name to the SFU.",
                )
            }
            NoteLink("Signs in with the device's sign-in method.", "Publishing sign-in →") { onOpen("/settings/engineering") }
        }
        "livekit" -> key(dest.url, dest.key, dest.secret) {
            DeviceForm(
                fields = listOf(
                    DeviceField(name = "url", label = "Server URL", value = dest.url, placeholder = "wss://livekit.example.com", keyboard = KeyboardType.Uri, hint = "The room is this camera's name."),
                    DeviceField(name = "livekitKey", label = "API key", value = dest.key, placeholder = "APIxxxxxxxx"),
                    DeviceField(name = "livekitSecret", label = "API secret", value = dest.secret, placeholder = "LiveKit API secret", keyboard = KeyboardType.Password),
                ),
                method = DeviceMethod.PUT,
                path = path,
                submit = "Save",
                success = "Publishing $name to LiveKit.",
                note = "One key pair is shared by every camera published to LiveKit — changing it here changes it everywhere.",
            )
        }
        "cloudflare" -> key(dest.app, dest.track, dest.secret) { CloudflareForm(name, dest) }
    }
    if (dest.on) {
        ButtonRow {
            ActionButton(
                label = "Stop publishing",
                tonal = false,
                request = DeviceRequest(DeviceMethod.DELETE, path),
                success = "$name stopped publishing to $label.",
                confirm = ActionConfirm("Stop publishing", "Stop publishing $name to $label?", "Stop"),
            )
        }
    }
    if (dest.kind == "cloudflare" && dest.on) {
        if (dest.watch.isNotEmpty()) ButtonRow { LinkButton(deviceUrl(dest.watch), "▶ Watch via Cloudflare", LinkLook.Filled, external = true) }
        DeviceText(
            "This player pulls the stream back from Cloudflare — proof of what your viewers get. There's no public watch URL: a viewer app pulls track ${dest.track} from app ${dest.app} through Cloudflare's Sessions API, authorized with the app secret.",
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CloudflareForm(name: String, dest: ServerDest) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var app by remember { mutableStateOf(dest.app) }
    var secret by remember { mutableStateOf(dest.secret) }
    var track by remember { mutableStateOf(dest.track) }
    val urlRefusal = action.refusal?.takeIf { it.field == "url" }
    val secretRefusal = action.refusal?.takeIf { it.field == "secret" }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                "In the Cloudflare dashboard, open Realtime → SFU and create an app — it gives you the App ID and App secret.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.align(Alignment.CenterVertically),
            )
            LinkButton("https://dash.cloudflare.com/", "Cloudflare dashboard", LinkLook.Outlined, external = true)
        }
        OutlinedTextField(
            value = app,
            onValueChange = { app = it },
            label = { Text("App ID") },
            placeholder = { Text("Realtime → SFU → your app") },
            singleLine = true,
            isError = urlRefusal != null,
            supportingText = urlRefusal?.let { { Text(it.message) } },
            keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth(),
        )
        SecretField("App secret", secret, { secret = it }, placeholder = "Realtime app secret", isError = secretRefusal != null, supportingText = secretRefusal?.message.orEmpty())
        OutlinedTextField(
            value = track,
            onValueChange = { track = it },
            label = { Text("Track name") },
            singleLine = true,
            supportingText = { Text("Not from Cloudflare — a name you pick. Viewers subscribe to this stream by this name. The camera name works fine.") },
            keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth(),
        )
        RefusalText(action.refusal?.takeIf { urlRefusal == null && secretRefusal == null }, action.refusal?.field.orEmpty())
        FilledTonalButton(
            enabled = !action.busy,
            onClick = {
                scope.launch {
                    action.run(
                        DeviceRequest(DeviceMethod.PUT, "/api/stream/sfu/$name/cloudflare", JSONObject(mapOf("url" to cloudflareTarget(app, track), "secret" to secret))),
                        "Publishing $name to Cloudflare.",
                    )
                }
            },
        ) { Text("Save") }
    }
}

@Composable
private fun ShareRow(link: ShareLink) {
    if (link.dead.isEmpty()) {
        CopyRow(link.label, link.url, hint = link.expires, qr = true)
        ButtonRow {
            ActionButton(
                label = "Turn off",
                tonal = false,
                request = DeviceRequest(DeviceMethod.DELETE, "/api/share/${link.id}"),
                success = "Link turned off.",
                confirm = ActionConfirm("Turn off this link", "Turn off this link? Whoever has it stops being able to open the picture.", "Turn off"),
            )
        }
    } else {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).alpha(0.7f)) {
            Text(link.label, style = MaterialTheme.typography.bodyLarge)
            Text(link.dead, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
