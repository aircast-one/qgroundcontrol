package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal data class WifiNetwork(val ssid: String, val signal: Int, val secured: Boolean, val active: Boolean)

internal data class WifiReopen(val host: String, val hostname: String)

internal data class WifiJoinAttempt(val attempt: Int, val target: String, val status: String, val instructions: Boolean)

internal fun parseWifiScan(json: JSONObject?): List<WifiNetwork>? =
    json?.deviceObjects("networks")?.map { WifiNetwork(it.optText("ssid"), it.optInt("signal"), it.optBoolean("secured"), it.optBoolean("active")) }

internal fun wifiJoinProblem(ssid: String, psk: String, pickedSecured: String): DeviceRefusal? {
    val name = ssid.trim()
    return when {
        name.isEmpty() -> DeviceRefusal("Enter your network name first.", "ssid")
        psk.isEmpty() && pickedSecured.isNotEmpty() && pickedSecured == name -> DeviceRefusal("“$name” is a secured network — enter its password.", "psk")
        psk.isNotEmpty() && psk.length !in 8..63 -> DeviceRefusal("Password must be 8–63 characters, or blank for an open network.", "psk")
        else -> null
    }
}

internal fun joinOutcome(join: WifiJoinAttempt, refusal: DeviceRefusal?): WifiJoinAttempt = when {
    refusal == null -> join.copy(status = "✓ Connected to “${join.target}” — it's now the active link.")
    refusal === NO_ANSWER -> join.copy(status = "Lost contact with the device — it may be switching networks. If it doesn't come back, reconnect using the steps below.")
    else -> join.copy(instructions = false, status = "✗ ${refusal.message.ifEmpty { "Couldn't join that network." }}")
}

private val NO_JOIN = WifiJoinAttempt(0, "", "", false)

@Composable
internal fun WifiJoin(reopen: WifiReopen, initiallyOpen: Boolean = false) {
    DeviceDisclosure("Join a Wi-Fi network", "Connect this device to your Wi-Fi. It leaves the recovery hotspot and rejoins on every boot.", initiallyOpen = initiallyOpen) {
        WifiWizard(reopen)
    }
}

@Composable
private fun WifiWizard(reopen: WifiReopen) {
    val scope = rememberCoroutineScope()
    var joining by remember { mutableStateOf(false) }
    var join by remember { mutableStateOf(NO_JOIN) }
    var ssid by remember { mutableStateOf("") }
    var psk by remember { mutableStateOf("") }
    var pickedSecured by remember { mutableStateOf("") }
    var problem by remember { mutableStateOf<DeviceRefusal?>(null) }
    var scanning by remember { mutableStateOf(false) }
    var scanFailed by remember { mutableStateOf(false) }
    var networks by remember { mutableStateOf<List<WifiNetwork>?>(null) }
    val scan = {
        scanning = true
        scope.launch {
            val found = deviceRead("/api/network/wifi/scan", ::parseWifiScan)
            scanning = false
            scanFailed = found == null
            networks = found
        }
    }
    val pick = { network: WifiNetwork ->
        ssid = network.ssid
        psk = ""
        pickedSecured = if (network.secured) network.ssid else ""
    }
    val connect = {
        val refused = wifiJoinProblem(ssid, psk, pickedSecured)
        problem = refused
        if (refused == null) {
            val target = ssid.trim()
            val attempt = join.attempt + 1
            join = WifiJoinAttempt(attempt, target, "Switching to “$target”…", true)
            joining = true
            scope.launch {
                val refusal = deviceCall(DeviceMethod.POST, "/api/network/wifi", JSONObject().put("ssid", target).put("psk", psk))
                if (join.attempt == attempt) join = joinOutcome(join, refusal)
            }
        }
    }
    val back = {
        joining = false
        join = join.copy(attempt = join.attempt + 1)
    }
    if (joining) {
        WifiJoining(join, reopen, back)
        return
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Hint("Pick a nearby network or type its name. Scanning is limited while the hotspot is on, so typing is usually fastest.", Modifier.weight(1f))
            OutlinedButton(onClick = { scan() }, enabled = !scanning) { Text(if (scanning) "Scanning…" else "Scan") }
        }
        if (scanFailed) Hint("Scan failed — type your network name below.")
        networks?.let { found ->
            if (found.isEmpty()) Hint("No networks found — type your network name below.")
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                found.map { network -> key(network.ssid) { NetworkRow(network) { pick(network) } } }
            }
        }
        OutlinedTextField(
            value = ssid,
            onValueChange = { ssid = it },
            label = { Text("Network name (SSID)") },
            placeholder = { Text("your Wi-Fi name") },
            singleLine = true,
            isError = problem?.field == "ssid",
            keyboardOptions = KeyboardOptions(autoCorrectEnabled = false, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { connect() }),
            modifier = Modifier.fillMaxWidth(),
        )
        SecretField("Password", psk, { psk = it }, isError = problem?.field == "psk", supportingText = "Leave blank for an open network.")
        problem?.let { Text(it.message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error) }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Hint("Saved with autoconnect, ahead of the hotspot.", Modifier.weight(1f))
            FilledTonalButton(onClick = { connect() }) { Text("Connect") }
        }
    }
}

