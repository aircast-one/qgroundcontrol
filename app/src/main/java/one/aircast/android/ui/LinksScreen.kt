package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.withContext
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath

private const val LINKS_PATH = "links.linkConfigurations"
private const val DEFAULT_PORT = "14550"

data class LinkRow(
    val index: Int,
    val name: String,
    val summary: String,
    val connected: Boolean,
    val dynamic: Boolean,
    val lastError: String,
    val heard: Boolean,
)

internal fun linkRows(json: JSONObject?): List<LinkRow> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        LinkRow(
            index = index,
            name = element.optString("name", "Link $index"),
            summary = element.optString("summary"),
            connected = element.optJSONArray("children")?.let { children ->
                (0 until children.length()).any { children.optString(it) == "link" }
            } ?: false,
            dynamic = element.optBoolean("dynamic"),
            lastError = element.optString("lastError"),
            heard = element.optBoolean("heardVehicle"),
        )
    }
}

internal fun configuredRows(rows: List<LinkRow>): List<LinkRow> = rows.filterNot { it.dynamic }

internal fun linkStatusLine(row: LinkRow): String {
    val state = when {
        !row.connected -> "Not connected"
        row.heard -> "Connected"
        else -> "Waiting for the vehicle"
    }
    val detail = row.summary.trim()
    return if (detail.isBlank() || row.name.contains(detail)) state else "$state · $detail"
}

internal fun autoLinkName(type: String, host: String, port: String): String =
    if (host.isBlank()) "${type.uppercase()} $port" else "${type.uppercase()} $host:$port"

internal fun linkFormError(type: String, host: String, port: String): String? {
    val parsed = port.toIntOrNull()
    return when {
        parsed == null || parsed !in 1..65535 -> "Port must be a number between 1 and 65535."
        type == "tcp" && host.isBlank() -> "A TCP link needs the address of the device to call."
        else -> null
    }
}

@Composable
private fun LinkRowItem(
    row: LinkRow,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
    onRemove: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }

    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                text = row.name,
                style = MaterialTheme.typography.titleMedium,
                fontSize = 18.sp,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = linkStatusLine(row),
                style = MaterialTheme.typography.bodyMedium,
                color = if (row.heard) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
                fontWeight = if (row.heard) FontWeight.Bold else FontWeight.Normal,
            )
            if (row.lastError.isNotBlank()) {
                Text(
                    row.lastError,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
        if (row.connected) {
            OutlinedButton(onClick = onDisconnect) { Text("Disconnect") }
        } else {
            Button(onClick = onConnect) { Text("Connect") }
        }
        Box {
            IconButton(onClick = { menuOpen = true }) {
                Icon(Icons.Filled.MoreVert, contentDescription = "More actions for ${row.name}")
            }
            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                DropdownMenuItem(
                    text = { Text("Remove link") },
                    onClick = {
                        menuOpen = false
                        onRemove()
                    },
                )
            }
        }
    }
}

