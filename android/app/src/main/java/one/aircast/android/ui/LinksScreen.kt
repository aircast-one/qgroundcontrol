package one.aircast.android.ui

import androidx.compose.foundation.background
import one.aircast.android.bridge.LinkCommands
import one.aircast.android.bridge.AccountCommands
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import one.aircast.android.R
import one.aircast.map.aircast

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.material3.Switch
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.withContext
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText

private const val LINKS_VIEW = "view.links"
private const val LINKS_PATH = "links.linkConfigurations"
private const val DEFAULT_PORT = "14550"
private const val DEFAULT_TCP_PORT = "5760"
private const val UDP_LISTEN_PORT = "settings.autoConnectSettings.udpListenPort"
private const val AUTO_CONNECT_UDP = "settings.autoConnectSettings.autoConnectUDP"

data class LinkRow(
    val index: Int,
    val name: String,
    val statusLine: String,
    val goneQuiet: Boolean = false,
    val connected: Boolean,
    val heard: Boolean,
    val lastError: String,
    val errorRemedy: String = "",
    val editing: String = "",
    val host: String = "",
    val port: Int = 0,
    val portName: String = "",
    val baud: Int = 0,
    val framing: SerialFraming = SerialFraming(),
    val servers: List<String> = emptyList(),
    val autoConnect: Boolean = false,
    val highLatency: Boolean = false,
    val type: String = "",
    val filename: String = "",
)

internal data class AutoLink(val name: String, val summary: String, val heard: Boolean, val type: String = "", val index: Int = 0)

internal fun autoLinks(view: JSONObject?): List<AutoLink> {
    val links = view?.optJSONArray("links") ?: return emptyList()
    return (0 until links.length()).mapNotNull { links.optJSONObject(it) }
        .filter { it.optBoolean("dynamic") && it.optBoolean("connected") }
        .map { AutoLink(it.optText("name"), it.optText("displaySummary"), it.optBoolean("heardVehicle"), it.optText("type"), it.optInt("index")) }
}

internal fun autoLinkStatus(link: AutoLink): String = if (link.heard) "Vehicle" else "Listening"

internal fun autoLinkSubtitle(link: AutoLink): String =
    if (link.type == MOCK_LINK) "Simulated" else listOf(link.summary, "automatic").filter { it.isNotBlank() }.joinToString(" \u00b7 ")

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
                errorRemedy = link.optText("errorRemedy"),
                editing = link.optText("editing"),
                host = link.optText("host"),
                port = link.optInt("port"),
                portName = link.optText("portName"),
                baud = link.optInt("baud"),
                framing = SerialFraming(link.optInt("dataBits", 8), link.optInt("stopBits", 1), link.optInt("parity", 0), link.optInt("flowControl", 0)),
                servers = link.optJSONArray("hostList")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
                autoConnect = link.optBoolean("autoConnect"),
                highLatency = link.optBoolean("highLatency"),
                type = link.optText("type"),
                filename = link.optText("filename"),
            )
        }
    }
}

@androidx.annotation.DrawableRes
internal fun linkIcon(type: String): Int = when (type) {
    "serial" -> R.drawable.ic_usb
    "udp" -> R.drawable.ic_wifi
    BLUETOOTH_LINK -> R.drawable.ic_bluetooth
    else -> R.drawable.ic_link
}

private const val REPLAY_LINK_FOLDER = "replay-links"

private fun replayFolder(context: android.content.Context, link: String): java.io.File =
    java.io.File(java.io.File(context.filesDir, REPLAY_LINK_FOLDER), link.replace(Regex("[^A-Za-z0-9._-]"), "_"))

