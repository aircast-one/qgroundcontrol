package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.R
import one.aircast.map.aircast

@Composable
internal fun DeviceConnectScreen(props: DeviceScreenProps) {
    val context = LocalContext.current
    var saved by remember { mutableStateOf(readStored(context, GCS_KEY)) }
    val view = deviceView("connect", ::parseConnect) ?: return FootNote("Waiting for the device to answer…")
    val pick = { value: String ->
        writeStored(context, GCS_KEY, value)
        saved = value
    }
    view.noCamera?.let { NoCamera(it, props.onOpen) } ?: ConnectLinks(view, chosenGcs(view.choices, saved), pick, props.onOpen)
}

@Composable
private fun NoCamera(add: PageLink, onOpen: (String) -> Unit) {
    DeviceSection("Cameras") {
        DeviceText("There is no camera on this device yet, so there is nothing to point a ground station at.")
        FilledTonalButton(onClick = { onOpen(add.href) }, modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) { Text(add.label) }
    }
}

@Composable
private fun rememberReach(url: String): Boolean? {
    var reach by remember(url) { mutableStateOf<Boolean?>(null) }
    var checks by remember(url) { mutableIntStateOf(0) }
    LaunchedEffect(url, checks) {
        if (!probeBlocked(url)) {
            reach = deviceReach(url)
            delay(REACH_RECHECK_MS)
            checks++
        }
    }
    return reach
}

@Composable
private fun ReachLine(reach: Boolean) {
    Text(
        if (reach) "This address answers from here" else "No answer from here — your ground station may not reach it either",
        style = MaterialTheme.typography.labelMedium,
        color = if (reach) MaterialTheme.aircast.success else MaterialTheme.aircast.warning,
        modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp),
    )
}

@Composable
private fun TargetRow(target: ConnectTarget) {
    val rule = MaterialTheme.colorScheme.outlineVariant
    val reach = rememberReach(target.reach)
    CopyRow(if (target.tag.isEmpty()) target.label else "${target.label} · ${target.tag}", target.url, target.hint, target.qr, if (target.down) DeviceLevel.Warn else DeviceLevel.Neutral)
    reach?.let { ReachLine(it) }
    if (target.alternatives.isNotEmpty()) {
        Column(Modifier.padding(start = 16.dp).drawBehind { drawLine(rule, Offset.Zero, Offset(0f, size.height), 2.dp.toPx()) }) {
            DeviceText("Doesn't resolve on Android or Windows? Use the address instead:")
            target.alternatives.map { alt -> key(alt.url) { CopyRow(alt.label, alt.url) } }
        }
    }
}

@Composable
private fun TargetRows(targets: List<ConnectTarget>, choice: GcsChoice?) {
    shownTargets(targets, choice).map { target -> key(target.url) { TargetRow(target) } }
}

@Composable
private fun GcsChooser(choices: List<GcsChoice>, choice: GcsChoice?, onPick: (String) -> Unit) {
    ButtonRow {
        choices.map { option ->
            key(option.value) {
                val selected = option.value == choice?.value
                FilterChip(
                    selected = selected,
                    onClick = { onPick(option.value) },
                    label = { Text(option.label) },
                    leadingIcon = if (selected) ({ Icon(painterResource(R.drawable.ic_check), null, Modifier.size(18.dp)) }) else null,
                )
            }
        }
    }
}

@Composable
private fun Subheading(text: String) {
    Text(text, style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 4.dp))
}

@Composable
private fun ConnectLinks(view: ConnectView, choice: GcsChoice?, onPick: (String) -> Unit, onOpen: (String) -> Unit) {
    DeviceSection("Your ground station") {
        GcsChooser(view.choices, choice, onPick)
        DeviceText("QGroundControl and Mission Planner play the RTSP link; a browser, OBS or Aircast QGC play WebRTC. This app remembers your answer.")
        DeviceText(view.scope)
        DeviceDisclosure("How to connect a ground station") {
            DeviceText("Each link below says where to paste it in your app; video and telemetry are two separate links.")
            DeviceText("Ground station on another device? Scan that link's QR code from there — no retyping.")
        }
    }
    view.cameras.map { camera ->
        key(camera.name) {
            DeviceSection(camera.name, camera.status) {
                if (camera.notice.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Warn, camera.notice))
                camera.fix?.let { fix -> FilledTonalButton(onClick = { onOpen(fix.href) }, modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) { Text(fix.label) } }
                TargetRows(camera.targets, choice)
            }
        }
    }
    view.telemetry?.let { telemetry ->
        DeviceSection("Telemetry", telemetry.status) {
            DeviceText("Video and telemetry are separate links — a ground station needs both.")
            TargetRows(telemetry.targets, choice)
        }
    }
    view.trouble?.let { TroubleSection(it, choice) }
}

@Composable
private fun TroubleSection(trouble: ConnectTrouble, choice: GcsChoice?) {
    DeviceSection("Troubleshooting") {
        DeviceDisclosure("This link didn't play in my app", "Reach the device by address, or fall back to a player that takes anything.") {
            if (trouble.fallback.isNotEmpty()) {
                Subheading("When nothing else will play")
                TargetRows(trouble.fallback, choice)
            }
            trouble.addresses?.let { addresses ->
                Subheading(addresses.title)
                DeviceText(
                    "Windows without Bonjour, and some networks, cannot look up a .local name — and a tailnet name needs Tailscale running there too. " +
                        "These links use this device's address instead: pick the one on the same network as your ground station.",
                )
                addresses.groups.map { group ->
                    key(group.label) {
                        Text(group.label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp))
                        TargetRows(group.targets, choice)
                    }
                }
            }
        }
    }
}
