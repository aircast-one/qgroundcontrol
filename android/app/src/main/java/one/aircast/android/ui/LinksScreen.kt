package one.aircast.android.ui

import androidx.compose.foundation.background
import one.aircast.android.bridge.LinkCommands
import one.aircast.android.bridge.AccountCommands
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.Role
import androidx.compose.foundation.selection.toggleable
import one.aircast.android.R
import one.aircast.map.aircast

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.rememberScrollState
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

internal enum class LinkType(val id: String) {
    Udp("udp"),
    Tcp("tcp"),
    Serial("serial"),
    Bluetooth("bluetooth"),
    LogReplay("logReplay"),
    Mock("mock"),
    AircastCloud("aircastCloud"),
    Other(""),
    ;

    companion object {
        fun from(id: String): LinkType = entries.firstOrNull { it.id == id && it != Other } ?: Other
    }
}

internal enum class LinkEditing(val id: String) {
    HostAndPort("hostAndPort"),
    PortOnly("portOnly"),
    Serial("serial"),
    LogFile("logFile"),
    Device("device"),
    None(""),
    ;

    companion object {
        fun from(id: String): LinkEditing = entries.firstOrNull { it.id == id && it != None } ?: None
    }
}

internal data class LinkRow(
    val index: Int,
    val name: String,
    val statusLine: String,
    val goneQuiet: Boolean = false,
    val connected: Boolean,
    val heard: Boolean,
    val lastError: String,
    val errorRemedy: String = "",
    val editing: LinkEditing = LinkEditing.None,
    val host: String = "",
    val port: Int = 0,
    val portName: String = "",
    val baud: Int = 0,
    val framing: SerialFraming = SerialFraming(),
    val servers: List<String> = emptyList(),
    val autoConnect: Boolean = false,
    val highLatency: Boolean = false,
    val type: LinkType = LinkType.Other,
    val filename: String = "",
)

internal data class AutoLink(val name: String, val summary: String, val heard: Boolean, val type: LinkType = LinkType.Other, val index: Int = 0)

internal fun autoLinks(view: JSONObject?): List<AutoLink> {
    val links = view?.optJSONArray("links") ?: return emptyList()
    return (0 until links.length()).mapNotNull { links.optJSONObject(it) }
        .filter { it.optBoolean("dynamic") && it.optBoolean("connected") }
        .map { AutoLink(it.optText("name"), it.optText("displaySummary"), it.optBoolean("heardVehicle"), LinkType.from(it.optText("type")), it.optInt("index")) }
}

internal fun autoLinkStatus(link: AutoLink): String = if (link.heard) "Vehicle" else "Listening"

internal fun autoLinkSubtitle(link: AutoLink): String =
    if (link.type == LinkType.Mock) "Simulated" else listOf(link.summary, "automatic").filter { it.isNotBlank() }.joinToString(" \u00b7 ")

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
                editing = LinkEditing.from(link.optText("editing")),
                host = link.optText("host"),
                port = link.optInt("port"),
                portName = link.optText("portName"),
                baud = link.optInt("baud"),
                framing = SerialFraming(link.optInt("dataBits", 8), link.optInt("stopBits", 1), link.optInt("parity", 0), link.optInt("flowControl", 0)),
                servers = link.optJSONArray("hostList")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
                autoConnect = link.optBoolean("autoConnect"),
                highLatency = link.optBoolean("highLatency"),
                type = LinkType.from(link.optText("type")),
                filename = link.optText("filename"),
            )
        }
    }
}

@androidx.annotation.DrawableRes
internal fun linkIcon(type: LinkType): Int = when (type) {
    LinkType.Serial -> R.drawable.ic_usb
    LinkType.Udp -> R.drawable.ic_wifi
    LinkType.Bluetooth -> R.drawable.ic_bluetooth
    LinkType.Tcp -> R.drawable.ic_lan
    LinkType.AircastCloud -> R.drawable.ic_cloud
    LinkType.LogReplay -> R.drawable.ic_history
    LinkType.Mock -> R.drawable.ic_science
    LinkType.Other -> R.drawable.ic_link
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

internal fun linkIsEditable(row: LinkRow): Boolean = !row.connected && row.editing != LinkEditing.None

internal val CREATABLE_LINK_TYPES = listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial)
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

