package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import one.aircast.map.aircast

private fun checkState(level: Int): SetupState = if (level == DeviceLevel.Ready) SetupState.Done else SetupState.NeedsAttention

@Composable
internal fun DeviceTab(open: String?, onOpen: (String) -> Unit, onClose: () -> Unit) {
    val reading = deviceReading("aircraft")
    val status = remember(reading) { reading.view?.let(::parseDeviceStatus) }
    var stack by remember(open) { mutableStateOf(listOfNotNull(open)) }
    val current = stack.lastOrNull()
    val back = { if (stack.size > 1) stack = stack.dropLast(1) else onClose() }
    val opened = current?.let(::devicePath)
    val entry = opened?.let { DEVICE_SCREENS[it.path] }
    CompositionLocalProvider(LocalDeviceOrigin provides reading.origin) {
        when {
            current != null && opened != null && entry != null -> key(current) {
                NativeDevicePage(entry, DeviceScreenProps(current, opened.query, { stack = stack + it }, back), back)
            }
            reading.origin.isEmpty() -> ConnectDevice()
            else -> Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                status?.let { DeviceHome(it, onOpen) } ?: FootNote("Waiting for the device to answer\u2026")
                HostedDevice(reading.host)
            }
        }
    }
}

@Composable
private fun DeviceHome(status: DeviceStatus, onOpen: (String) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(status.headline, style = MaterialTheme.typography.titleMedium, color = deviceLevelColor(status.level))
        if (status.detail.isNotEmpty()) Text(status.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        status.action?.let { action -> FilledTonalButton(onClick = { onOpen(action.href) }, modifier = Modifier.padding(top = 8.dp)) { Text(action.label) } }
    }
    status.setup?.let { setup -> SetupRow(title = setup.label, icon = R.drawable.ic_build, state = SetupState.NeedsAttention, onClick = { onOpen(setup.href) }) }
    SectionHeader("Before you fly")
    status.checks.map { check ->
        key(check.label) {
            SetupRow(title = check.label, status = check.value, state = checkState(check.level), onClick = check.href.takeIf { it.isNotEmpty() }?.let { { onOpen(it) } })
        }
    }
    SetupRow(
        title = status.events.ifEmpty { "Flight log" },
        state = if (status.events.isNotEmpty()) SetupState.NeedsAttention else SetupState.Neutral,
        onClick = { onOpen("/flights") },
    )
    status.link?.let { UplinkSection(it) }
    status.health?.let { HealthSection(it) }
    SectionHeader(status.name.ifEmpty { "Device" })
    status.pages.map { page ->
        key(page.href) {
            SetupRow(
                title = page.title,
                status = page.summary,
                icon = devicePageIcon(page.icon),
                state = if (page.needs) SetupState.NeedsAttention else SetupState.Neutral,
                onClick = { onOpen(page.href) },
            )
        }
    }
}

@Composable
private fun ConnectDevice() {
    val scope = rememberCoroutineScope()
    var address by rememberSaveable { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf("") }
    val connect = {
        busy = true
        scope.launch {
            refusal = DeviceLink.connect(address).orEmpty()
            busy = false
        }
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Connect to an Aircast device", style = MaterialTheme.typography.titleMedium)
        Text("Type the address of a device on this network. Its status, cameras and settings open here.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedTextField(
            value = address,
            onValueChange = { address = it },
            label = { Text("Device address") },
            placeholder = { Text("192.168.1.100 or falcon.local") },
            singleLine = true,
            isError = refusal.isNotEmpty(),
            supportingText = refusal.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false, imeAction = ImeAction.Go),
            keyboardActions = KeyboardActions(onGo = { connect() }),
            modifier = Modifier.fillMaxWidth(),
        )
        FilledTonalButton(onClick = { connect() }, enabled = !busy) { Text("Connect") }
    }
}

@Composable
private fun HostedDevice(host: String) {
    DeviceSection("This app") {
        DeviceFact("Connected to", host)
        TextButton(onClick = { DeviceLink.disconnect() }, modifier = Modifier.padding(horizontal = 8.dp)) { Text("Disconnect") }
    }
}

@Composable
private fun UplinkSection(link: DeviceUplink) {
    DeviceSection("Link") {
        DeviceFact("Carrying video", link.carrier)
        DeviceText(listOf(link.backup, link.network).filter { it.isNotEmpty() }.joinToString(" · "))
        if (link.rate.isNotEmpty()) DeviceFact("Throughput", link.rate)
        if (link.standby.isNotEmpty()) DeviceFact("Cellular · standby", link.standby)
        if (link.trouble.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Warn, link.trouble))
        if (link.data.isNotEmpty()) DeviceFact("Data", link.data)
        link.charts.map { chart -> key(chart.label) { DeviceChart(chart) } }
    }
}

@Composable
private fun HealthSection(health: DeviceHealth) {
    DeviceSection("Device health") {
        if (health.memory.isNotEmpty()) DeviceFact("Memory in use", health.memory)
        health.warning?.let { DeviceNotice(it) }
        health.charts.map { chart -> key(chart.label) { DeviceChart(chart) } }
    }
}

@Composable
private fun NativeDevicePage(entry: DeviceScreenEntry, props: DeviceScreenProps, onBack: () -> Unit) {
    BackHandler(onBack = onBack)
    OverridePageHeading(entry.title, onBack)
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) { entry.screen(props) }
}

internal fun chipText(status: DeviceStatus): String = if (status.name.isNotEmpty()) "${status.name} · ${status.headline}" else status.headline

@Composable
internal fun DeviceStatusChip() {
    val navigation = LocalAppNavigation.current
    val shown = deviceView("aircraft", ::parseDeviceStatus)?.takeIf(::deviceNeedsAttention) ?: return
    val backdrop = osdBackdrop(Color.Black.copy(alpha = 0.45f))
    val open = {
        navigation.settingsPage = DEVICE_PAGE
        navigation.settingsOpen = true
    }
    val label = "${shown.name}: ${shown.headline}"
    if (flyIsPortrait()) {
        Surface(onClick = open, shape = CircleShape, color = backdrop, contentColor = MaterialTheme.aircast.outdoorForeground, modifier = Modifier.size(40.dp).semantics { contentDescription = label }) {
            Box(contentAlignment = Alignment.Center) {
                Icon(painterResource(R.drawable.ic_developer_board), null, Modifier.size(22.dp))
                Box(Modifier.align(Alignment.TopEnd).padding(6.dp).size(10.dp).border(2.dp, Color.Black.copy(alpha = 0.4f), CircleShape).background(deviceLevelColor(shown.level), CircleShape))
            }
        }
    } else {
        Surface(onClick = open, shape = CircleShape, color = backdrop, contentColor = MaterialTheme.aircast.outdoorForeground, modifier = Modifier.widthIn(max = 320.dp).semantics { contentDescription = label }) {
            Row(Modifier.padding(horizontal = 12.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(Modifier.size(8.dp).background(deviceLevelColor(shown.level), CircleShape))
                Text(chipText(shown), style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}