internal fun stagedReplayLog(context: android.content.Context, link: String, uri: android.net.Uri): String? = runCatching {
    val shown = context.contentResolver.query(uri, arrayOf(android.provider.OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }?.replace(Regex("[/\\\\]"), "_")?.ifBlank { null } ?: "replay.tlog"
    val folder = replayFolder(context, link).apply { mkdirs() }
    val incoming = java.io.File(folder, ".incoming")
    val copied = context.contentResolver.openInputStream(uri)?.use { source -> incoming.outputStream().use { source.copyTo(it) } } != null
    val staged = java.io.File(folder, shown)
    staged.takeIf { copied && incoming.renameTo(it) }?.absolutePath
}.getOrNull()

internal fun pruneReplayFolder(context: android.content.Context, link: String, keep: String) {
    replayFolder(context, link).listFiles()?.filter { it.absolutePath != keep }?.forEach { it.delete() }
}

internal fun linkIsEditable(row: LinkRow): Boolean =
    !row.connected && row.editing in setOf("hostAndPort", "portOnly", "serial", "logFile", "device")

internal val CREATABLE_LINK_TYPES = listOf("udp", "tcp", "serial")
internal const val BLUETOOTH_LINK = "bluetooth"
internal const val REPLAY_LINK = "logReplay"
internal const val MOCK_LINK = "mock"
internal const val REPLAY_LINK_NAME = "Log Replay"

internal data class BluetoothDeviceChoice(val name: String, val address: String)

internal data class BluetoothState(val available: Boolean, val scanning: Boolean, val devices: List<BluetoothDeviceChoice>)

internal fun bluetoothState(view: JSONObject?): BluetoothState {
    val bluetooth = view?.optJSONObject("bluetooth")
    val devices = bluetooth?.optJSONArray("devices")
    return BluetoothState(
        available = bluetooth?.optBoolean("available") == true,
        scanning = bluetooth?.optBoolean("scanning") == true,
        devices = (0 until (devices?.length() ?: 0)).mapNotNull { at -> devices!!.optJSONObject(at)?.let { BluetoothDeviceChoice(it.optText("name"), it.optText("address")) } },
    )
}

internal fun addableLinkTypes(view: JSONObject?): List<String> {
    val listed = view?.optJSONArray("linkTypeIds") ?: return CREATABLE_LINK_TYPES
    val served = (0 until listed.length()).map { listed.optString(it) }.toSet()
    val bluetooth = listOf(BLUETOOTH_LINK).filter { it in served && bluetoothState(view).available }
    return CREATABLE_LINK_TYPES.filter { it in served }.ifEmpty { CREATABLE_LINK_TYPES } + bluetooth + listOf(REPLAY_LINK, MOCK_LINK).filter { it in served }
}

internal fun linkTypeIds(view: JSONObject?): Set<String> {
    val listed = view?.optJSONArray("linkTypeIds") ?: return emptySet()
    return (0 until listed.length()).map { listed.optString(it) }.toSet()
}

internal fun bluetoothPermissions(sdk: Int): Array<String> =
    if (sdk >= android.os.Build.VERSION_CODES.S) {
        arrayOf(android.Manifest.permission.BLUETOOTH_SCAN, android.Manifest.permission.BLUETOOTH_CONNECT)
    } else {
        arrayOf(android.Manifest.permission.ACCESS_FINE_LOCATION)
    }

data class SerialFraming(val dataBits: Int = 8, val stopBits: Int = 1, val parity: Int = 0, val flowControl: Int = 0)

internal val PARITY_CHOICES = listOf("None" to 0, "Even" to 2, "Odd" to 3)
internal val DATA_BITS_CHOICES = listOf(5, 6, 7, 8)
internal val STOP_BITS_CHOICES = listOf(1, 2)

internal fun editWrites(
    editing: String,
    name: String,
    host: String,
    port: Int,
    portName: String,
    baud: Int,
    autoConnect: Boolean,
    highLatency: Boolean,
    framing: SerialFraming = SerialFraming(),
    logFile: String = "",
): List<Pair<String, Any>> = listOf<Pair<String, Any>>("name" to name, "autoConnect" to autoConnect, "highLatency" to highLatency) + when (editing) {
    "hostAndPort" -> listOf("host" to host, "port" to port)
    "portOnly" -> listOf("localPort" to port)
    "logFile" -> listOf("filename" to logFile)
    "serial" -> listOf(
        "portName" to portName,
        "baud" to baud,
        "dataBits" to framing.dataBits,
        "stopBits" to framing.stopBits,
        "parity" to framing.parity,
        "flowControl" to framing.flowControl,
    )
    else -> emptyList()
}

internal fun linkTypeLabel(id: String): String = when (id) {
    "serial" -> "Serial"
    BLUETOOTH_LINK -> "Bluetooth"
    AIRCAST_CLOUD_LINK -> "Aircast Cloud"
    REPLAY_LINK -> "Log replay"
    MOCK_LINK -> "Simulated"
    else -> id.uppercase()
}

internal fun autoLinkName(type: String, host: String, port: String): String = when {
    type == "udp" -> "UDP $port"
    host.isBlank() -> type.uppercase()
    else -> "${type.uppercase()} $host:$port"
}

internal fun uniqueLinkName(base: String, taken: List<String>): String =
    (listOf(base) + (2..taken.size + 2).map { "$base ($it)" }).first { it !in taken }

internal fun editedLinkSuggestion(editing: String, host: String, port: String, portLabel: String): String = when (editing) {
    "serial" -> autoSerialName(portLabel)
    "hostAndPort" -> autoLinkName("tcp", host, port)
    else -> autoLinkName("udp", host, port)
}

internal const val DEFAULT_BAUD = 57600

internal data class SerialPortChoice(val port: String, val label: String)

internal fun serialPortChoices(view: JSONObject?): List<SerialPortChoice> {
    val ports = view?.optJSONArray("serialPorts") ?: return emptyList()
    return (0 until ports.length()).mapNotNull { index ->
        val entry = ports.optJSONObject(index) ?: return@mapNotNull null
        val port = entry.optText("port").ifBlank { return@mapNotNull null }
        SerialPortChoice(port, entry.optText("label").ifBlank { port })
    }
}

internal fun serialBauds(view: JSONObject?): List<Int> {
    val rates = view?.optJSONArray("baudRates") ?: return emptyList()
    return (0 until rates.length()).mapNotNull { rates.opt(it)?.toString()?.toIntOrNull() }.filter { it > 0 }
}

internal fun autoSerialName(portLabel: String): String = portLabel.ifBlank { "Serial" }

internal fun portLabel(ports: List<SerialPortChoice>, portName: String): String = ports.firstOrNull { it.port == portName }?.label.orEmpty()

internal fun portFor(type: String, udpDefault: String): String = if (type == "tcp") DEFAULT_TCP_PORT else udpDefault

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
    name.isNotBlank() && name.trim() in taken -> "A link with that name already exists."
    else -> null
}

internal fun udpServer(typed: String, localPort: String): String? {
    val parts = typed.trim().split(":")
    val host = parts.first().trim()
    val port = (parts.getOrNull(1) ?: localPort).trim().toIntOrNull()
    return if (parts.size > 2 || host.isEmpty() || port == null || port !in 1..65535) null else "$host:$port"
}

