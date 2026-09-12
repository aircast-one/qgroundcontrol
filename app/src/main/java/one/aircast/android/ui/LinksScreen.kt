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
import androidx.compose.runtime.mutableIntStateOf
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
import one.aircast.android.bridge.qgcStrings
import one.aircast.mapspike.optText

private const val LINKS_VIEW = "view.links"
private const val LINKS_PATH = "links.linkConfigurations"
private const val DEFAULT_PORT = "14550"

data class LinkRow(
    val index: Int,
    val name: String,
    val statusLine: String,
    val goneQuiet: Boolean = false,
    val connected: Boolean,
    val heard: Boolean,
    val lastError: String,
    val editing: String = "",
    val host: String = "",
    val port: Int = 0,
    val portName: String = "",
    val baud: Int = 0,
)

internal fun linkRows(view: JSONObject?): List<LinkRow> {
    val links = view?.optJSONArray("configured") ?: return emptyList()
    return (0 until links.length()).mapNotNull { position ->
        links.optJSONObject(position)?.let { link ->
            LinkRow(
                index = link.optInt("index"),
                name = link.optText("name"),
                statusLine = link.optText("statusLine"),
                goneQuiet = link.optBoolean("goneQuiet"),
                connected = link.optBoolean("connected"),
                heard = link.optBoolean("heardVehicle"),
                lastError = link.optText("lastError"),
                editing = link.optText("editing"),
                host = link.optText("host"),
                port = link.optInt("port"),
                portName = link.optText("portName"),
                baud = link.optInt("baud"),
            )
        }
    }
}

internal fun linkIsEditable(row: LinkRow): Boolean =
    !row.connected && row.editing in setOf("hostAndPort", "portOnly", "serial")

internal fun editWrites(
    editing: String,
    name: String,
    host: String,
    port: Int,
    portName: String,
    baud: Int,
): List<Pair<String, Any>> = listOf<Pair<String, Any>>("name" to name) + when (editing) {
    "hostAndPort" -> listOf("host" to host, "port" to port)
    "portOnly" -> listOf("localPort" to port)
    "serial" -> listOf("portName" to portName, "baud" to baud)
    else -> emptyList()
}

internal fun autoLinkName(type: String, host: String, port: String): String =
    if (host.isBlank()) "${type.uppercase()} $port" else "${type.uppercase()} $host:$port"

internal const val DEFAULT_BAUD = 57600

internal data class SerialPortChoice(val port: String, val label: String)

internal fun serialPortChoices(ports: List<String>, labels: List<String>): List<SerialPortChoice> =
    ports.filter { it.isNotBlank() }
        .mapIndexed { index, port ->
            SerialPortChoice(port, labels.getOrNull(index)?.ifBlank { null } ?: port)
        }

internal fun serialBauds(view: JSONObject?): List<Int> {
    val rates = view?.optJSONArray("baudRates") ?: return emptyList()
    return (0 until rates.length()).mapNotNull { rates.opt(it)?.toString()?.toIntOrNull() }.filter { it > 0 }
}

internal fun autoSerialName(portName: String): String = "Serial ${portName.substringAfterLast('/')}"

