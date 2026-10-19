package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRowDefaults
import androidx.compose.material3.TabRowDefaults.tabIndicatorOffset
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import one.aircast.android.R

@Composable
private fun AnnounceComebacks(view: CamerasView?) {
    val notice = LocalChangeNotice.current
    var seen by remember { mutableStateOf(emptyMap<String, Boolean>()) }
    LaunchedEffect(view) {
        val cameras = view?.cameras ?: return@LaunchedEffect
        val back = cameBack(seen, cameras)
        if (back.isNotEmpty()) notice.show("${back.joinToString(", ")} ${if (back.size == 1) "is" else "are"} back — sending video.", null)
        seen = liveness(cameras)
    }
}

@Composable
internal fun DeviceCamerasScreen(props: DeviceScreenProps) {
    val view = deviceView("cameras", ::parseCameras)
    var picked by rememberSaveable { mutableStateOf(props.query["camera"].orEmpty()) }
    var adding by rememberSaveable { mutableStateOf("add" in props.query) }
    AnnounceComebacks(view)
    LaunchedEffect(view) { if (view?.cameras?.isEmpty() == true) adding = true }
    val pick = { name: String ->
        adding = false
        picked = name
    }
    if (view == null) FootNote("Waiting for the device to answer…") else Cameras(view, camerasShown(view.cameras, picked, adding), props, { adding = true }, pick)
}

@Composable
private fun Cameras(view: CamerasView, shown: CamerasShown, props: DeviceScreenProps, onAdd: () -> Unit, onPick: (String) -> Unit) {
    val canAdd = shown != CamerasShown.Add
    if (view.cameras.isNotEmpty()) CameraTabs(view.cameras, (shown as? CamerasShown.Camera)?.camera, onPick)
    if (canAdd || view.cloudLink.isNotEmpty()) {
        Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            if (canAdd) TextButton(onClick = onAdd) { Text("+ Add camera") }
            Spacer(Modifier.weight(1f))
            if (view.cloudLink.isNotEmpty()) LinkButton(view.cloudLink, "Watch in the cloud", LinkLook.Outlined, external = true)
        }
    }
    when (shown) {
        is CamerasShown.Camera -> {
            key(shown.camera.name) { CameraDetail(shown.camera, "scan" in props.query && props.query["camera"] == shown.camera.name, props.onOpen) }
            TopicRow(shown.camera.topic, props.onOpen)
            TopicRow(view.video, props.onOpen)
        }
        CamerasShown.Wall -> {
            view.cameras.map { camera -> key(camera.name) { CameraTile(camera) { onPick(camera.name) } } }
            TopicRow(view.video, props.onOpen)
        }
        CamerasShown.Add -> DeviceSection("Add a camera") { CameraWizard(view.add, onDone = onPick) }
    }
}

@Composable
private fun CameraTabs(cameras: List<CameraView>, shown: CameraView?, onPick: (String) -> Unit) {
    val selected = cameras.indexOfFirst { it == shown }
    ScrollableTabRow(
        selectedTabIndex = selected.coerceAtLeast(0),
        indicator = { positions -> positions.getOrNull(selected)?.let { TabRowDefaults.SecondaryIndicator(Modifier.tabIndicatorOffset(it)) } },
    ) {
        cameras.map { camera ->
            key(camera.name) {
                Tab(
                    selected = camera == shown,
                    onClick = { onPick(camera.name) },
                    text = {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Box(Modifier.size(8.dp).background(deviceLevelColor(camera.status.level), CircleShape))
                            Text(camera.name)
                            Text(camera.status.text, style = MaterialTheme.typography.labelSmall, color = deviceLevelColor(camera.status.level))
                        }
                    },
                )
            }
        }
    }
}

@Composable
private fun TopicRow(page: DevicePage, onOpen: (String) -> Unit) {
    SetupRow(
        title = page.title,
        subtitle = page.summary,
        icon = devicePageIcon(page.icon),
        state = if (page.needs) SetupState.NeedsAttention else SetupState.Neutral,
        onClick = { onOpen(page.href) },
    )
}

@Composable
private fun CameraPicture(camera: CameraView) {
    if (camera.preview.isNotEmpty()) WhepPreview(deviceUrl(camera.preview), deviceUrl(camera.watch), camera.name) else PreviewPlaceholder(camera.placeholder)
}

