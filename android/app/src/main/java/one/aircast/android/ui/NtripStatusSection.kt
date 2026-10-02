package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val NTRIP_VIEW = "view.ntrip"
internal const val NTRIP_CONNECT_SETTING = "settings.ntripSettings.ntripServerConnectEnabled"
private const val NTRIP_POLL_MS = 1000L
private val NTRIP_GREEN = Color(0xFF34C759)
private val NTRIP_ORANGE = Color(0xFFFF9F0A)

internal data class NtripStatus(
    val status: String,
    val message: String,
    val button: String,
    val buttonEnabled: Boolean,
    val active: Boolean,
    val dataStale: Boolean,
    val mountpoint: String,
    val messages: Long,
    val messageTypes: List<Pair<Int, Long>>,
    val bytesReceived: Long,
    val dataRate: Double,
    val dataWarning: Boolean,
    val bytesSent: Long,
    val sentKBps: Double,
    val securityWarning: String,
    val ggaSource: String,
    val browser: NtripBrowser,
)

internal fun ntripStatus(view: JSONObject?): NtripStatus? = view?.takeIf { it.optText("status").isNotBlank() }?.let {
    val types = it.optJSONArray("messageTypes")
    NtripStatus(
        status = it.optText("status"),
        message = it.optText("statusMessage"),
        button = it.optText("button"),
        buttonEnabled = it.optBoolean("buttonEnabled"),
        active = it.optBoolean("active"),
        dataStale = it.optBoolean("dataStale"),
        mountpoint = it.optText("mountpoint"),
        messages = it.optLong("messages"),
        messageTypes = (0 until (types?.length() ?: 0)).mapNotNull { at -> types!!.optJSONArray(at)?.let { pair -> pair.optInt(0) to pair.optLong(1) } },
        bytesReceived = it.optLong("bytesReceived"),
        dataRate = it.optDouble("dataRateBytesPerSec", 0.0),
        dataWarning = it.optBoolean("dataWarning"),
        bytesSent = it.optLong("bytesSent"),
        sentKBps = it.optDouble("sentKBps", 0.0),
        securityWarning = it.optText("securityWarning"),
        ggaSource = it.optText("ggaSource"),
        browser = ntripBrowser(it.optJSONObject("browser")),
    )
}

internal data class NtripMountpointRow(val mountpoint: String, val detail: String, val selected: Boolean)

internal data class NtripBrowser(val status: String, val error: String, val canBrowse: Boolean, val mountpoints: List<NtripMountpointRow>)

internal fun ntripBrowser(json: JSONObject?): NtripBrowser {
    val rows = json?.optJSONArray("mountpoints")
    return NtripBrowser(
        status = json?.optText("status").orEmpty(),
        error = json?.optText("error").orEmpty(),
        canBrowse = json?.optBoolean("canBrowse") == true,
        mountpoints = (0 until (rows?.length() ?: 0)).mapNotNull { at ->
            rows!!.optJSONObject(at)?.let { NtripMountpointRow(it.optText("mountpoint"), it.optText("detail"), it.optBoolean("selected")) }
        },
    )
}

internal fun dataSize(bytes: Long): String = when {
    bytes < 1024 -> "$bytes B"
    bytes < 1048576 -> "%.1f KB".format(java.util.Locale.ROOT, bytes / 1024.0)
    else -> "%.1f MB".format(java.util.Locale.ROOT, bytes / 1048576.0)
}