internal fun addableLinkTypes(view: JSONObject?): List<LinkType> {
    if (view?.optJSONArray("linkTypeIds") == null) return CREATABLE_LINK_TYPES
    val served = linkTypeIds(view)
    val bluetooth = listOf(LinkType.Bluetooth).filter { it in served && bluetoothState(view).available }
    return CREATABLE_LINK_TYPES.filter { it in served }.ifEmpty { CREATABLE_LINK_TYPES } + bluetooth + listOf(LinkType.LogReplay, LinkType.Mock).filter { it in served }
}

internal fun linkTypeIds(view: JSONObject?): Set<LinkType> {
    val listed = view?.optJSONArray("linkTypeIds") ?: return emptySet()
    return (0 until listed.length()).map { LinkType.from(listed.optString(it)) }.toSet()
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
    editing: LinkEditing,
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
    LinkEditing.HostAndPort -> listOf("host" to host, "port" to port)
    LinkEditing.PortOnly -> listOf("localPort" to port)
    LinkEditing.LogFile -> listOf("filename" to logFile)
    LinkEditing.Serial -> listOf(
        "portName" to portName,
        "baud" to baud,
        "dataBits" to framing.dataBits,
        "stopBits" to framing.stopBits,
        "parity" to framing.parity,
        "flowControl" to framing.flowControl,
    )
    else -> emptyList()
}

internal fun linkTypeLabel(type: LinkType): String = when (type) {
    LinkType.Serial -> "Serial"
    LinkType.Bluetooth -> "Bluetooth"
    LinkType.AircastCloud -> "Aircast Cloud"
    LinkType.LogReplay -> "Log replay"
    LinkType.Mock -> "Simulated"
    else -> type.id.uppercase()
}

internal fun autoLinkName(type: LinkType, host: String, port: String): String = when {
    type == LinkType.Udp -> "UDP $port"
    host.isBlank() -> type.id.uppercase()
    else -> "${type.id.uppercase()} $host:$port"
}

internal fun uniqueLinkName(base: String, taken: List<String>): String =
    (listOf(base) + (2..taken.size + 2).map { "$base ($it)" }).first { it !in taken }