@Composable
private fun CameraTile(camera: CameraView, onPick: () -> Unit) {
    Column(Modifier.fillMaxWidth().semantics { contentDescription = camera.name }) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(camera.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.clickable(onClick = onPick))
                DeviceBadgeText(camera.status)
                Spacer(Modifier.weight(1f))
                FilledTonalButton(onClick = onPick) { Text("Open") }
            }
            CameraPicture(camera)
            camera.diagnosis?.let { Text(it.text, style = MaterialTheme.typography.bodyMedium, color = deviceLevelColor(it.level)) }
        }
        DeviceSwitchRow(
            title = "Record to disk",
            checked = camera.recording,
            on = DeviceRequest(DeviceMethod.PUT, "/api/streams/${camera.name}/record"),
            off = DeviceRequest(DeviceMethod.DELETE, "/api/streams/${camera.name}/record"),
            success = "Recording turned on for ${camera.name}.",
            offSuccess = "Recording turned off for ${camera.name} — download it under Settings → Recordings.",
        )
    }
}

@Composable
private fun CameraDetail(camera: CameraView, scan: Boolean, onOpen: (String) -> Unit) {
    var wizard by rememberSaveable { mutableStateOf(scan.takeIf { it }) }
    when (val scanning = wizard) {
        null -> CameraOverview(camera, onOpen) { wizard = it }
        else -> DeviceSection("Set up ${camera.name}", camera.status) {
            CameraWizard(camera.wizard, scan = scanning, onCancel = { wizard = null }, onDone = { wizard = null })
        }
    }
}

@Composable
private fun CameraOverview(camera: CameraView, onOpen: (String) -> Unit, onWizard: (Boolean) -> Unit) {
    DeviceSection(camera.name, camera.status) {
        if (camera.shared) {
            Box(Modifier.padding(horizontal = 16.dp)) { DeviceBadgeText(DeviceBadge(DeviceLevel.Warn, "shared source")) }
            DeviceNotice(
                DeviceBadge(
                    DeviceLevel.Warn,
                    "Another camera pulls this same RTSP source. One camera fed to two pulls splits its video and audio across the streams — remove the duplicate camera for a stable feed.",
                ),
            )
        }
        Box(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
            Box(Modifier.widthIn(max = 576.dp)) { CameraPicture(camera) }
        }
        if (camera.live) {
            ButtonRow {
                LinkButton(deviceUrl(camera.watch), "▶ Watch live", LinkLook.Filled, external = true)
                camera.copies.map { copy -> key(copy.label) { CopyButton(copy) } }
            }
        }
        camera.facts.map { fact -> key(fact.label) { DeviceFact(fact.label, fact.value) } }
        camera.bitrate?.let { DeviceChart(it) }
        camera.diagnosis?.let { diagnosis ->
            DeviceNotice(diagnosis)
            camera.fix?.let { fix -> Button(onClick = { onWizard(fix.scan) }, modifier = Modifier.padding(horizontal = 16.dp)) { Text(fix.label) } }
            Text(
                "This page keeps checking — it updates by itself the moment the camera answers again.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
            )
        }
        if (camera.fix == null) {
            Box(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                if (camera.live) FilledTonalButton(onClick = { onWizard(false) }) { Text("Reconfigure") } else Button(onClick = { onWizard(true) }) { Text("Reconfigure") }
            }
        }
        if (camera.links.isNotEmpty()) {
            if (camera.live) {
                Column(Modifier.fillMaxWidth().semantics { contentDescription = "Every way to watch this camera" }) {
                    Text(
                        "Every way to watch this camera",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp),
                    )
                    EndpointRows(camera.links)
                    MoreWays(onOpen)
                }
            } else {
                DeviceDisclosure("Links — they will play once it's back") {
                    EndpointRows(camera.links)
                    MoreWays(onOpen)
                }
            }
        }
        if (camera.chips.isNotEmpty()) {
            ButtonRow {
                camera.chips.map { chip ->
                    key(chip.key) {
                        AssistChip(
                            onClick = { onOpen(chip.href) },
                            label = { Text(chip.label) },
                            leadingIcon = { Box(Modifier.size(8.dp).background(deviceLevelColor(chip.level), CircleShape)) },
                            trailingIcon = { Icon(painterResource(R.drawable.ic_arrow_forward), null, Modifier.size(16.dp)) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun MoreWays(onOpen: (String) -> Unit) {
    NoteLink("Didn't play, or the ground station is another machine?", "Ground station links →") { onOpen("/connect") }
}

@Composable
private fun CopyButton(copy: CameraCopy) {
    val doCopy = rememberCopy()
    var qr by remember { mutableStateOf(false) }
    val title = copy.label.removePrefix("Copy ")
    Row(verticalAlignment = Alignment.CenterVertically) {
        FilledTonalButton(onClick = { doCopy(copy.text) }) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Icon(painterResource(R.drawable.ic_content_copy), null, Modifier.size(18.dp))
                Text(copy.label)
            }
        }
        if (copy.qr) IconButton(onClick = { qr = true }) { Icon(painterResource(R.drawable.ic_qr_code_2), "Show $title as a QR code") }
    }
    if (qr) QrDialog(title, copy.text) { qr = false }
}
