package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal const val PACKET_RADIO_VIEW = "view.packetRadio"
private const val PACKET_RADIO_POLL_MS = 1000L
private const val PACKET_RADIO_DEVICE = "settings.packetRadioSettings.deviceName.rawValue"
private const val AUTOMATIC_ADAPTER = "Automatic"
private const val WAITING = "waiting"

internal data class PacketRadioStatus(
    val statusText: String,
    val linkActive: Boolean,
    val haveSignal: Boolean,
    val signal: String,
    val noise: String,
    val linkScore: String,
    val packetLoss: String,
    val videoPackets: String,
    val adapters: List<String>,
)

private fun joined(values: JSONArray?): String = (0 until (values?.length() ?: 0)).joinToString("  |  ") { values!!.opt(it).toString() }

internal fun packetRadioStatus(view: JSONObject?): PacketRadioStatus? = view?.takeIf { it.optText("class") == "PacketRadio" }?.let {
    val signal = it.optBoolean("haveSignal")
    val adapters = it.optJSONArray("adapters")
    PacketRadioStatus(
        statusText = it.optText("statusText").ifBlank { "Unavailable" },
        linkActive = it.optBoolean("linkActive"),
        haveSignal = signal,
        signal = if (signal) joined(it.optJSONArray("antennaRssiRaw")) else WAITING,
        noise = if (signal) joined(it.optJSONArray("antennaSnr")) + " dB" else WAITING,
        linkScore = if (signal && !it.isNull("linkScore")) it.optInt("linkScore").toString() else WAITING,
        packetLoss = if (it.isNull("packetLoss")) "" else it.optInt("packetLoss").toString(),
        videoPackets = if (it.isNull("videoPackets")) "" else it.optLong("videoPackets").toString(),
        adapters = (0 until (adapters?.length() ?: 0)).map { at -> adapters!!.optString(at) },
    )
}

@Composable
internal fun PacketRadioSection(onWrite: () -> Unit) {
    var read by remember { mutableStateOf<PacketRadioStatus?>(null) }
    var device by remember { mutableStateOf("") }
    var choosing by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) { Qgc.invoke("packetRadio.refreshAdapters") }
        while (true) {
            read = withContext(Dispatchers.Default) { packetRadioStatus(Qgc.get(PACKET_RADIO_VIEW)) }
            device = withContext(Dispatchers.Default) { Qgc.get(PACKET_RADIO_DEVICE)?.optText("value").orEmpty() }
            delay(PACKET_RADIO_POLL_MS)
        }
    }
    val status = read
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        StatusRow("Status", status?.statusText ?: "Unavailable")
        if (status?.linkActive == true) {
            Text("Link quality", style = MaterialTheme.typography.titleSmall)
            StatusRow("Signal per antenna (0-126)", status.signal)
            StatusRow("Noise margin per antenna", status.noise)
            StatusRow("Link score (1000-2000)", status.linkScore)
            StatusRow("Packets lost (last second)", status.packetLoss)
            StatusRow("Video packets", status.videoPackets)
        }
        Text("Radio", style = MaterialTheme.typography.titleSmall)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("Wi-Fi adapter", modifier = Modifier.weight(1f))
            Box {
                OutlinedButton(onClick = { choosing = true }) { Text(device.ifBlank { AUTOMATIC_ADAPTER }) }
                DropdownMenu(expanded = choosing, onDismissRequest = { choosing = false }) {
                    (listOf(AUTOMATIC_ADAPTER) + status?.adapters.orEmpty()).forEachIndexed { index, name ->
                        DropdownMenuItem(text = { Text(name) }, onClick = {
                            choosing = false
                            scope.launch {
                                withContext(Dispatchers.Default) { Qgc.set(PACKET_RADIO_DEVICE, if (index == 0) "" else name) }
                                onWrite()
                            }
                        })
                    }
                }
            }
        }
    }
}

@Composable
private fun StatusRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
        Text(value, style = MaterialTheme.typography.bodySmall)
    }
}
