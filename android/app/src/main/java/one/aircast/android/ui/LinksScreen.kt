package one.aircast.android.ui

import androidx.compose.foundation.background
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
import one.aircast.mapspike.aircast

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
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
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
            )
        }
    }
}

internal fun linkIsEditable(row: LinkRow): Boolean =
    !row.connected && row.editing in setOf("hostAndPort", "portOnly", "serial")

internal val CREATABLE_LINK_TYPES = listOf("udp", "tcp", "serial")
internal const val BLUETOOTH_LINK = "bluetooth"

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
    return CREATABLE_LINK_TYPES.filter { it in served }.ifEmpty { CREATABLE_LINK_TYPES } + bluetooth
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
): List<Pair<String, Any>> = listOf<Pair<String, Any>>("name" to name, "autoConnect" to autoConnect, "highLatency" to highLatency) + when (editing) {
    "hostAndPort" -> listOf("host" to host, "port" to port)
    "portOnly" -> listOf("localPort" to port)
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

internal fun autoLinkName(type: String, host: String, port: String): String =
    if (host.isBlank()) "${type.uppercase()} $port" else "${type.uppercase()} $host:$port"

internal const val DEFAULT_BAUD = 57600

internal data class SerialPortChoice(val port: String, val label: String)

// view.links pairs each port with its label before dropping blank ports; pairing after the filter
// shifted every label after a blank entry onto the wrong port.
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
                    painterResource(if (row.portName.isNotBlank()) R.drawable.ic_usb else R.drawable.ic_link),
                    null,
                    tint = if (row.connected) MaterialTheme.aircast.success else MaterialTheme.colorScheme.onSecondaryContainer,
                    modifier = Modifier.size(24.dp),
                )
            }
            Column(Modifier.weight(1f)) {
                Text(
                    text = row.name,
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = row.statusLine,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (row.goneQuiet) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                )
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
                Text("Connected", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.aircast.success)
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
                text = { Text("Edit Link") },
                enabled = linkIsEditable(row),
                onClick = {
                    menuOpen = false
                    onEdit()
                },
            )
            DropdownMenuItem(
                text = { Text("Delete Link") },
                onClick = {
                    menuOpen = false
                    onRemove()
                },
            )
        }
    }
}

@Composable
private fun LinkFlagSwitches(autoConnect: Boolean, highLatency: Boolean, onAutoConnect: (Boolean) -> Unit, onHighLatency: (Boolean) -> Unit) {
    listOf(Triple("Automatically Connect on Start", autoConnect, onAutoConnect), Triple("High Latency", highLatency, onHighLatency)).forEach { (label, checked, onChange) ->
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f))
            Switch(checked = checked, onCheckedChange = onChange)
        }
    }
}

internal fun linkFlagWrites(index: Int, autoConnect: Boolean, highLatency: Boolean): List<Pair<String, Boolean>> =
    listOf("$LINKS_PATH.$index.autoConnect" to autoConnect, "$LINKS_PATH.$index.highLatency" to highLatency)