internal fun serialFormError(
    portName: String,
    baud: Int,
    taken: List<String>,
    name: String,
    anyPorts: Boolean,
): String? = when {
    !anyPorts -> "Nothing is plugged in. Connect a radio over USB and it will appear here."
    portName.isBlank() -> "Pick the port the radio is plugged into."
    baud <= 0 -> "Pick a baud rate."
    name.ifBlank { autoSerialName(portName) } in taken -> "A link with that name already exists."
    else -> null
}

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
    onEdit: () -> Unit,
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
                text = row.statusLine,
                style = MaterialTheme.typography.bodyMedium,
                color = when {
                    row.goneQuiet -> MaterialTheme.colorScheme.error
                    row.heard -> MaterialTheme.colorScheme.primary
                    else -> MaterialTheme.colorScheme.onSurfaceVariant
                },
                fontWeight = if (row.heard && !row.goneQuiet) FontWeight.Bold else FontWeight.Normal,
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
                    text = { Text("Edit link") },
                    enabled = linkIsEditable(row),
                    onClick = {
                        menuOpen = false
                        onEdit()
                    },
                )
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
private fun EditLinkDialog(row: LinkRow, onDismiss: () -> Unit, onSaved: () -> Unit) {
    var name by remember { mutableStateOf(row.name) }
    var host by remember { mutableStateOf(row.host) }
    var port by remember { mutableStateOf(row.port.toString()) }
    var portName by remember { mutableStateOf(row.portName) }
    var baud by remember { mutableIntStateOf(if (row.baud > 0) row.baud else DEFAULT_BAUD) }
    var baudsOpen by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val linksJson by qgcPath(LINKS_VIEW)
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Edit ${row.name}") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Name") },
                    singleLine = true,
                )
                if (row.editing == "hostAndPort") {
                    OutlinedTextField(
                        value = host,
                        onValueChange = { host = it },
                        label = { Text("Address") },
                        singleLine = true,
                    )
                }
                if (row.editing == "serial") {
                    Box {
                        OutlinedButton(onClick = { baudsOpen = true }) { Text("$baud baud") }
                        DropdownMenu(expanded = baudsOpen, onDismissRequest = { baudsOpen = false }) {
                            bauds.forEach { rate ->
                                DropdownMenuItem(
                                    text = { Text("$rate") },
                                    onClick = { baud = rate; baudsOpen = false },
                                )
                            }
                        }
                    }
                } else {
                    OutlinedTextField(
                        value = port,
                        onValueChange = { port = it },
                        label = { Text("Port") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                }
                error?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    val parsed = port.toIntOrNull() ?: 0
                    val invalid = when {
                        name.isBlank() -> "A link needs a name."
                        row.editing != "serial" && parsed !in 1..65535 ->
                            "Port must be a number between 1 and 65535."
                        else -> null
                    }
                    error = invalid
                    if (invalid == null) {
                        scope.launch {
                            // The menu gated on a watched row. A link can connect between that
                            // poll and this tap, and writing a port or baud underneath a live
                            // link changes the configuration without changing the connection.
                            val live = withContext(Dispatchers.Default) {
                                currentRows().firstOrNull { it.index == row.index }
                            }
                            if (live == null || live.connected) {
                                error = "Disconnect the link before changing its settings."
                                return@launch
                            }
                            withContext(Dispatchers.Default) {
                                editWrites(row.editing, name, host, parsed, portName, baud)
                                    .forEach { (field, value) ->
                                        Qgc.set("$LINKS_PATH.${row.index}.$field", value)
                                    }
                                Qgc.invoke("links.commitLinkConfigurations")
                            }
                            onSaved()
                        }
                    }
                },
            ) { Text("Save") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun AddLinkDialog(onDismiss: () -> Unit, onAdded: () -> Unit) {
    var type by remember { mutableStateOf("udp") }
    var name by remember { mutableStateOf("") }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf(DEFAULT_PORT) }
    var portName by remember { mutableStateOf("") }
    var baud by remember { mutableIntStateOf(DEFAULT_BAUD) }
    var portsOpen by remember { mutableStateOf(false) }
    var baudsOpen by remember { mutableStateOf(false) }
    val linksJson by qgcPath(LINKS_VIEW)
    val portPaths by qgcStrings("links.serialPorts")
    val portLabels by qgcStrings("links.serialPortStrings")
    val ports = remember(portPaths, portLabels) { serialPortChoices(portPaths, portLabels) }
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val taken = remember(linksJson) { linkRows(linksJson).map { it.name } }
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
                    FilterChip(
                        selected = type == "serial",
                        onClick = { type = "serial" },
                        label = { Text("Serial") },
                    )
                }
                Text(
                    text = when (type) {
                        "udp" -> "Listens on a port. Leave the address blank unless you need to " +
                            "reach a specific device."
                        "serial" -> "A radio plugged into this device over USB."
                        else -> "Calls out to a device that is listening, such as a ground station."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (type == "serial" && ports.isEmpty()) {
                    Text(
                        text = "Nothing is plugged in. Connect a radio over USB and it will " +
                            "appear here.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else if (type == "serial") {
                    Box {
                        OutlinedButton(onClick = { portsOpen = true }) {
                            Text(ports.firstOrNull { it.port == portName }?.label ?: "Choose a port")
                        }
                        DropdownMenu(expanded = portsOpen, onDismissRequest = { portsOpen = false }) {
                            ports.forEach { choice ->
                                DropdownMenuItem(
                                    text = { Text(choice.label) },
                                    onClick = { portName = choice.port; portsOpen = false },
                                )
                            }
                        }
                    }
                    Box {
                        OutlinedButton(onClick = { baudsOpen = true }) { Text("$baud baud") }
                        DropdownMenu(expanded = baudsOpen, onDismissRequest = { baudsOpen = false }) {
                            bauds.forEach { rate ->
                                DropdownMenuItem(
                                    text = { Text("$rate") },
                                    onClick = { baud = rate; baudsOpen = false },
                                )
                            }
                        }
                    }
                } else {
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
                }
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Name (optional)") },
                    placeholder = {
                        Text(
                            if (type == "serial") autoSerialName(portName)
                            else autoLinkName(type, host, port),
                        )
                    },
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
                    val invalid = if (type == "serial") {
                        serialFormError(portName, baud, taken, name, ports.isNotEmpty())
                    } else {
                        linkFormError(type, host, port)
                    }
                    error = invalid
                    if (invalid == null) {
                        busy = true
                        val chosen = name.ifBlank {
                            if (type == "serial") autoSerialName(portName) else autoLinkName(type, host, port)
                        }
                        scope.launch {
                            val added = withContext(Dispatchers.Default) {
                                if (type == "serial") {
                                    Qgc.invokeResult("links.createSerialConfiguration", chosen, portName, baud) == true &&
                                        connectNamed(chosen)
                                } else {
                                    Qgc.invokeResult(
                                        "links.createAndConnectLink", type, chosen, host, port.toInt(),
                                    ) == true
                                }
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

private fun currentRows(): List<LinkRow> = linkRows(Qgc.get(LINKS_VIEW))

private fun connectNamed(name: String): Boolean {
    val row = currentRows().firstOrNull { it.name == name } ?: return false
    Qgc.invoke("links.createConnectedLink", "@$LINKS_PATH.${row.index}")
    return true
}

@Composable
fun LinksScreen(modifier: Modifier = Modifier) {
    val view by qgcPath(LINKS_VIEW)
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val rows = linkRows(view)
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
    var editing by remember { mutableStateOf<LinkRow?>(null) }

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
                    onEdit = { editing = row },
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
                    "Bluetooth links are set up on the desktop.",
            )
        }
    }

    }

    if (adding) {
        AddLinkDialog(onDismiss = { adding = false }, onAdded = { adding = false })
    }

    editing?.let { row ->
        EditLinkDialog(
            row = row,
            onDismiss = { editing = null },
            onSaved = { editing = null },
        )
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