internal fun dataRate(bytesPerSec: Double): String =
    if (bytesPerSec < 1024) "%.0f B/s".format(java.util.Locale.ROOT, bytesPerSec) else "%.1f KB/s".format(java.util.Locale.ROOT, bytesPerSec / 1024)

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun NtripStatusSection(onWrite: () -> Unit) {
    var read by remember { mutableStateOf<NtripStatus?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) {
        while (true) {
            read = withContext(Dispatchers.Default) { ntripStatus(Qgc.get(NTRIP_VIEW)) }
            delay(NTRIP_POLL_MS)
        }
    }
    val status = read ?: return
    val connected = status.status == "connected"
    val dot = when (status.status) {
        "connected" -> NTRIP_GREEN
        "connecting", "reconnecting" -> NTRIP_ORANGE
        "error" -> MaterialTheme.colorScheme.error
        else -> MaterialTheme.colorScheme.outline
    }
    SectionHeader("Connection")
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Box(Modifier.size(10.dp).background(dot, CircleShape))
            Text(status.message.ifBlank { "Disconnected" }, modifier = Modifier.weight(1f))
            Button(
                enabled = status.buttonEnabled,
                onClick = {
                    scope.launch {
                        withContext(Dispatchers.Default) { Qgc.set(NTRIP_CONNECT_SETTING, !status.active) }
                        onWrite()
                    }
                },
            ) { Text(status.button) }
        }
        if (status.status != "disconnected") {
            if (status.dataStale && connected) Text("Connected but no data received recently", color = NTRIP_ORANGE)
            if (status.mountpoint.isNotBlank()) StatusLine("Mountpoint", status.mountpoint)
            if (connected) StatusLine("Messages", status.messages.toString())
            if (connected && status.messageTypes.isNotEmpty()) {
                Text("Message types", style = MaterialTheme.typography.labelLarge)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    status.messageTypes.forEach { (id, count) ->
                        Text(
                            "${if (id == 0) "unknown" else id.toString()} × $count",
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.background(MaterialTheme.colorScheme.surfaceVariant, RoundedCornerShape(50)).padding(horizontal = 8.dp, vertical = 2.dp),
                        )
                    }
                }
            }
            if (connected && status.bytesReceived > 0) StatusLine("Data Received", "${dataSize(status.bytesReceived)} (${dataRate(status.dataRate)})")
            if (connected && status.dataWarning) Text("Warning: Data usage: ${dataSize(status.bytesReceived)} — consider connection costs", color = NTRIP_ORANGE, style = MaterialTheme.typography.bodySmall)
            if (connected && status.bytesSent > 0) StatusLine("To Vehicle", "${dataSize(status.bytesSent)} (${"%.1f".format(java.util.Locale.ROOT, status.sentKBps)} KB/s)")
            if (connected && status.ggaSource.isNotBlank()) StatusLine("GGA Source", status.ggaSource)
            if (status.securityWarning.isNotBlank()) Text(status.securityWarning, color = NTRIP_ORANGE, style = MaterialTheme.typography.bodySmall)
            if (status.status == "error") Text(status.message, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
private fun StatusLine(label: String, value: String) {
    Row(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        Text(value, style = MaterialTheme.typography.bodyMedium)
    }
}

internal const val NTRIP_FETCH_MOUNTPOINTS = "ntrip.fetchMountpoints"
internal const val NTRIP_SELECT_MOUNTPOINT = "ntrip.selectMountpoint"

@Composable
internal fun NtripMountpointBrowser(onWrite: () -> Unit) {
    var browser by remember { mutableStateOf<NtripBrowser?>(null) }
    var refreshes by remember { mutableStateOf(0) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(refreshes) {
        browser = withContext(Dispatchers.Default) { ntripStatus(Qgc.get(NTRIP_VIEW))?.browser }
        if (browser?.status == "inProgress") {
            delay(NTRIP_POLL_MS)
            refreshes++
        }
    }
    val read = browser ?: return
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        androidx.compose.material3.OutlinedButton(
            enabled = read.canBrowse,
            onClick = {
                scope.launch {
                    withContext(Dispatchers.Default) { Qgc.invoke(NTRIP_FETCH_MOUNTPOINTS) }
                    refreshes++
                }
            },
        ) { Text("Browse") }
        if (read.status == "inProgress") Text("Fetching mountpoints…", color = NTRIP_ORANGE)
        if (read.status == "error") Text(read.error, color = MaterialTheme.colorScheme.error)
        read.mountpoints.forEach { row ->
            Row(
                Modifier.fillMaxWidth().padding(vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(row.mountpoint, style = MaterialTheme.typography.bodyLarge, color = if (row.selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface)
                        if (row.selected) Text("(selected)", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                    }
                    if (row.detail.isNotBlank()) Text(row.detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                androidx.compose.material3.TextButton(
                    enabled = !row.selected,
                    onClick = {
                        scope.launch {
                            withContext(Dispatchers.Default) { Qgc.invoke(NTRIP_SELECT_MOUNTPOINT, row.mountpoint) }
                            refreshes++
                            onWrite()
                        }
                    },
                ) { Text(if (row.selected) "Selected" else "Select") }
            }
        }
    }
}