internal fun withServer(servers: List<String>, typed: String, localPort: String): List<String> =
    udpServer(typed, localPort)?.takeIf { it !in servers }?.let { servers + it } ?: servers

internal fun linkFormError(type: String, host: String, port: String, udpDefault: String = DEFAULT_PORT): String? {
    val parsed = port.ifBlank { if (type == "udp") udpDefault else port }.toIntOrNull()
    return when {
        type == "udp" && (parsed == null || parsed !in 1..65535) -> "Enter a port between 1 and 65535, or leave it blank for $udpDefault"
        parsed == null || parsed !in 1..65535 -> "Port must be a number between 1 and 65535."
        type == "tcp" && host.isBlank() -> "A TCP link needs the address of the device to call."
        else -> null
    }
}

internal fun linkFormErrorField(type: String, host: String, port: String, udpDefault: String = DEFAULT_PORT): String? {
    val parsed = port.ifBlank { if (type == "udp") udpDefault else port }.toIntOrNull()
    return when {
        parsed == null || parsed !in 1..65535 -> "port"
        type == "tcp" && host.isBlank() -> "host"
        else -> null
    }
}

private const val CONNECTED_STATUS = "Connected"

@Composable
private fun AutoLinkItem(link: AutoLink, onStop: (() -> Unit)? = null) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 72.dp).padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier.size(40.dp).background(if (link.heard) MaterialTheme.aircast.successContainer else MaterialTheme.colorScheme.secondaryContainer, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(painterResource(linkIcon(link.type)), null, tint = if (link.heard) MaterialTheme.aircast.success else MaterialTheme.colorScheme.onSecondaryContainer, modifier = Modifier.size(24.dp))
        }
        Column(Modifier.weight(1f)) {
            Text(link.name, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(autoLinkSubtitle(link), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Text(autoLinkStatus(link), style = MaterialTheme.typography.labelMedium, color = if (link.heard) MaterialTheme.aircast.success else MaterialTheme.colorScheme.onSurfaceVariant)
        onStop?.let { TextButton(onClick = it) { Text("Stop") } }
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
    val fixByEditing = row.errorRemedy == REMEDY_EDIT_ADDRESS && linkIsEditable(row)

    Box {
        Row(
            Modifier
                .fillMaxWidth()
                .clickable { menuOpen = true }
                .semantics { contentDescription = "More actions for ${row.name}" }
                .heightIn(min = 72.dp)
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Box(
                Modifier.size(40.dp).background(
                    if (row.connected) MaterialTheme.aircast.successContainer else MaterialTheme.colorScheme.secondaryContainer,
                    CircleShape,
                ),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    painterResource(linkIcon(row.type)),
                    null,
                    tint = if (row.connected) MaterialTheme.aircast.success else MaterialTheme.colorScheme.onSecondaryContainer,
                    modifier = Modifier.size(24.dp),
                )
            }
            Column(Modifier.weight(1f)) {
                Text(
                    text = row.name,
                    style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (!(row.connected && row.statusLine == CONNECTED_STATUS)) {
                    Text(
                        text = row.statusLine,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (row.goneQuiet) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (row.lastError.isNotBlank()) {
                    Text(
                        row.lastError,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                    remedyText(row.errorRemedy)?.let { advice ->
                        Text(
                            advice,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            if (row.connected) {
                Text(CONNECTED_STATUS, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.aircast.success)
            }
            Icon(painterResource(R.drawable.ic_chevron_right), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
            DropdownMenuItem(
                text = { Text(if (row.connected) "Disconnect" else if (fixByEditing) "Edit" else "Connect") },
                onClick = {
                    menuOpen = false
                    when {
                        row.connected -> onDisconnect()
                        fixByEditing -> onEdit()
                        else -> onConnect()
                    }
                },
            )
            DropdownMenuItem(
                text = { Text("Edit link") },
                enabled = linkIsEditable(row),
                onClick = {
                    menuOpen = false
                    onEdit()
                },
            )
            DropdownMenuItem(
                text = { Text("Delete link") },
                onClick = {
                    menuOpen = false
                    onRemove()
                },
            )
        }
    }
}

@Composable
private fun LinkOptions(autoConnect: Boolean, highLatency: Boolean, onAutoConnect: (Boolean) -> Unit, onHighLatency: (Boolean) -> Unit) {
    var open by remember { mutableStateOf(autoConnect || highLatency) }
    Row(
        Modifier.fillMaxWidth().clickable { open = !open }.heightIn(min = 48.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("Options", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
        Icon(
            if (open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = if (open) "Hide options" else "Show options",
        )
    }
    if (open) LinkFlagSwitches(autoConnect, highLatency, onAutoConnect, onHighLatency)
}

@Composable
private fun LinkFlagSwitches(autoConnect: Boolean, highLatency: Boolean, onAutoConnect: (Boolean) -> Unit, onHighLatency: (Boolean) -> Unit) {
    LINK_FLAG_TEXT.zip(listOf(autoConnect to onAutoConnect, highLatency to onHighLatency)).forEach { (text, state) ->
        val (checked, onChange) = state
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(text.first)
                Text(text.second, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = checked, onCheckedChange = onChange)
        }
    }
}

internal val LINK_FLAG_TEXT = listOf(
    "Connect on start" to "Open this link when the app launches.",
    "High latency" to "Tune the link for satellite or cellular round trips.",
)

internal fun linkFlagWrites(index: Int, autoConnect: Boolean, highLatency: Boolean): List<Pair<String, Boolean>> =
    listOf("$LINKS_PATH.$index.autoConnect" to autoConnect, "$LINKS_PATH.$index.highLatency" to highLatency)

internal fun framingWrites(index: Int, framing: SerialFraming): List<Pair<String, Int>> = listOf(
    "dataBits" to framing.dataBits,
    "stopBits" to framing.stopBits,
    "parity" to framing.parity,
    "flowControl" to framing.flowControl,
).map { (field, value) -> "$LINKS_PATH.$index.$field" to value }

private fun writeNewLinkFraming(name: String, framing: SerialFraming): Boolean {
    if (framing == SerialFraming()) return true
    val row = currentRows().firstOrNull { it.name == name } ?: return false
    framingWrites(row.index, framing).forEach { (path, value) -> Qgc.set(path, value) }
    LinkCommands.commitConfigurations()
    return true
}

private fun addServers(name: String, servers: List<String>): Boolean {
    if (servers.isEmpty()) return true
    val row = currentRows().firstOrNull { it.name == name } ?: return false
    servers.forEach { Qgc.invoke("$LINKS_PATH.${row.index}.addHost", it) }
    LinkCommands.commitConfigurations()
    return true
}

private fun writeNewLinkFlags(name: String, autoConnect: Boolean, highLatency: Boolean) {
    if (!autoConnect && !highLatency) return
    val row = currentRows().firstOrNull { it.name == name } ?: return
    linkFlagWrites(row.index, autoConnect, highLatency).forEach { (path, value) -> Qgc.set(path, value) }
    LinkCommands.commitConfigurations()
}

private const val BLUETOOTH_POLL_MS = 1000L

@Composable
private fun BluetoothPicker(chosen: BluetoothDeviceChoice?, onPick: (BluetoothDeviceChoice) -> Unit) {
    var state by remember { mutableStateOf(BluetoothState(available = false, scanning = false, devices = emptyList())) }
    LaunchedEffect(Unit) {
        while (true) {
            state = withContext(Dispatchers.Default) { bluetoothState(Qgc.get(LINKS_VIEW)) }
            delay(BLUETOOTH_POLL_MS)
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Device: ${chosen?.name.orEmpty()}", style = MaterialTheme.typography.bodySmall)
        Text("Address: ${chosen?.address.orEmpty()}", style = MaterialTheme.typography.bodySmall)
        Text("Bluetooth devices", style = MaterialTheme.typography.titleSmall)
        state.devices.forEach { device ->
            FilterChip(selected = device == chosen, onClick = { onPick(device) }, label = { Text(device.name) })
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(enabled = !state.scanning, onClick = { offMainInOrder { LinkCommands.scanBluetooth(true) } }) { Text("Scan") }
            OutlinedButton(enabled = state.scanning, onClick = { offMainInOrder { LinkCommands.scanBluetooth(false) } }) { Text("Stop") }
        }
    }
}

@Composable
private fun EditLinkPage(row: LinkRow, onDismiss: () -> Unit, onSaved: () -> Unit, modifier: Modifier = Modifier) {
    var name by remember { mutableStateOf(row.name) }
    var host by remember { mutableStateOf(row.host) }
    var port by remember { mutableStateOf(row.port.toString()) }
    var portName by remember { mutableStateOf(row.portName) }
    var baud by remember { mutableIntStateOf(if (row.baud > 0) row.baud else DEFAULT_BAUD) }
    var baudsOpen by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var autoConnect by remember { mutableStateOf(row.autoConnect) }
    var highLatency by remember { mutableStateOf(row.highLatency) }
    var framing by remember { mutableStateOf(row.framing) }
    var advanced by remember { mutableStateOf(false) }
    var logFile by remember { mutableStateOf(row.filename) }
    var device by remember { mutableStateOf<BluetoothDeviceChoice?>(null) }
    val scope = rememberCoroutineScope()
    val context = androidx.compose.ui.platform.LocalContext.current
    val cancel: () -> Unit = {
        if (row.editing == "logFile") one.aircast.android.bridge.offMainDetached { pruneReplayFolder(context, row.name, row.filename) }
        onDismiss()
    }
    val logPicker = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = withContext(Dispatchers.IO) { stagedReplayLog(context, row.name, chosen) }
            if (staged == null) error = "That file could not be read." else logFile = staged
        }
    }
    val linksJson by qgcPath(LINKS_VIEW)
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    var portsOpen by remember { mutableStateOf(false) }
    val udpListen by one.aircast.android.bridge.qgcValue(UDP_LISTEN_PORT)
    val udpDefault = (udpListen as? Number)?.toInt()?.takeIf { it in 1..65535 }?.toString() ?: DEFAULT_PORT

    BackHandler(onBack = cancel)
    OverridePageHeading("Edit ${row.name}", cancel)
    Column(modifier.fillMaxSize()) {
        if (LocalPageHeading.current == null) PageTopBar("Edit ${row.name}", "Back to links", cancel)
            Column(
                Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Name (optional)") },
                    singleLine = true,
                )
                LinkOptions(autoConnect, highLatency, { autoConnect = it }, { highLatency = it })
                if (row.editing == "hostAndPort") {
                    OutlinedTextField(
                        value = host,
                        onValueChange = { host = it },
                        label = { Text("Address") },
                        singleLine = true,
                    )
                }
                if (row.editing == "device") {
                    BluetoothPicker(device) { device = it }
                } else if (row.editing == "logFile") {
                    Text(logFile.substringAfterLast('/').ifBlank { "No log file chosen" }, style = MaterialTheme.typography.bodyMedium)
                    OutlinedButton(onClick = { logPicker.launch(arrayOf("*/*")) }) { Text("Choose log file") }
                } else if (row.editing == "serial") {
                    Box {
                        OutlinedButton(onClick = { portsOpen = true }) {
                            Text(portLabel(ports, portName).ifBlank { portName.ifBlank { "Choose a port" } })
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
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = advanced, onCheckedChange = { advanced = it })
                        Text("Advanced settings")
                    }
                    if (advanced) SerialFramingControls(framing) { framing = it }
                } else {
                    OutlinedTextField(
                        value = port,
                        onValueChange = { port = it },
                        label = { Text("Port") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                }
                if (row.editing == "portOnly") {
                    UdpAutoConnectSwitch()
                    UdpServers(row.index, row.servers)
                }
                error?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
            }
        Row(
            Modifier.fillMaxWidth().padding(16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        ) {
            TextButton(onClick = cancel) { Text("Cancel") }
            Button(
                onClick = {
                    val udp = row.editing == "portOnly"
                    val parsed = port.ifBlank { if (udp) udpDefault else port }.toIntOrNull() ?: 0
                    val invalid = when {
                        row.editing == "serial" || row.editing == "device" -> null
                        row.editing == "logFile" -> "Choose a log file.".takeIf { logFile.isBlank() }
                        udp -> linkFormError("udp", "", port, udpDefault)
                        parsed !in 1..65535 -> "Port must be a number between 1 and 65535."
                        else -> null
                    }
                    error = invalid
                    if (invalid == null) {
                        scope.launch {
                            val rows = withContext(Dispatchers.Default) { currentRows() }
                            val live = rows.firstOrNull { it.index == row.index }
                            if (live == null || live.connected) {
                                error = "Disconnect the link before changing its settings."
                                return@launch
                            }
                            val resolved = name.trim().ifBlank {
                                uniqueLinkName(
                                    if (row.editing == "device") device?.name ?: row.name else editedLinkSuggestion(row.editing, host, port, portLabel(ports, portName)),
                                    rows.filter { it.index != row.index }.map { it.name },
                                )
                            }
                            withContext(Dispatchers.Default) {
                                editWrites(row.editing, resolved, host, parsed, portName, baud, autoConnect, highLatency, framing, logFile)
                                    .forEach { (field, value) ->
                                        // qtpaths: links.linkConfigurations.0.name, links.linkConfigurations.0.autoConnect, links.linkConfigurations.0.highLatency, links.linkConfigurations.0.host, links.linkConfigurations.0.port, links.linkConfigurations.0.localPort, links.linkConfigurations.0.portName, links.linkConfigurations.0.baud, links.linkConfigurations.0.dataBits, links.linkConfigurations.0.stopBits, links.linkConfigurations.0.parity, links.linkConfigurations.0.flowControl, links.linkConfigurations.0.filename
                                        Qgc.set("$LINKS_PATH.${row.index}.$field", value)
                                    }
                                // qtpaths: links.linkConfigurations.0.setDeviceByAddress
                                device?.let { Qgc.invoke("$LINKS_PATH.${row.index}.setDeviceByAddress", it.address) }
                                LinkCommands.commitConfigurations()
                                LinkCommands.connect("@$LINKS_PATH.${row.index}")
                            }
                            if (row.editing == "logFile") withContext(Dispatchers.IO) { pruneReplayFolder(context, row.name, logFile) }
                            onSaved()
                        }
                    }
                },
            ) { Text("Save & Connect") }
        }
    }
}

@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
private fun AddLinkPage(onDismiss: () -> Unit, onAdded: () -> Unit, modifier: Modifier = Modifier) {
    var type by remember { mutableStateOf("udp") }
    var name by remember { mutableStateOf("") }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("") }
    var servers by remember { mutableStateOf(emptyList<String>()) }
    var replayLog by remember { mutableStateOf<android.net.Uri?>(null) }
    val context = androidx.compose.ui.platform.LocalContext.current
    val logPicker = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) replayLog = uri
    }
    var portName by remember { mutableStateOf("") }
    var baud by remember { mutableIntStateOf(DEFAULT_BAUD) }
    var advancedSerial by remember { mutableStateOf(false) }
    var newFraming by remember { mutableStateOf(SerialFraming()) }
    var portsOpen by remember { mutableStateOf(false) }
    var baudsOpen by remember { mutableStateOf(false) }
    val linksJson by qgcPath(LINKS_VIEW)
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val taken = remember(linksJson) { linkRows(linksJson).map { it.name } }
    val offered = remember(linksJson) { addableLinkTypes(linksJson) }
    val udpListen by one.aircast.android.bridge.qgcValue(UDP_LISTEN_PORT)
    val udpDefault = (udpListen as? Number)?.toInt()?.takeIf { it in 1..65535 }?.toString() ?: DEFAULT_PORT
    var typeChosen by remember { mutableStateOf(false) }
    LaunchedEffect(offered, ports.isNotEmpty()) {
        if (!typeChosen && "serial" in offered && ports.isNotEmpty()) type = "serial"
    }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var autoConnect by remember { mutableStateOf(false) }
    var highLatency by remember { mutableStateOf(false) }
    var mock by remember { mutableStateOf(MockLinkChoices()) }
    var device by remember { mutableStateOf<BluetoothDeviceChoice?>(null) }
    var apiBase by remember { mutableStateOf("") }
    var deviceId by remember { mutableStateOf("") }
    var cloudErrors by remember { mutableStateOf(false) }
    var cloudOffered by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { cloudOffered = withContext(Dispatchers.Default) { accountState(AccountCommands.state()) != null } }
    val choices = offered + listOf(AIRCAST_CLOUD_LINK).filter { cloudOffered && it in linkTypeIds(linksJson) }
    val askBluetooth = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestMultiplePermissions()) { }
    val scope = rememberCoroutineScope()
    val fieldError = error?.takeIf { (type == "udp" || type == "tcp") && it == linkFormError(type, host, port.ifBlank { portFor(type, udpDefault) }, udpDefault) }
        ?.let { linkFormErrorField(type, host, port.ifBlank { portFor(type, udpDefault) }, udpDefault) }

    BackHandler(onBack = onDismiss)
    OverridePageHeading("Add link", onDismiss)
    Column(modifier.fillMaxSize()) {
        if (LocalPageHeading.current == null) PageTopBar("Add link", "Back to links", onDismiss)
            Column(
                Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    choices.forEach { id ->
                        FilterChip(
                            selected = type == id,
                            onClick = {
                                type = id
                                typeChosen = true
                                port = ""
                                if (id == BLUETOOTH_LINK) askBluetooth.launch(bluetoothPermissions(android.os.Build.VERSION.SDK_INT))
                            },
                            label = { Text(linkTypeLabel(id)) },
                        )
                    }
                }
                Text(
                    text = when (type) {
                        "udp" -> "Listens on a port. Add a server address only to send to a " +
                            "specific device."
                        "serial" -> "A radio plugged into this device over USB."
                        BLUETOOTH_LINK -> "A radio paired with or near this device over Bluetooth."
                        AIRCAST_CLOUD_LINK -> "A backup link to the aircraft through your Aircast account."
                        REPLAY_LINK -> "Plays back a saved telemetry log as if the vehicle were connected."
                        MOCK_LINK -> "A simulated vehicle for trying the app without hardware. It is not saved, so it ends when the app restarts."
                        else -> "Calls out to a device that is listening, such as a ground station."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (type != MOCK_LINK) OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Name (optional)") },
                    placeholder = {
                        Text(
                            when (type) {
                                "serial" -> autoSerialName(portLabel(ports, portName))
                                REPLAY_LINK -> REPLAY_LINK_NAME
                                else -> autoLinkName(type, host, port.ifBlank { portFor(type, udpDefault) })
                            },
                        )
                    },
                    singleLine = true,
                )
                if (type == MOCK_LINK) {
                    MockLinkFields(mock) { mock = it }
                } else if (type == REPLAY_LINK) {
                    OutlinedButton(onClick = { logPicker.launch(arrayOf("*/*")) }) {
                        Text(replayLog?.lastPathSegment?.substringAfterLast('/') ?: "Choose a log file")
                    }
                } else if (type == AIRCAST_CLOUD_LINK) {
                    AircastCloudFields(apiBase, deviceId, cloudErrors, { apiBase = it }, { deviceId = it })
                } else if (type == BLUETOOTH_LINK) {
                    BluetoothPicker(device) { device = it }
                } else if (type == "serial" && ports.isEmpty()) {
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
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = advancedSerial, onCheckedChange = { advancedSerial = it })
                        Text("Advanced settings")
                    }
                    if (advancedSerial) SerialFramingControls(newFraming) { newFraming = it }
                } else if (type == "udp") {
                    OutlinedTextField(
                        value = port,
                        onValueChange = { port = it; if (fieldError == "port") error = null },
                        label = { Text("Port") },
                        placeholder = { Text(udpDefault) },
                        isError = fieldError == "port",
                        supportingText = { Text(if (fieldError == "port") error.orEmpty() else "Leave blank for $udpDefault") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    )
                    UdpAutoConnectSwitch()
                    ServerList(
                        servers,
                        onAdd = { typed ->
                            val local = port.ifBlank { udpDefault }
                            error = if (udpServer(typed, local) == null) "Enter a server as an address, or address:port." else null
                            servers = withServer(servers, typed, local)
                        },
                        onRemove = { gone -> servers = servers.filterNot { it == gone } },
                    )
                } else {
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        OutlinedTextField(
                            value = host,
                            onValueChange = { host = it; if (fieldError == "host") error = null },
                            label = { Text(if (type == "tcp") "Host" else "Host (optional)", maxLines = 1) },
                            isError = fieldError == "host",
                            supportingText = { if (fieldError == "host") Text(error.orEmpty()) },
                            singleLine = true,
                            modifier = Modifier.weight(1.6f),
                        )
                        OutlinedTextField(
                            value = port,
                            onValueChange = { port = it; if (fieldError == "port") error = null },
                            label = { Text("Port") },
                            placeholder = { Text(portFor(type, udpDefault)) },
                            isError = fieldError == "port",
                            supportingText = { Text(if (fieldError == "port") error.orEmpty() else "Default ${portFor(type, udpDefault)}") },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
                if (type != MOCK_LINK) LinkOptions(autoConnect, highLatency, { autoConnect = it }, { highLatency = it })
                if (fieldError == null) error?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
            }
        Row(
            Modifier.fillMaxWidth().padding(16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        ) {
            TextButton(onClick = onDismiss) { Text("Cancel") }
            Button(
                enabled = !busy,
                onClick = {
                    cloudErrors = type == AIRCAST_CLOUD_LINK
                    val invalid = if (type == MOCK_LINK) {
                        null
                    } else if (type == AIRCAST_CLOUD_LINK) {
                        if (cloudApiBaseValid(apiBase) && cloudDeviceValid(deviceId)) null else ""
                    } else if (type == BLUETOOTH_LINK) {
                        if (device == null) "Pick a Bluetooth device." else null
                    } else if (type == REPLAY_LINK) {
                        if (replayLog == null) "Choose a log file to replay." else null
                    } else if (type == "serial") {
                        serialFormError(portName, baud, taken, name, ports.isNotEmpty())
                    } else {
                        linkFormError(type, host, port.ifBlank { portFor(type, udpDefault) }, udpDefault)
                    }
                    error = invalid?.ifBlank { null }
                    if (invalid == null) {
                        busy = true
                        val chosen = name.trim().ifBlank {
                            uniqueLinkName(
                                when (type) {
                                    AIRCAST_CLOUD_LINK -> AIRCAST_CLOUD_NAME
                                    REPLAY_LINK -> REPLAY_LINK_NAME
                                    BLUETOOTH_LINK -> device?.name.orEmpty()
                                    "serial" -> autoSerialName(portLabel(ports, portName))
                                    else -> autoLinkName(type, host, port.ifBlank { portFor(type, udpDefault) })
                                },
                                taken,
                            )
                        }
                        scope.launch {
                            val added = withContext(Dispatchers.Default) {
                                if (type == MOCK_LINK) return@withContext LinkCommands.startMock(mockLinkArguments(mock))
                                if (type == AIRCAST_CLOUD_LINK) {
                                    AccountCommands.setApiBase(apiBase)
                                    LinkCommands.createAircastCloud(chosen, apiBase, deviceId)
                                } else if (type == REPLAY_LINK) {
                                    val staged = stagedReplayLog(context, chosen, replayLog!!)
                                    val created = staged != null && LinkCommands.createLogReplay(chosen, staged)
                                    if (!created) staged?.let { java.io.File(it).delete() }
                                    created && connectNamed(chosen)
                                } else if (type == BLUETOOTH_LINK) {
                                    val picked = device!!
                                    LinkCommands.createBluetooth(chosen, picked.name, picked.address)
                                } else if (type == "serial") {
                                    LinkCommands.createSerial(chosen, portName, baud) &&
                                        writeNewLinkFraming(chosen, newFraming) &&
                                        connectNamed(chosen)
                                } else if (type == "udp") {
                                    Qgc.invokeResult(
                                        "links.createAndConnectLink", type, chosen, "", port.ifBlank { udpDefault }.toInt(),
                                    ) == true && addServers(chosen, servers)
                                } else {
                                    Qgc.invokeResult(
                                        "links.createAndConnectLink", type, chosen, host, port.ifBlank { portFor(type, udpDefault) }.toInt(),
                                    ) == true
                                }.also { created -> if (created) writeNewLinkFlags(chosen, autoConnect, highLatency) }
                            }
                            busy = false
                            if (added) {
                                onAdded()
                            } else {
                                error = if (type == MOCK_LINK) "Could not start the simulated vehicle." else "Could not add that link. The name may already be in use."
                            }
                        }
                    }
                },
            ) { Text(if (busy) "Adding…" else if (type == MOCK_LINK) "Start" else "Add and connect") }
        }
    }
}

private const val LINK_SETTLE_MS = 4000L

internal fun linkFailure(action: String, done: Boolean): String? =
    if (done) null else "Could not $action that link."

private fun currentRows(): List<LinkRow> = linkRows(Qgc.get(LINKS_VIEW))

private fun connectNamed(name: String): Boolean {
    val row = currentRows().firstOrNull { it.name == name } ?: return false
    LinkCommands.connect("@$LINKS_PATH.${row.index}")
    return true
}

@Composable
fun LinksScreen(modifier: Modifier = Modifier, footer: @Composable () -> Unit = {}) {
    val view by qgcPath(LINKS_VIEW)
    val hasVehicle = hasVehicle()
    val rows = linkRows(view)
    val auto = autoLinks(view)
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

    if (adding) {
        AddLinkPage(onDismiss = { adding = false }, onAdded = { adding = false }, modifier = modifier)
        return
    }
    editing?.let { row ->
        EditLinkPage(row = row, onDismiss = { editing = null }, onSaved = { editing = null }, modifier = modifier)
        return
    }

    Box(modifier.fillMaxSize()) {
    Column(Modifier.fillMaxSize()) {
    notice?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
        )
    }

    val listState = rememberLazyListState()
    if (!listState.canScrollBackward) listState.requestScrollToItem(0)
    LazyColumn(Modifier.weight(1f), state = listState) {
        val connected = rows.filter { it.connected }
        val saved = rows.filterNot { it.connected }
        if (rows.isEmpty() && auto.isEmpty()) {
            item(key = "empty") {
                FootNote(
                    "No links saved. Aircast finds a vehicle on the network by itself, so you " +
                        "only need to add one when that does not reach it.",
                )
            }
        }
        listOf("Connected" to connected, "Saved" to saved).forEach { (title, group) ->
            if (group.isNotEmpty() || (title == "Connected" && auto.isNotEmpty())) item(key = "head$title") { SectionHeader(title) }
            items(group, key = { "link${it.name}" }) { row ->
                LinkRowItem(
                    row = row,
                    onConnect = {
                        attempt(
                            action = "connect",
                            settled = { currentRows().getOrNull(row.index)?.connected == true },
                        ) { LinkCommands.connect("@$LINKS_PATH.${row.index}") }
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
            }
            if (title == "Connected") items(auto, key = { "auto${it.name}" }) { link ->
                AutoLinkItem(
                    link,
                    onStop = {
                        attempt(
                            action = "stop",
                            settled = { autoLinks(Qgc.get(LINKS_VIEW)).none { it.name == link.name } },
                        ) { Qgc.invoke("$LINKS_PATH.${link.index}.link.disconnect") }
                    }.takeIf { link.type == MOCK_LINK },
                )
            }
        }

        item(key = "footer") { footer() }
        item(key = "fabSpace") { Spacer(Modifier.height(88.dp)) }
    }

    }
    ExtendedFloatingActionButton(
        onClick = { adding = true },
        icon = { Icon(painterResource(R.drawable.ic_add), null) },
        text = { Text("Add link\u2026") },
        containerColor = MaterialTheme.colorScheme.primaryContainer,
        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp).semantics { contentDescription = "Add link\u2026" },
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
                    ) { LinkCommands.remove("@$LINKS_PATH.${row.index}") }
                    confirmingRemove = null
                }) { Text("Remove") }
            },
            dismissButton = {
                TextButton(onClick = { confirmingRemove = null }) { Text("Cancel") }
            },
        )
    }
}

internal const val REMEDY_EDIT_ADDRESS = "editAddress"

internal fun remedyText(remedy: String): String? = when (remedy) {
    REMEDY_EDIT_ADDRESS -> "Nothing is listening at that address. Retrying will not help until it is changed."
    else -> null
}

@Composable
private fun SerialFramingControls(framing: SerialFraming, onChange: (SerialFraming) -> Unit) {
    Column {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Checkbox(checked = framing.flowControl != 0, onCheckedChange = { onChange(framing.copy(flowControl = if (it) 1 else 0)) })
            Text("Enable flow control")
        }
        FramingPicker("Parity", PARITY_CHOICES.firstOrNull { it.second == framing.parity }?.first.orEmpty(), PARITY_CHOICES.map { it.first }) {
            onChange(framing.copy(parity = PARITY_CHOICES[it].second))
        }
        FramingPicker("Data Bits", framing.dataBits.toString(), DATA_BITS_CHOICES.map { it.toString() }) {
            onChange(framing.copy(dataBits = DATA_BITS_CHOICES[it]))
        }
        FramingPicker("Stop Bits", framing.stopBits.toString(), STOP_BITS_CHOICES.map { it.toString() }) {
            onChange(framing.copy(stopBits = STOP_BITS_CHOICES[it]))
        }
    }
}

@Composable
private fun FramingPicker(label: String, shown: String, choices: List<String>, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(onClick = { open = true }) { Text(shown) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                choices.forEachIndexed { index, choice ->
                    DropdownMenuItem(text = { Text(choice) }, onClick = { onPick(index); open = false })
                }
            }
        }
    }
}

@Composable
private fun UdpAutoConnectSwitch() {
    val checked by one.aircast.android.bridge.qgcBool(one.aircast.android.bridge.settingControl(AUTO_CONNECT_UDP))
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text("Auto connect to UDP devices")
            Text("Applies to every UDP link. Turn it off for the best performance on this one.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Switch(checked = checked, onCheckedChange = { on -> offMainInOrder { Qgc.set(AUTO_CONNECT_UDP, on) } })
    }
}

@Composable
private fun UdpServers(index: Int, initial: List<String>) {
    val scope = rememberCoroutineScope()
    var servers by remember { mutableStateOf(initial) }

    fun change(action: String, host: String) {
        scope.launch {
            servers = withContext(Dispatchers.Default) {
                // qtpaths: links.linkConfigurations.0.addHost, links.linkConfigurations.0.removeHost
                Qgc.invoke("$LINKS_PATH.$index.$action", host)
                currentRows().firstOrNull { it.index == index }?.servers ?: servers
            }
        }
    }

    ServerList(servers, onAdd = { change("addHost", it) }, onRemove = { change("removeHost", it) })
}

@Composable
private fun ServerList(servers: List<String>, onAdd: (String) -> Unit, onRemove: (String) -> Unit) {
    var typed by remember { mutableStateOf("") }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Server addresses (optional)", style = MaterialTheme.typography.labelLarge)
        servers.forEach { server ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(server, modifier = Modifier.weight(1f))
                TextButton(onClick = { onRemove(server) }) { Text("Remove") }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = typed,
                onValueChange = { typed = it },
                placeholder = { Text("Example: 127.0.0.1:14550") },
                singleLine = true,
                modifier = Modifier.weight(1f),
            )
            TextButton(enabled = typed.isNotBlank(), onClick = {
                onAdd(typed.trim())
                typed = ""
            }) { Text("Add server") }
        }
    }
}