internal fun editedLinkSuggestion(editing: LinkEditing, host: String, port: String, portLabel: String): String = when (editing) {
    LinkEditing.Serial -> autoSerialName(portLabel)
    LinkEditing.HostAndPort -> autoLinkName(LinkType.Tcp, host, port)
    else -> autoLinkName(LinkType.Udp, host, port)
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

internal fun portFor(type: LinkType, udpDefault: String): String = if (type == LinkType.Tcp) DEFAULT_TCP_PORT else udpDefault

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

internal fun linkFormError(type: LinkType, host: String, port: String, udpDefault: String = DEFAULT_PORT): String? {
    val parsed = port.ifBlank { if (type == LinkType.Udp) udpDefault else port }.toIntOrNull()
    return when {
        type == LinkType.Udp && (parsed == null || parsed !in 1..65535) -> "Enter a port between 1 and 65535, or leave it blank for $udpDefault"
        parsed == null || parsed !in 1..65535 -> "Port must be a number between 1 and 65535."
        type == LinkType.Tcp && host.isBlank() -> "A TCP link needs the address of the device to call."
        else -> null
    }
}

internal fun linkFormErrorField(type: LinkType, host: String, port: String, udpDefault: String = DEFAULT_PORT): String? {
    val parsed = port.ifBlank { if (type == LinkType.Udp) udpDefault else port }.toIntOrNull()
    return when {
        parsed == null || parsed !in 1..65535 -> "port"
        type == LinkType.Tcp && host.isBlank() -> "host"
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
            val primary: @Composable () -> Unit = {
                DropdownMenuItem(
                    text = { Text(primaryLinkAction(row.connected, fixByEditing)) },
                    onClick = {
                        menuOpen = false
                        if (row.connected) onDisconnect() else onConnect()
                    },
                )
            }
            val edit: @Composable () -> Unit = {
                DropdownMenuItem(
                    text = { Text("Edit link") },
                    enabled = linkIsEditable(row),
                    onClick = {
                        menuOpen = false
                        onEdit()
                    },
                )
            }
            if (fixByEditing) edit() else primary()
            if (fixByEditing) primary() else edit()
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

internal fun primaryLinkAction(connected: Boolean, fixByEditing: Boolean): String = when {
    connected -> "Disconnect"
    fixByEditing -> "Try again"
    else -> "Connect"
}

@Composable
private fun AdvancedDisclosure(open: Boolean, onToggle: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().toggleable(value = open, role = Role.Button) { onToggle() }.heightIn(min = 48.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("Advanced", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
        Icon(
            if (open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = if (open) "Hide advanced options" else "Show advanced options",
        )
    }
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

internal data class LinkEdit(
    val name: String,
    val host: String,
    val port: String,
    val portName: String,
    val baud: Int,
    val autoConnect: Boolean,
    val highLatency: Boolean,
    val framing: SerialFraming,
    val logFile: String,
)

internal fun linkEdit(row: LinkRow): LinkEdit = LinkEdit(
    name = row.name,
    host = row.host,
    port = row.port.toString(),
    portName = row.portName,
    baud = if (row.baud > 0) row.baud else DEFAULT_BAUD,
    autoConnect = row.autoConnect,
    highLatency = row.highLatency,
    framing = row.framing,
    logFile = row.filename,
)

internal fun editShowsAdvanced(row: LinkRow): Boolean =
    row.autoConnect || row.highLatency || row.servers.isNotEmpty() || row.framing != SerialFraming()

internal fun editedPort(editing: LinkEditing, port: String, udpDefault: String): Int =
    port.ifBlank { if (editing == LinkEditing.PortOnly) udpDefault else port }.toIntOrNull() ?: 0

internal fun editError(editing: LinkEditing, edit: LinkEdit, udpDefault: String): String? = when (editing) {
    LinkEditing.Serial, LinkEditing.Device, LinkEditing.None -> null
    LinkEditing.LogFile -> "Choose a log file.".takeIf { edit.logFile.isBlank() }
    LinkEditing.PortOnly -> linkFormError(LinkType.Udp, "", edit.port, udpDefault)
    LinkEditing.HostAndPort -> "Port must be a number between 1 and 65535.".takeIf { editedPort(editing, edit.port, udpDefault) !in 1..65535 }
}

internal fun editSuggestion(row: LinkRow, edit: LinkEdit, device: BluetoothDeviceChoice?, ports: List<SerialPortChoice>): String =
    if (row.editing == LinkEditing.Device) device?.name ?: row.name else editedLinkSuggestion(row.editing, edit.host, edit.port, portLabel(ports, edit.portName))

internal const val EDIT_WHILE_CONNECTED = "Disconnect this link to change it."

private fun applyEdit(index: Int, writes: List<Pair<String, Any>>, device: BluetoothDeviceChoice?) {
    writes.forEach { (field, value) ->
        // qtpaths: links.linkConfigurations.0.name, links.linkConfigurations.0.autoConnect, links.linkConfigurations.0.highLatency, links.linkConfigurations.0.host, links.linkConfigurations.0.port, links.linkConfigurations.0.localPort, links.linkConfigurations.0.portName, links.linkConfigurations.0.baud, links.linkConfigurations.0.dataBits, links.linkConfigurations.0.stopBits, links.linkConfigurations.0.parity, links.linkConfigurations.0.flowControl, links.linkConfigurations.0.filename
        Qgc.set("$LINKS_PATH.$index.$field", value)
    }
    // qtpaths: links.linkConfigurations.0.setDeviceByAddress
    device?.let { Qgc.invoke("$LINKS_PATH.$index.setDeviceByAddress", it.address) }
    LinkCommands.commitConfigurations()
    LinkCommands.connect("@$LINKS_PATH.$index")
}

@Composable
private fun LinkPortField(label: String, value: String, error: String?, enabled: Boolean, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        enabled = enabled,
        isError = error != null,
        supportingText = error?.let { { Text(it) } },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun LinkHostField(value: String, error: String?, enabled: Boolean, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text("Host address") },
        placeholder = { Text("Example: 192.168.1.10") },
        enabled = enabled,
        isError = error != null,
        supportingText = error?.let { { Text(it) } },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun SerialPortFields(ports: List<SerialPortChoice>, bauds: List<Int>, portName: String, baud: Int, enabled: Boolean, onPort: (String) -> Unit, onBaud: (Int) -> Unit) {
    val wide = Modifier.fillMaxWidth()
    ChoiceField("Port", portLabel(ports, portName).ifBlank { portName.ifBlank { "Choose a port" } }, ports.map { it.label }, wide, enabled) { onPort(ports[it].port) }
    ChoiceField("Baud rate", "$baud", bauds.map(Int::toString), wide, enabled) { onBaud(bauds[it]) }
}

@Composable
private fun EditLinkFields(
    editing: LinkEditing,
    edit: LinkEdit,
    enabled: Boolean,
    ports: List<SerialPortChoice>,
    bauds: List<Int>,
    device: BluetoothDeviceChoice?,
    onDevice: (BluetoothDeviceChoice) -> Unit,
    onPickLog: () -> Unit,
    onEdit: (LinkEdit) -> Unit,
) {
    when (editing) {
        LinkEditing.Device -> BluetoothPicker(device, onDevice)
        LinkEditing.LogFile -> {
            Text(edit.logFile.substringAfterLast('/').ifBlank { "No log file chosen" }, style = MaterialTheme.typography.bodyLarge)
            OutlinedButton(enabled = enabled, onClick = onPickLog, modifier = Modifier.fillMaxWidth()) { Text("Choose another log file") }
        }
        LinkEditing.Serial -> SerialPortFields(ports, bauds, edit.portName, edit.baud, enabled, { onEdit(edit.copy(portName = it)) }, { onEdit(edit.copy(baud = it)) })
        LinkEditing.HostAndPort -> {
            LinkHostField(edit.host, null, enabled) { onEdit(edit.copy(host = it)) }
            LinkPortField("Port", edit.port, null, enabled) { onEdit(edit.copy(port = it)) }
        }
        LinkEditing.PortOnly, LinkEditing.None ->
            LinkPortField(if (editing == LinkEditing.PortOnly) "Listening port" else "Port", edit.port, null, enabled) { onEdit(edit.copy(port = it)) }
    }
}

@Composable
private fun EditLinkPage(row: LinkRow, onDismiss: () -> Unit, onSaved: () -> Unit, modifier: Modifier = Modifier) {
    var edit by remember { mutableStateOf(linkEdit(row)) }
    var device by remember { mutableStateOf<BluetoothDeviceChoice?>(null) }
    var advanced by remember { mutableStateOf(editShowsAdvanced(row)) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val context = androidx.compose.ui.platform.LocalContext.current
    val cancel: () -> Unit = {
        if (row.editing == LinkEditing.LogFile) one.aircast.android.bridge.offMainDetached { pruneReplayFolder(context, row.name, row.filename) }
        onDismiss()
    }
    val logPicker = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = withContext(Dispatchers.IO) { stagedReplayLog(context, row.name, chosen) }
            if (staged == null) error = "That file could not be read." else edit = edit.copy(logFile = staged)
        }
    }
    val linksJson by qgcPath(LINKS_VIEW)
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    val udpListen by one.aircast.android.bridge.qgcValue(UDP_LISTEN_PORT)
    val udpDefault = (udpListen as? Number)?.toInt()?.takeIf { it in 1..65535 }?.toString() ?: DEFAULT_PORT
    val editable = linkRows(linksJson).firstOrNull { it.index == row.index }?.connected != true

    BackHandler(onBack = cancel)
    OverridePageHeading("Edit ${row.name}", cancel)
    Column(modifier.fillMaxSize()) {
        if (LocalPageHeading.current == null) PageTopBar("Edit ${row.name}", "Back to links", cancel)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            linkKind(row.type).about.takeIf { it.isNotBlank() }?.let {
                Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (!editable) Text(EDIT_WHILE_CONNECTED, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
            EditLinkFields(row.editing, edit, editable, ports, bauds, device, { device = it }, { logPicker.launch(arrayOf("*/*")) }) { edit = it }
            AdvancedDisclosure(advanced) { advanced = !advanced }
            if (advanced) {
                OutlinedTextField(value = edit.name, onValueChange = { edit = edit.copy(name = it) }, label = { Text("Name") }, enabled = editable, singleLine = true, modifier = Modifier.fillMaxWidth())
                if (row.editing == LinkEditing.PortOnly) UdpServers(row.index, row.servers)
                if (row.editing == LinkEditing.Serial) SerialFramingControls(edit.framing) { edit = edit.copy(framing = it) }
                LinkFlagSwitches(edit.autoConnect, edit.highLatency, { edit = edit.copy(autoConnect = it) }, { edit = edit.copy(highLatency = it) })
            }
            error?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
        }
        Button(
            enabled = editable,
            modifier = Modifier.fillMaxWidth().padding(16.dp).heightIn(min = 52.dp),
            onClick = {
                error = editError(row.editing, edit, udpDefault)
                if (error == null) scope.launch {
                    val rows = withContext(Dispatchers.Default) { currentRows() }
                    if (rows.firstOrNull { it.index == row.index }?.connected != false) {
                        error = EDIT_WHILE_CONNECTED
                        return@launch
                    }
                    val resolved = edit.name.trim().ifBlank { uniqueLinkName(editSuggestion(row, edit, device, ports), rows.filter { it.index != row.index }.map { it.name }) }
                    val writes = editWrites(row.editing, resolved, edit.host, editedPort(row.editing, edit.port, udpDefault), edit.portName, edit.baud, edit.autoConnect, edit.highLatency, edit.framing, edit.logFile)
                    withContext(Dispatchers.Default) { applyEdit(row.index, writes, device) }
                    if (row.editing == LinkEditing.LogFile) withContext(Dispatchers.IO) { pruneReplayFolder(context, row.name, edit.logFile) }
                    onSaved()
                }
            },
        ) { Text("Save and connect") }
    }
}

internal data class LinkKind(val type: LinkType, val title: String, val detail: String, val about: String)

internal val PILOT_LINK_ORDER = listOf(LinkType.Udp, LinkType.Serial, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp)
internal val TOOL_LINK_ORDER = listOf(LinkType.LogReplay, LinkType.Mock)

internal fun linkKind(type: LinkType): LinkKind = when (type) {
    LinkType.Udp -> LinkKind(type, "Wi-Fi or network", "Wi-Fi or Ethernet (UDP)", "Listens on a port for telemetry from the aircraft or its radio.")
    LinkType.Serial -> LinkKind(type, "Serial radio", "USB or built-in telemetry radio", "A telemetry radio plugged in over USB or built into this device.")
    LinkType.Bluetooth -> LinkKind(type, "Bluetooth radio", "Paired telemetry radio", "A radio paired with or near this device over Bluetooth.")
    LinkType.AircastCloud -> LinkKind(type, "Aircast Cloud", "Backup link via your account", "A backup link to the aircraft through your Aircast account.")
    LinkType.Tcp -> LinkKind(type, "Network server (TCP)", "A listening ground station", "Calls out to a device that is listening, such as a ground station or bridge.")
    LinkType.LogReplay -> LinkKind(type, "Replay a flight log", "Play back a saved log", "Plays back a saved telemetry log as if the vehicle were connected.")
    LinkType.Mock -> LinkKind(type, "Simulated vehicle", "Try the app without hardware", "A simulated vehicle for trying the app without hardware. It is not saved, so it ends when the app restarts.")
    LinkType.Other -> LinkKind(type, linkTypeLabel(type), "", "")
}

internal fun pilotLinkKinds(offered: List<LinkType>, radioPlugged: Boolean): List<LinkType> =
    PILOT_LINK_ORDER.filter { it in offered }.sortedBy { if (radioPlugged && it == LinkType.Serial) 0 else 1 }

internal fun toolLinkKinds(offered: List<LinkType>): List<LinkType> = TOOL_LINK_ORDER.filter { it in offered }

internal data class LinkDraft(
    val type: LinkType,
    val name: String = "",
    val host: String = "",
    val port: String = "",
    val servers: List<String> = emptyList(),
    val portName: String = "",
    val baud: Int = DEFAULT_BAUD,
    val framing: SerialFraming = SerialFraming(),
    val autoConnect: Boolean = false,
    val highLatency: Boolean = false,
    val device: BluetoothDeviceChoice? = null,
    val replayLog: String = "",
    val apiBase: String = "",
    val deviceId: String = "",
    val mock: MockLinkChoices = MockLinkChoices(),
)

internal fun LinkDraft.shownPort(udpDefault: String): String = port.ifBlank { portFor(type, udpDefault) }

internal fun LinkDraft.withPluggedPort(ports: List<SerialPortChoice>): LinkDraft =
    copy(portName = portName.ifBlank { ports.singleOrNull()?.port.orEmpty() })

internal fun suggestedLinkName(draft: LinkDraft, ports: List<SerialPortChoice>, udpDefault: String): String = when (draft.type) {
    LinkType.Serial -> autoSerialName(portLabel(ports, draft.portName))
    LinkType.LogReplay -> REPLAY_LINK_NAME
    LinkType.AircastCloud -> AIRCAST_CLOUD_NAME
    LinkType.Bluetooth -> draft.device?.name.orEmpty()
    else -> autoLinkName(draft.type, draft.host, draft.shownPort(udpDefault))
}

internal fun draftError(draft: LinkDraft, udpDefault: String, taken: List<String>, anyPorts: Boolean): String? = when (draft.type) {
    LinkType.Mock -> null
    LinkType.AircastCloud -> "".takeUnless { cloudApiBaseValid(draft.apiBase) && cloudDeviceValid(draft.deviceId) }
    LinkType.Bluetooth -> "Pick a Bluetooth device.".takeIf { draft.device == null }
    LinkType.LogReplay -> "Choose a log file to replay.".takeIf { draft.replayLog.isBlank() }
    LinkType.Serial -> serialFormError(draft.portName, draft.baud, taken, draft.name, anyPorts)
    else -> linkFormError(draft.type, draft.host, draft.shownPort(udpDefault), udpDefault)
}

internal enum class AddOutcome { Connected, SavedNotConnected, Failed }

internal fun addOutcome(created: Boolean, saved: Boolean): AddOutcome = when {
    created -> AddOutcome.Connected
    saved -> AddOutcome.SavedNotConnected
    else -> AddOutcome.Failed
}

internal fun addFailure(type: LinkType): String =
    if (type == LinkType.Mock) "Could not start the simulated vehicle." else "Could not add that link. The name may already be in use."

private fun createLink(draft: LinkDraft, name: String, port: Int, staged: String?): Boolean = when (draft.type) {
    LinkType.Mock -> LinkCommands.startMock(mockLinkArguments(draft.mock))
    LinkType.AircastCloud -> createCloudLink(draft, name)
    LinkType.LogReplay -> staged != null && LinkCommands.createLogReplay(name, staged) && connectNamed(name)
    LinkType.Bluetooth -> draft.device?.let { LinkCommands.createBluetooth(name, it.name, it.address) } == true
    LinkType.Serial -> LinkCommands.createSerial(name, draft.portName, draft.baud) && writeNewLinkFraming(name, draft.framing) && connectNamed(name)
    else -> Qgc.invokeResult("links.createAndConnectLink", draft.type.id, name, if (draft.type == LinkType.Udp) "" else draft.host, port) == true
}

private fun createCloudLink(draft: LinkDraft, name: String): Boolean {
    AccountCommands.setApiBase(draft.apiBase)
    return LinkCommands.createAircastCloud(name, draft.apiBase, draft.deviceId)
}

private fun addLink(context: android.content.Context, draft: LinkDraft, name: String, udpDefault: String): AddOutcome {
    val staged = draft.replayLog.takeIf { draft.type == LinkType.LogReplay }?.let { stagedReplayLog(context, name, android.net.Uri.parse(it)) }
    val created = createLink(draft, name, draft.shownPort(udpDefault).toIntOrNull() ?: 0, staged)
    val saved = draft.type != LinkType.Mock && currentRows().any { it.name == name }
    val outcome = addOutcome(created, saved)
    if (outcome == AddOutcome.Failed) staged?.let { java.io.File(it).delete() }
    if (saved) {
        addServers(name, draft.servers)
        writeNewLinkFlags(name, draft.autoConnect, draft.highLatency)
    }
    return outcome
}

@Composable
private fun AddLinkPage(onDismiss: () -> Unit, onAdded: () -> Unit, modifier: Modifier = Modifier) {
    var type by remember { mutableStateOf<LinkType?>(null) }
    val linksJson by qgcPath(LINKS_VIEW)
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    val offered = remember(linksJson) { addableLinkTypes(linksJson) }
    var cloudOffered by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { cloudOffered = withContext(Dispatchers.Default) { accountState(AccountCommands.state()) != null } }
    val choices = offered + listOf(LinkType.AircastCloud).filter { cloudOffered && it in linkTypeIds(linksJson) }
    val askBluetooth = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestMultiplePermissions()) { }

    type?.let { chosen ->
        AddLinkDetails(chosen, onBack = { type = null }, onAdded = onAdded, modifier = modifier)
        return
    }

    BackHandler(onBack = onDismiss)
    OverridePageHeading("Add link", onDismiss)
    val pick: (LinkType) -> Unit = { picked ->
        if (picked == LinkType.Bluetooth) askBluetooth.launch(bluetoothPermissions(android.os.Build.VERSION.SDK_INT))
        type = picked
    }
    val row: @Composable (LinkKind) -> Unit = { kind ->
        SetupRow(title = kind.title, subtitle = kind.detail, icon = linkIcon(kind.type), onClick = { pick(kind.type) })
    }
    Column(modifier.fillMaxSize()) {
        if (LocalPageHeading.current == null) PageTopBar("Add link", "Back to links", onDismiss)
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
            SectionHeader("Connect over")
            pilotLinkKinds(choices, ports.isNotEmpty()).map(::linkKind).map { row(it) }
            toolLinkKinds(choices).takeIf { it.isNotEmpty() }?.let { tools ->
                SectionHeader("Other")
                tools.map(::linkKind).map { row(it) }
            }
        }
    }
}

@Composable
private fun NewLinkFields(
    draft: LinkDraft,
    ports: List<SerialPortChoice>,
    bauds: List<Int>,
    hostError: String?,
    portError: String?,
    cloudErrors: Boolean,
    onPickLog: () -> Unit,
    onDraft: (LinkDraft) -> Unit,
) {
    when (draft.type) {
        LinkType.Mock -> MockLinkFields(draft.mock) { onDraft(draft.copy(mock = it)) }
        LinkType.LogReplay -> OutlinedButton(onClick = onPickLog, modifier = Modifier.fillMaxWidth()) {
            Text(draft.replayLog.takeIf { it.isNotBlank() }?.let { android.net.Uri.parse(it).lastPathSegment?.substringAfterLast('/') } ?: "Choose a log file")
        }
        LinkType.AircastCloud -> AircastCloudFields(draft.apiBase, draft.deviceId, cloudErrors, { onDraft(draft.copy(apiBase = it)) }, { onDraft(draft.copy(deviceId = it)) })
        LinkType.Bluetooth -> BluetoothPicker(draft.device) { onDraft(draft.copy(device = it)) }
        LinkType.Serial -> if (ports.isEmpty()) {
            Text(
                text = "Nothing is plugged in. Connect a radio over USB and it will appear here.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            SerialPortFields(ports, bauds, draft.portName, draft.baud, true, { onDraft(draft.copy(portName = it)) }, { onDraft(draft.copy(baud = it)) })
        }
        LinkType.Udp -> LinkPortField("Listening port", draft.port, portError, true) { onDraft(draft.copy(port = it)) }
        LinkType.Tcp, LinkType.Other -> {
            LinkHostField(draft.host, hostError, true) { onDraft(draft.copy(host = it)) }
            LinkPortField("Port", draft.port, portError, true) { onDraft(draft.copy(port = it)) }
        }
    }
}

@Composable
private fun NewLinkAdvanced(draft: LinkDraft, suggestedName: String, ports: List<SerialPortChoice>, udpDefault: String, onError: (String?) -> Unit, onDraft: (LinkDraft) -> Unit) {
    OutlinedTextField(
        value = draft.name,
        onValueChange = { onDraft(draft.copy(name = it)) },
        label = { Text("Name") },
        placeholder = { Text(suggestedName) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    if (draft.type == LinkType.Udp) ServerList(
        draft.servers,
        onAdd = { typed ->
            onError("Enter a server as an address, or address:port.".takeIf { udpServer(typed, draft.shownPort(udpDefault)) == null })
            onDraft(draft.copy(servers = withServer(draft.servers, typed, draft.shownPort(udpDefault))))
        },
        onRemove = { gone -> onDraft(draft.copy(servers = draft.servers.filterNot { it == gone })) },
    )
    if (draft.type == LinkType.Serial && ports.isNotEmpty()) SerialFramingControls(draft.framing) { onDraft(draft.copy(framing = it)) }
    LinkFlagSwitches(draft.autoConnect, draft.highLatency, { onDraft(draft.copy(autoConnect = it)) }, { onDraft(draft.copy(highLatency = it)) })
}

@Composable
private fun AddLinkDetails(type: LinkType, onBack: () -> Unit, onAdded: () -> Unit, modifier: Modifier = Modifier) {
    val linksJson by qgcPath(LINKS_VIEW)
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val taken = remember(linksJson) { linkRows(linksJson).map { it.name } }
    val udpListen by one.aircast.android.bridge.qgcValue(UDP_LISTEN_PORT)
    val udpDefault = (udpListen as? Number)?.toInt()?.takeIf { it in 1..65535 }?.toString() ?: DEFAULT_PORT
    var typed by remember { mutableStateOf(LinkDraft(type, port = portFor(type, udpDefault))) }
    val draft = typed.withPluggedPort(ports)
    var advanced by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var cloudErrors by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val context = androidx.compose.ui.platform.LocalContext.current
    val logPicker = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) typed = typed.copy(replayLog = uri.toString())
    }
    val fieldError = error?.takeIf { (type == LinkType.Udp || type == LinkType.Tcp) && it == linkFormError(type, draft.host, draft.shownPort(udpDefault), udpDefault) }
        ?.let { linkFormErrorField(type, draft.host, draft.shownPort(udpDefault), udpDefault) }
    val suggestedName = suggestedLinkName(draft, ports, udpDefault)
    val change: (LinkDraft) -> Unit = { changed ->
        typed = changed
        if (fieldError != null) error = null
    }

    BackHandler(onBack = onBack)
    OverridePageHeading(linkKind(type).title, onBack)
    Column(modifier.fillMaxSize()) {
        if (LocalPageHeading.current == null) PageTopBar(linkKind(type).title, "Back to link types", onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(linkKind(type).about, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            NewLinkFields(draft, ports, bauds, error.takeIf { fieldError == "host" }, error.takeIf { fieldError == "port" }, cloudErrors, { logPicker.launch(arrayOf("*/*")) }, change)
            if (type != LinkType.Mock) {
                AdvancedDisclosure(advanced) { advanced = !advanced }
                if (advanced) NewLinkAdvanced(draft, suggestedName, ports, udpDefault, { error = it }, change)
            }
            if (fieldError == null) error?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
        }
        Button(
            enabled = !busy,
            modifier = Modifier.fillMaxWidth().padding(16.dp).heightIn(min = 52.dp),
            onClick = {
                cloudErrors = type == LinkType.AircastCloud
                val invalid = draftError(draft, udpDefault, taken, ports.isNotEmpty())
                error = invalid?.ifBlank { null }
                if (invalid == null) {
                    busy = true
                    val name = draft.name.trim().ifBlank { uniqueLinkName(suggestedName, taken) }
                    scope.launch {
                        val outcome = withContext(Dispatchers.Default) { addLink(context, draft, name, udpDefault) }
                        busy = false
                        if (outcome == AddOutcome.Failed) error = addFailure(type) else onAdded()
                    }
                }
            },
        ) { Text(if (busy) "Connecting…" else if (type == LinkType.Mock) "Start" else "Connect") }
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
    val navigation = LocalAppNavigation.current
    var adding by remember { mutableStateOf(navigation.addLinkRequested) }
    LaunchedEffect(navigation.addLinkRequested) {
        if (navigation.addLinkRequested) {
            adding = true
            navigation.addLinkRequested = false
        }
    }
    var advanced by androidx.compose.runtime.saveable.rememberSaveable { mutableStateOf(false) }

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
                    "No links saved. A USB cable or telemetry radio connects by itself; " +
                        "add a link for Wi-Fi or a network connection.",
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
                    }.takeIf { link.type == LinkType.Mock },
                )
            }
        }

        item(key = "addLink") {
            Button(onClick = { adding = true }, modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) {
                Icon(painterResource(R.drawable.ic_add), null)
                Text("Add link\u2026", modifier = Modifier.padding(start = 8.dp))
            }
        }
        item(key = "advanced") { AdvancedToggle(advanced) { advanced = !advanced } }
        if (advanced) item(key = "footer") { footer() }
    }

    }
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
    val add = {
        onAdd(typed.trim())
        typed = ""
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        servers.forEach { server ->
            Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(server, modifier = Modifier.weight(1f))
                androidx.compose.material3.IconButton(onClick = { onRemove(server) }) { Icon(painterResource(R.drawable.ic_close), "Remove $server") }
            }
        }
        OutlinedTextField(
            value = typed,
            onValueChange = { typed = it },
            label = { Text("Also send to") },
            placeholder = { Text("127.0.0.1:14550") },
            supportingText = { Text("Optional. One address per device that should get telemetry too.") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = androidx.compose.ui.text.input.ImeAction.Done),
            keyboardActions = androidx.compose.foundation.text.KeyboardActions(onDone = { if (typed.isNotBlank()) add() }),
            trailingIcon = {
                androidx.compose.material3.IconButton(enabled = typed.isNotBlank(), onClick = add) { Icon(painterResource(R.drawable.ic_add), "Add address") }
            },
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