@Composable
private fun NetworkRow(network: WifiNetwork, onPick: () -> Unit) {
    val shape = RoundedCornerShape(12.dp)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 48.dp).clip(shape).border(1.dp, MaterialTheme.colorScheme.outlineVariant, shape).clickable(onClick = onPick).padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(network.ssid, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f, fill = false))
            if (network.active) Text("connected", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.aircast.success)
        }
        Hint("${network.signal}%")
        if (network.secured) Icon(painterResource(R.drawable.ic_lock), "secured", tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(18.dp))
    }
}

@Composable
private fun Hint(text: String, modifier: Modifier = Modifier) {
    Text(text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = modifier)
}

@Composable
private fun WifiJoining(join: WifiJoinAttempt, reopen: WifiReopen, onBack: () -> Unit) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(join.status, style = MaterialTheme.typography.bodyLarge)
        if (join.instructions) {
            Column(
                Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(horizontal = 16.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("The recovery hotspot is turning off. To reach the device again:", style = MaterialTheme.typography.bodyMedium, color = muted)
                Text(
                    buildAnnotatedString {
                        append("1. On your phone or laptop, join ")
                        withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface)) { append(join.target) }
                        append(".")
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = muted,
                )
                Text("2. Reconnect this app to the device on that network.", style = MaterialTheme.typography.bodyMedium, color = muted)
                ReconnectDevice(reopen.host, onBack)
                if (reopen.host.isNotEmpty()) {
                    Hint("Doesn't connect? Android can't always reach .local names — find ${reopen.hostname} in your router's list of connected devices and use its address instead.")
                }
                Hint("The app reaches the device over the link that's going away, so it can't follow it on its own.")
            }
        }
        OutlinedButton(onClick = onBack) { Text("← Back") }
    }
}

@Composable
private fun ReconnectDevice(host: String, onConnected: () -> Unit) {
    val scope = rememberCoroutineScope()
    var address by remember(host) { mutableStateOf(host) }
    var busy by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf("") }
    val reconnect = {
        busy = true
        scope.launch {
            refusal = DeviceLink.connect(address).orEmpty()
            busy = false
            if (refusal.isEmpty()) onConnected()
        }
    }
    OutlinedTextField(
        value = address,
        onValueChange = { address = it },
        label = { Text("Device address") },
        placeholder = { Text("192.168.1.100 or falcon.local") },
        singleLine = true,
        isError = refusal.isNotEmpty(),
        supportingText = refusal.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false, imeAction = ImeAction.Go),
        keyboardActions = KeyboardActions(onGo = { reconnect() }),
        modifier = Modifier.fillMaxWidth(),
    )
    FilledTonalButton(onClick = { reconnect() }, enabled = !busy && address.isNotBlank()) { Text("Reconnect") }
}
