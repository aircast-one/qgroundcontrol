package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val ESP_BRIDGE_VIEW = "view.espBridge"
internal const val ESP_BRIDGE_SCREEN = "espBridge"
private const val ESP_POLL_MS = 1000L
private val WIFI_MODES = listOf("Access point mode", "Station mode")

internal data class LinkCounts(val received: String, val lost: String, val sent: String)

internal data class EspBridge(
    val modeIndex: Int?,
    val modePath: String,
    val channel: Int,
    val channelPath: String,
    val ssid: String,
    val password: String,
    val ssidSta: String?,
    val passwordSta: String?,
    val baudRates: List<Long>,
    val baudIndex: Int,
    val hostPort: String,
    val vehicle: LinkCounts,
    val bridge: LinkCounts,
    val qgc: LinkCounts,
    val rebootPrompt: String,
)

internal fun grouped(value: Any?): String = (value as? Number)?.toLong()?.let { "%,d".format(java.util.Locale.ROOT, it) } ?: ""

private fun counts(json: JSONObject?): LinkCounts = LinkCounts(grouped(json?.opt("received")), grouped(json?.opt("lost")), grouped(json?.opt("sent")))

internal fun espBridge(view: JSONObject?): EspBridge? = view?.takeIf { it.optBoolean("available") }?.let {
    val bauds = it.optJSONArray("baudRates")
    val status = it.optJSONObject("status")
    EspBridge(
        modeIndex = if (it.isNull("modeIndex")) null else it.optInt("modeIndex"),
        modePath = it.optText("modePath"),
        channel = it.optInt("channel", 1),
        channelPath = it.optText("channelPath"),
        ssid = it.optText("ssid"),
        password = it.optText("password"),
        ssidSta = if (it.isNull("ssidSta")) null else it.optText("ssidSta"),
        passwordSta = if (it.isNull("passwordSta")) null else it.optText("passwordSta"),
        baudRates = (0 until (bauds?.length() ?: 0)).map { at -> bauds!!.optLong(at) },
        baudIndex = it.optInt("baudIndex", 4),
        hostPort = it.optJSONObject("hostPort")?.optText("valueString").orEmpty(),
        vehicle = counts(status?.optJSONObject("vehicle")),
        bridge = counts(status?.optJSONObject("bridge")),
        qgc = counts(status?.optJSONObject("qgc")),
        rebootPrompt = it.optText("rebootPrompt"),
    )
}

@Composable
fun EspBridgeScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<EspBridge?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var confirmReboot by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { espBridge(Qgc.get(ESP_BRIDGE_VIEW)) }
        delay(ESP_POLL_MS)
        revision++
    }
    fun act(path: String, vararg args: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) } }
    }
    fun set(path: String, value: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(path, value) } }
    }
    val bridge = read ?: run {
        Text("No WiFi bridge is connected.", modifier.padding(16.dp))
        return
    }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SectionHeader("ESP Wi-Fi bridge settings")
        bridge.modeIndex?.let { mode -> Choice("Wi-Fi mode", WIFI_MODES, mode) { set("${bridge.modePath}.rawValue", it) } }
        Choice("Wi-Fi channel", (1..11).map { it.toString() }, bridge.channel - 1, enabled = (bridge.modeIndex ?: 0) == 0) { set("${bridge.channelPath}.rawValue", it + 1) }
        TextSetting("Wi-Fi AP SSID", bridge.ssid) { act("espBridge.setText", "ssid", it) }
        TextSetting("Wi-Fi AP password", bridge.password) { act("espBridge.setText", "password", it) }
        bridge.ssidSta?.let { value -> TextSetting("Wi-Fi STA SSID", value) { act("espBridge.setText", "ssidSta", it) } }
        bridge.passwordSta?.let { value -> TextSetting("Wi-Fi STA password", value) { act("espBridge.setText", "passwordSta", it) } }
        Choice("UART baud rate", bridge.baudRates.map { it.toString() }, bridge.baudIndex) { act("espBridge.baud", it) }
        StatusRow("QGC UDP port", bridge.hostPort)

        SectionHeader("ESP Wi-Fi bridge status")
        listOf("Bridge to vehicle link" to bridge.vehicle, "Bridge to QGC link" to bridge.bridge, "QGC to bridge link" to bridge.qgc).forEach { (title, link) ->
            Text(title, style = MaterialTheme.typography.titleSmall)
            StatusRow("Messages received", link.received)
            StatusRow("Messages lost", link.lost)
            StatusRow("Messages sent", link.sent)
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = { act("espBridge.restoreDefaults") }) { Text("Restore defaults") }
            OutlinedButton(onClick = { confirmReboot = true }) { Text("Restart Wi-Fi bridge") }
            OutlinedButton(onClick = { act("espBridge.resetCounters") }) { Text("Reset counters") }
        }
    }
    if (confirmReboot) {
        AlertDialog(
            onDismissRequest = { confirmReboot = false },
            title = { Text("Reboot Wi-Fi bridge") },
            text = { Text(bridge.rebootPrompt) },
            confirmButton = {
                TextButton(onClick = {
                    confirmReboot = false
                    act("espBridge.reboot")
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { confirmReboot = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun StatusRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
        Text(value, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun Choice(label: String, options: List<String>, index: Int, enabled: Boolean = true, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(enabled = enabled, onClick = { open = true }) { Text(options.getOrElse(index) { "" }) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                options.forEachIndexed { at, option ->
                    DropdownMenuItem(text = { Text(option) }, onClick = {
                        open = false
                        onPick(at)
                    })
                }
            }
        }
    }
}

@Composable
private fun TextSetting(label: String, value: String, onDone: (String) -> Unit) {
    var typed by remember(value) { mutableStateOf(value) }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        OutlinedTextField(
            value = typed,
            onValueChange = { typed = it.take(16) },
            singleLine = true,
            keyboardActions = KeyboardActions(onDone = { onDone(typed) }),
            modifier = Modifier.width(180.dp),
        )
    }
}