private fun writeNewLinkFlags(name: String, autoConnect: Boolean, highLatency: Boolean) {
    if (!autoConnect && !highLatency) return
    val row = currentRows().firstOrNull { it.name == name } ?: return
    linkFlagWrites(row.index, autoConnect, highLatency).forEach { (path, value) -> Qgc.set(path, value) }
    Qgc.invoke("links.commitLinkConfigurations")
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
        Text("Bluetooth Devices", style = MaterialTheme.typography.titleSmall)
        state.devices.forEach { device ->
            FilterChip(selected = device == chosen, onClick = { onPick(device) }, label = { Text(device.name) })
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(enabled = !state.scanning, onClick = { offMainDetached { Qgc.invoke("links.bluetoothScan", true) } }) { Text("Scan") }
            OutlinedButton(enabled = state.scanning, onClick = { offMainDetached { Qgc.invoke("links.bluetoothScan", false) } }) { Text("Stop") }
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
    var autoConnect by remember { mutableStateOf(row.autoConnect) }
    var highLatency by remember { mutableStateOf(row.highLatency) }
    var framing by remember { mutableStateOf(row.framing) }
    var advanced by remember { mutableStateOf(false) }
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
                LinkFlagSwitches(autoConnect, highLatency, { autoConnect = it }, { highLatency = it })
                if (row.editing == "portOnly") UdpServers(row.index, row.servers)
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
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = advanced, onCheckedChange = { advanced = it })
                        Text("Advanced Settings")
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
                                editWrites(row.editing, name, host, parsed, portName, baud, autoConnect, highLatency, framing)
                                    .forEach { (field, value) ->
                                        // qtpaths: links.linkConfigurations.0.name, links.linkConfigurations.0.autoConnect, links.linkConfigurations.0.highLatency, links.linkConfigurations.0.host, links.linkConfigurations.0.port, links.linkConfigurations.0.localPort, links.linkConfigurations.0.portName, links.linkConfigurations.0.baud, links.linkConfigurations.0.dataBits, links.linkConfigurations.0.stopBits, links.linkConfigurations.0.parity, links.linkConfigurations.0.flowControl
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
    val ports = remember(linksJson) { serialPortChoices(linksJson) }
    val bauds = remember(linksJson) { serialBauds(linksJson).ifEmpty { listOf(DEFAULT_BAUD) } }
    val taken = remember(linksJson) { linkRows(linksJson).map { it.name } }
    val offered = remember(linksJson) { addableLinkTypes(linksJson) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var autoConnect by remember { mutableStateOf(false) }
    var highLatency by remember { mutableStateOf(false) }
    var device by remember { mutableStateOf<BluetoothDeviceChoice?>(null) }
    var apiBase by remember { mutableStateOf("") }
    var deviceId by remember { mutableStateOf("") }
    var cloudErrors by remember { mutableStateOf(false) }
    var cloudOffered by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { cloudOffered = withContext(Dispatchers.Default) { accountState(Qgc.get("account")) != null } }
    val choices = offered + listOf(AIRCAST_CLOUD_LINK).filter { cloudOffered && it in linkTypeIds(linksJson) }
    val askBluetooth = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestMultiplePermissions()) { }
    val scope = rememberCoroutineScope()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Link") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                LinkFlagSwitches(autoConnect, highLatency, { autoConnect = it }, { highLatency = it })
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    choices.forEach { id ->
                        FilterChip(
                            selected = type == id,
                            onClick = {
                                type = id
                                if (id == BLUETOOTH_LINK) askBluetooth.launch(bluetoothPermissions(android.os.Build.VERSION.SDK_INT))
                            },
                            label = { Text(when (id) { "serial" -> "Serial"; BLUETOOTH_LINK -> "Bluetooth"; AIRCAST_CLOUD_LINK -> "Aircast Cloud"; else -> id.uppercase() }) },
                        )
                    }
                }
                Text(
                    text = when (type) {
                        "udp" -> "Listens on a port. Leave the address blank unless you need to " +
                            "reach a specific device."
                        "serial" -> "A radio plugged into this device over USB."
                        BLUETOOTH_LINK -> "A radio paired with or near this device over Bluetooth."
                        AIRCAST_CLOUD_LINK -> "A backup link to the aircraft through your Aircast account."
                        else -> "Calls out to a device that is listening, such as a ground station."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (type == AIRCAST_CLOUD_LINK) {
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
                    cloudErrors = type == AIRCAST_CLOUD_LINK
                    val invalid = if (type == AIRCAST_CLOUD_LINK) {
                        if (cloudApiBaseValid(apiBase) && cloudDeviceValid(deviceId)) null else ""
                    } else if (type == BLUETOOTH_LINK) {
                        if (device == null) "Pick a Bluetooth device." else null
                    } else if (type == "serial") {
                        serialFormError(portName, baud, taken, name, ports.isNotEmpty())
                    } else {
                        linkFormError(type, host, port)
                    }
                    error = invalid?.ifBlank { null }
                    if (invalid == null) {
                        busy = true
                        val chosen = name.ifBlank {
                            when (type) {
                                AIRCAST_CLOUD_LINK -> AIRCAST_CLOUD_NAME
                                BLUETOOTH_LINK -> device?.name.orEmpty()
                                "serial" -> autoSerialName(portName)
                                else -> autoLinkName(type, host, port)
                            }
                        }
                        scope.launch {
                            val added = withContext(Dispatchers.Default) {
                                if (type == AIRCAST_CLOUD_LINK) {
                                    Qgc.set("account.apiBase", apiBase)
                                    Qgc.invokeResult("links.createAircastCloudLink", chosen, apiBase, deviceId) == true
                                } else if (type == BLUETOOTH_LINK) {
                                    val picked = device!!
                                    Qgc.invokeResult("links.createBluetoothLink", chosen, picked.name, picked.address) == true
                                } else if (type == "serial") {
                                    Qgc.invokeResult("links.createSerialConfiguration", chosen, portName, baud) == true &&
                                        connectNamed(chosen)
                                } else {
                                    Qgc.invokeResult(
                                        "links.createAndConnectLink", type, chosen, host, port.toInt(),
                                    ) == true
                                }.also { created -> if (created) writeNewLinkFlags(chosen, autoConnect, highLatency) }
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
fun LinksScreen(modifier: Modifier = Modifier, footer: @Composable () -> Unit = {}) {
    val view by qgcPath(LINKS_VIEW)
    val hasVehicle = hasVehicle()
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

    LazyColumn(Modifier.weight(1f)) {
        if (rows.isEmpty()) {
            item(key = "empty") {
                FootNote(
                    "No links saved. Aircast finds a vehicle on the network by itself, so you " +
                        "only need to add one when that does not reach it.",
                )
            }
        } else {
            rows.groupBy { it.connected }.toList().sortedByDescending { it.first }.forEach { (connected, group) ->
            item(key = "head$connected") { SectionHeader(if (connected) "Connected" else "Saved") }
            items(group, key = { "link${it.name}" }) { row ->
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
            }
            }
        }

        item(key = "note") {
            FootNote(
                "Automatic connections are not listed here — they come and go on their own.",
            )
        }

        item(key = "footer") { footer() }
        item(key = "fabSpace") { Spacer(Modifier.height(88.dp)) }
    }

    }
    ExtendedFloatingActionButton(
        onClick = { adding = true },
        icon = { Icon(painterResource(R.drawable.ic_add), null) },
        text = { Text("Add Link…") },
        containerColor = MaterialTheme.colorScheme.primaryContainer,
        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
    )
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
            Text("Enable Flow Control")
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
    var typed by remember { mutableStateOf("") }

    fun change(action: String, host: String) {
        scope.launch {
            servers = withContext(Dispatchers.Default) {
                // qtpaths: links.linkConfigurations.0.addHost, links.linkConfigurations.0.removeHost
                Qgc.invoke("$LINKS_PATH.$index.$action", host)
                currentRows().firstOrNull { it.index == index }?.servers ?: servers
            }
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Server Addresses (optional)", style = MaterialTheme.typography.labelLarge)
        servers.forEach { server ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(server, modifier = Modifier.weight(1f))
                TextButton(onClick = { change("removeHost", server) }) { Text("Remove") }
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
                change("addHost", typed.trim())
                typed = ""
            }) { Text("Add Server") }
        }
    }
}