@Composable
private fun AddLinkDialog(onDismiss: () -> Unit, onAdded: () -> Unit) {
    var type by remember { mutableStateOf("udp") }
    var name by remember { mutableStateOf("") }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf(DEFAULT_PORT) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add a link") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = type == "udp",
                        onClick = { type = "udp" },
                        label = { Text("UDP") },
                    )
                    FilterChip(
                        selected = type == "tcp",
                        onClick = { type = "tcp" },
                        label = { Text("TCP") },
                    )
                }
                Text(
                    text = if (type == "udp") {
                        "Listens on a port. Leave the address blank unless you need to reach a " +
                            "specific device."
                    } else {
                        "Calls out to a device that is listening, such as a ground station."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = host,
                    onValueChange = { host = it },
                    label = { Text(if (type == "tcp") "Address" else "Address (optional)") },
                    singleLine = true,
                )
                OutlinedTextField(
                    value = port,
                    onValueChange = { port = it },
                    label = { Text("Port") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                )
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Name (optional)") },
                    placeholder = { Text(autoLinkName(type, host, port)) },
                    singleLine = true,
                )
                error?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
            }
        },
        confirmButton = {
            Button(
                enabled = !busy,
                onClick = {
                    val invalid = linkFormError(type, host, port)
                    error = invalid
                    if (invalid == null) {
                        busy = true
                        val chosen = name.ifBlank { autoLinkName(type, host, port) }
                        scope.launch {
                            val added = withContext(Dispatchers.Default) {
                                Qgc.invokeResult(
                                    "links.createAndConnectLink", type, chosen, host, port.toInt(),
                                ) == true
                            }
                            busy = false
                            if (added) {
                                onAdded()
                            } else {
                                error = "Could not add that link. The name may already be in use."
                            }
                        }
                    }
                },
            ) { Text(if (busy) "Adding…" else "Add and connect") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

private const val LINK_SETTLE_MS = 4000L

internal fun linkFailure(action: String, done: Boolean): String? =
    if (done) null else "Could not $action that link."

private fun currentRows(): List<LinkRow> = linkRows(Qgc.get(LINKS_PATH))

@Composable
fun LinksScreen(modifier: Modifier = Modifier) {
    val json by qgcPath(LINKS_PATH)
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val rows = configuredRows(linkRows(json))
    val scope = rememberCoroutineScope()

    var notice by remember { mutableStateOf<String?>(null) }
    var adding by remember { mutableStateOf(false) }

    fun attempt(action: String, settled: () -> Boolean, call: () -> Boolean) {
        scope.launch {
            val dispatched = withContext(Dispatchers.Default) { call() }
            val done = dispatched && withTimeoutOrNull(LINK_SETTLE_MS) {
                while (!withContext(Dispatchers.Default) { settled() }) {
                    delay(150)
                }
                true
            } == true
            notice = linkFailure(action, done)
        }
    }
    var confirmingDisconnect by remember { mutableStateOf<LinkRow?>(null) }
    var confirmingRemove by remember { mutableStateOf<LinkRow?>(null) }

    Column(modifier.fillMaxSize()) {
    notice?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
        )
    }

    LazyColumn(Modifier.weight(1f)) {
        if (rows.isEmpty()) {
            item(key = "empty") {
                FootNote(
                    "No links saved. Aircast finds a vehicle on the network by itself, so you " +
                        "only need to add one when that does not reach it.",
                )
            }
        } else {
            items(rows, key = { "link${it.name}" }) { row ->
                LinkRowItem(
                    row = row,
                    onConnect = {
                        attempt(
                            action = "connect",
                            settled = { currentRows().getOrNull(row.index)?.connected == true },
                        ) { Qgc.invoke("links.createConnectedLink", "@$LINKS_PATH.${row.index}") }
                    },
                    onDisconnect = {
                        if (hasVehicle) {
                            confirmingDisconnect = row
                        } else {
                            attempt(
                                action = "disconnect",
                                settled = { currentRows().getOrNull(row.index)?.connected != true },
                            ) { Qgc.invoke("$LINKS_PATH.${row.index}.link.disconnect") }
                        }
                    },
                    onRemove = { confirmingRemove = row },
                )
                HorizontalDivider()
            }
        }

        item(key = "add") {
            Row(Modifier.fillMaxWidth().padding(20.dp)) {
                Button(onClick = { adding = true }, modifier = Modifier.fillMaxWidth()) {
                    Text("Add a link")
                }
            }
        }

        item(key = "note") {
            FootNote(
                "Automatic connections are not listed here — they come and go on their own. " +
                    "Serial and Bluetooth links are set up on the desktop.",
            )
        }
    }

    }

    if (adding) {
        AddLinkDialog(onDismiss = { adding = false }, onAdded = { adding = false })
    }

    confirmingDisconnect?.let { row ->
        AlertDialog(
            onDismissRequest = { confirmingDisconnect = null },
            title = { Text("Disconnect ${row.name}?") },
            text = {
                Text(
                    "A vehicle is connected on this link. Disconnecting stops telemetry and " +
                        "you will not be able to command it until it reconnects.",
                )
            },
            confirmButton = {
                Button(onClick = {
                    attempt(
                        action = "disconnect",
                        settled = { currentRows().getOrNull(row.index)?.connected != true },
                    ) { Qgc.invoke("$LINKS_PATH.${row.index}.link.disconnect") }
                    confirmingDisconnect = null
                }) { Text("Disconnect") }
            },
            dismissButton = {
                TextButton(onClick = { confirmingDisconnect = null }) { Text("Keep connected") }
            },
        )
    }

    confirmingRemove?.let { row ->
        AlertDialog(
            onDismissRequest = { confirmingRemove = null },
            title = { Text("Remove ${row.name}?") },
            text = {
                Text(
                    if (row.connected) {
                        "This link is connected. Removing it disconnects it first and forgets " +
                            "the settings."
                    } else {
                        "Aircast will forget this link and its settings."
                    },
                )
            },
            confirmButton = {
                Button(onClick = {
                    attempt(
                        action = "remove",
                        settled = { currentRows().none { it.name == row.name } },
                    ) { Qgc.invoke("links.removeConfiguration", "@$LINKS_PATH.${row.index}") }
                    confirmingRemove = null
                }) { Text("Remove") }
            },
            dismissButton = {
                TextButton(onClick = { confirmingRemove = null }) { Text("Cancel") }
            },
        )
    }
}
