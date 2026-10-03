package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.width
import kotlinx.coroutines.isActive
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import kotlinx.coroutines.CoroutineScope
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.runtime.key
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.settingControl
import one.aircast.android.bridge.qgcValue
import one.aircast.android.bridge.truthy
import one.aircast.mapspike.VehicleChoice
import org.json.JSONObject
import one.aircast.mapspike.CHOOSER_TITLE
import one.aircast.mapspike.VEHICLES_VIEW
import one.aircast.mapspike.FleetBridge
import one.aircast.mapspike.VehicleBridge
import one.aircast.mapspike.VehicleChoices
import one.aircast.mapspike.activeVehicleTitle
import one.aircast.mapspike.optText
import one.aircast.mapspike.lostVehicles
import one.aircast.mapspike.lostVehiclesText
import one.aircast.mapspike.linkDistinguishes
import one.aircast.mapspike.vehicleChoiceLine
import one.aircast.mapspike.vehicleFlightModePath
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.DropdownMenu
import androidx.compose.foundation.layout.Box
import one.aircast.mapspike.vehicleTelemetryLine
import one.aircast.mapspike.vehicleChoices

internal data class MvAction(
    val id: String,
    val title: String,
    val confirmTitle: String,
    val prompt: String,
    val offer: String,
    val reason: String,
) {
    val ready: Boolean get() = offer == "ready"
}

internal fun mvActions(view: JSONObject?): List<MvAction> {
    val listed = view?.optJSONArray("actions") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { entry ->
            MvAction(
                id = entry.optText("id").ifBlank { return@mapNotNull null },
                title = entry.optText("title"),
                confirmTitle = entry.optText("confirmTitle").ifBlank { entry.optText("title") },
                prompt = entry.optText("prompt"),
                offer = entry.optText("offer"),
                reason = entry.optText("reason"),
            )
        }
    }
}

internal fun mvReasonFor(action: MvAction): String? =
    action.reason.ifBlank { null }?.takeIf { !action.ready }

internal const val STATUS_SETTINGS_PAGE = "Status Settings"
internal const val CLOSE_VEHICLE = "vehicle.closeVehicle"
private const val LOAD_POLL_MS = 500L

internal fun loadingProgress(fields: org.json.JSONObject?): Float? =
    fields?.takeIf { it.has("initialConnectComplete") && !it.optBoolean("initialConnectComplete") }
        ?.let { it.optDouble("loadProgress", 0.0).toFloat().coerceIn(0f, 1f) }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VehicleStateChip(modifier: Modifier = Modifier) {
    val flyJson by qgcPath(FLY_STATE)
    val fly = remember(flyJson) { flyState(flyJson) }
    val vehiclesJson by qgcPath(VEHICLES_VIEW)
    val choices = remember(vehiclesJson) { vehicleChoices(vehiclesJson) }
    val controlJson by qgcPath(OPERATOR_CONTROL_VIEW)
    val station = remember(controlJson) { controlStation(controlJson) }
    val taken = controlIsElsewhere(station)
    val lost = fly?.contactLost == true
    val offlineJson by qgcPath(OFFLINE_STATUS_VIEW)
    val subtitle = vehicleSubtitle(fly, remember(offlineJson) { offlineMainStatus(offlineJson) })
    var picking by remember { mutableStateOf(false) }
    val panelJson by qgcPath(MULTI_VEHICLE_PANEL)
    val panelEnabled = multiVehiclePanelEnabled(panelJson)
    val panelFact by qgcPath(MULTI_VEHICLE_PANEL_SETTING)
    var offline by remember { mutableStateOf(false) }
    var statusSettings by remember { mutableStateOf(false) }
    var modeMenu by remember { mutableStateOf(false) }
    val disconnected = fly?.connected != true
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }

    androidx.compose.foundation.layout.Box(modifier) {
    FlightModeMenu(expanded = modeMenu && !disconnected, onDismiss = { modeMenu = false }, onStatus = { statusSettings = true })
    androidx.compose.material3.Surface(
        shape = MaterialTheme.shapes.small,
        color = when {
            lost || fly?.fault == true -> MaterialTheme.colorScheme.errorContainer
            disconnected -> MaterialTheme.colorScheme.surfaceContainerHigh
            fly?.nominal == false -> MaterialTheme.aircast.warningContainer
            else -> MaterialTheme.aircast.successContainer
        },
        contentColor = when {
            lost || fly?.fault == true -> MaterialTheme.colorScheme.onErrorContainer
            disconnected -> MaterialTheme.colorScheme.onSurface
            fly?.nominal == false -> MaterialTheme.aircast.warning
            else -> MaterialTheme.aircast.success
        },
        onClick = {
            when {
                choices.ambiguous || taken -> picking = true
                disconnected -> offline = true
                else -> modeMenu = true
            }
        },
    ) {
    Row(
        Modifier.heightIn(min = 32.dp).padding(horizontal = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(painterResource(R.drawable.ic_flight), null, Modifier.size(24.dp))
        Text(
            text = activeVehicleTitle(choices, subtitle),
            style = MaterialTheme.typography.labelLarge,
            maxLines = 1,
        )
        if (FlightModePending.mode != null) {
            androidx.compose.material3.CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
        } else if (choices.ambiguous || taken) {
            val alarm = lostVehiclesText(lostVehicles(choices)) ?: controlLine(station).takeIf { taken }
            Icon(
                painter = painterResource(if (alarm == null) R.drawable.ic_arrow_drop_down else R.drawable.ic_warning),
                contentDescription = alarm ?: "Choose which vehicle to fly",
            )
        } else if (!disconnected) {
            Icon(painterResource(R.drawable.ic_arrow_drop_down), contentDescription = "Change flight mode")
        }
        var loading by remember { mutableStateOf<Float?>(null) }
        androidx.compose.runtime.LaunchedEffect(choices.choices.size) {
            while (isActive) {
                loading = withContext(Dispatchers.Default) { loadingProgress(Qgc.get("vehicle", listOf("initialConnectComplete", "loadProgress"))) }
                kotlinx.coroutines.delay(LOAD_POLL_MS)
            }
        }
        loading?.let { progress ->
            Column(Modifier.padding(start = 8.dp)) {
                Text("Loading vehicle", style = MaterialTheme.typography.labelSmall)
                androidx.compose.material3.LinearProgressIndicator(progress = { progress }, modifier = Modifier.width(72.dp))
            }
        }
        if (lost) {
            androidx.compose.material3.TextButton(onClick = {
                scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(CLOSE_VEHICLE) } }
            }) { Text("Disconnect") }
        }
    }
    }
    }

    androidx.compose.runtime.LaunchedEffect(disconnected) { if (!disconnected) offline = false }
    if (offline) {
        OfflineStatusSheet { offline = false }
    }
    if (statusSettings && !disconnected) {
        VehicleStatusSheet { statusSettings = false }
    }

    if (picking) {
        ModalBottomSheet(
            onDismissRequest = { picking = false },
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        ) {
            Text(
                text = lostVehiclesText(lostVehicles(choices)) ?: CHOOSER_TITLE,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            VehicleRows(choices, selectable = panelEnabled, scope = scope, onRefusal = { refusal = it }) { picking = false }
            ListItem(
                headlineContent = { Text("Connect another vehicle", color = MaterialTheme.colorScheme.primary) },
                leadingContent = { Icon(painterResource(R.drawable.ic_add), null, tint = MaterialTheme.colorScheme.primary) },
                modifier = Modifier.clickable { picking = false; AppNavigation.settingsPage = CONNECTIONS_PAGE },
            )
            refusal?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp, vertical = 12.dp),
                )
            }
            if (panelToggleShown(panelFact)) ListItem(
                headlineContent = { Text("Enable multi-vehicle panel") },
                trailingContent = {
                    Switch(
                        checked = panelEnabled,
                        onCheckedChange = { wanted -> scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(MULTI_VEHICLE_PANEL_SETTING, wanted) } } },
                    )
                },
            )
            if (!disconnected && !taken) {
                Row(Modifier.padding(horizontal = 16.dp)) {
                    androidx.compose.material3.TextButton(onClick = { picking = false; modeMenu = true }) { Text("Flight mode") }
                    androidx.compose.material3.TextButton(onClick = { picking = false; statusSettings = true }) { Text("Vehicle status") }
                }
            }
            ControlHolderNote(station, activeVehicleId(vehiclesJson)) { message -> refusal = message }
            FootNote(
                "Tap a name to fly that aircraft. Arm, Takeoff and every action on the flight screen go to the one you pick.",
            )
            if (panelEnabled) FleetControls(vehiclesJson, choices) { message -> refusal = message }
        }
    }
}

@Composable
private fun FleetControls(view: JSONObject?, choices: VehicleChoices, onRefusal: (String?) -> Unit) {
    if (!choices.ambiguous) return
    val actions = remember(view) { mvActions(view) }
    val scope = rememberCoroutineScope()
    var confirming by remember { mutableStateOf<MvAction?>(null) }
    Text(
        text = fleetHeading(selectedIds(choices)),
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
    Row(
        modifier = Modifier.padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(
            enabled = choices.canSelectAll,
            onClick = { scope.launch { withContext(Dispatchers.Default) { FleetBridge.selectAll(choices) } } },
        ) { Text("Select all") }
        TextButton(
            enabled = choices.canDeselectAll,
            onClick = { scope.launch { withContext(Dispatchers.Default) { FleetBridge.deselectAll() } } },
        ) { Text("Deselect all") }
    }
    Text(
        text = "Multi vehicle actions",
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
    actions.forEach { action ->
        ListItem(
            headlineContent = {
                Text(
                    text = action.title,
                    color = when {
                        action.ready -> MaterialTheme.colorScheme.onSurface
                        else -> MaterialTheme.colorScheme.onSurfaceVariant
                    },
                )
            },
            supportingContent = { if (confirming?.id != action.id) Text(fleetActionLine(action)) },
            modifier = Modifier.clickable(enabled = action.ready) { confirming = action },
        )
        if (confirming?.id == action.id) {
            ConfirmTrack(
                action = GuidedAction(
                    name = action.confirmTitle,
                    confirm = action.prompt,
                    destructive = fleetIsDestructive(action),
                    run = {
                        val confirmed = choices.choices.filter { it.selected }.map { it.id }.toSet()
                        scope.launch {
                            val sent = withContext(Dispatchers.Default) {
                                FleetBridge.command(action.id, confirmed)
                            }
                            onRefusal(if (sent) null else "${action.title} did not reach every selected aircraft.")
                        }
                    },
                ),
                onSent = { confirming = null },
                onCancel = { confirming = null },
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
        }
    }
}

@Composable
private fun VehicleModeMenu(choice: VehicleChoice, scope: CoroutineScope, onRefusal: (String?) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        TextButton(onClick = { open = true }) { Text("Mode") }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            choice.flightModes.forEach { mode ->
                DropdownMenuItem(text = { Text(mode) }, onClick = {
                    open = false
                    scope.launch { onRefusal(withContext(Dispatchers.Default) { Qgc.writeForVehicleRefusal(vehicleFlightModePath(choice), mode, choice.id) }) }
                })
            }
        }
    }
}

internal fun fleetPanelShown(vehicleCount: Int, panelEnabled: Boolean): Boolean = vehicleCount >= 2 && panelEnabled

@Composable
internal fun FleetPanel(modifier: Modifier = Modifier) {
    val vehiclesJson by qgcPath(one.aircast.mapspike.VEHICLES_VIEW)
    val choices = remember(vehiclesJson) { vehicleChoices(vehiclesJson) }
    val panelJson by qgcPath(MULTI_VEHICLE_PANEL)
    var refusal by remember { mutableStateOf<String?>(null) }
    if (!fleetPanelShown(choices.choices.size, multiVehiclePanelEnabled(panelJson))) return
    Column(modifier) {
        VehicleRows(choices, selectable = true, scope = rememberCoroutineScope(), onRefusal = { refusal = it }) {}
        FleetControls(vehiclesJson, choices) { refusal = it }
        refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp)) }
    }
}

@Composable
private fun VehicleRows(choices: VehicleChoices, selectable: Boolean, scope: CoroutineScope, onRefusal: (String?) -> Unit, onSwitched: () -> Unit) {
    val distinguishes = linkDistinguishes(choices.choices)
    choices.choices.forEach { choice -> key(choice.id) {
        ListItem(
            headlineContent = { Text(choice.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
            supportingContent = {
                Column {
                    Text(vehicleChoiceLine(choice, distinguishes), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    vehicleTelemetryLine(choice)?.let { Text(it, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis) }
                }
            },
            leadingContent = if (selectable) ({
                Checkbox(
                    checked = choice.selected,
                    onCheckedChange = { wanted -> scope.launch { withContext(Dispatchers.Default) { FleetBridge.setSelected(choice.id, wanted) } } },
                )
            }) else null,
            trailingContent = {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (choice.flightModes.isNotEmpty() && choice.index >= 0) VehicleModeMenu(choice, scope, onRefusal)
                    VehicleRowCompass(choice.heading, choice.armed)
                    if (choice.active) Icon(Icons.Default.Check, contentDescription = "Flying this one")
                }
            },
            modifier = Modifier.clickable(enabled = !choice.active) {
                scope.launch {
                    val switched = withContext(Dispatchers.Default) { VehicleBridge.askFor(choice.id) }
                    onRefusal(if (switched) null else VehicleBridge.lastRefusal ?: "That vehicle did not take control.")
                    if (switched) onSwitched()
                }
            },
        )
        HorizontalDivider()
    } }
}

internal fun selectedIds(choices: VehicleChoices): List<Int> = choices.choices.filter { it.selected }.map { it.id }.sorted()

internal fun fleetHeading(selectedIds: List<Int>): String = "Vehicles Selected: ${selectedIds.joinToString(", ").ifEmpty { "-" }}"

internal fun fleetActionLine(action: MvAction): String =
    mvReasonFor(action) ?: action.prompt.ifBlank { action.title }

internal fun fleetIsDestructive(action: MvAction): Boolean = action.id != "mvPause"

internal object ControlRequestDeadlines {
    val endsAt = androidx.compose.runtime.mutableStateMapOf<Int, Long>()
}

@Composable
private fun ControlHolderNote(station: ControlStation?, vehicleId: Int?, onRefusal: (String?) -> Unit) {
    val holder = station ?: return
    if (holder.inControl == true) {
        InControlNote(holder, onRefusal)
        return
    }
    val line = holderLine(holder) ?: return
    val scope = rememberCoroutineScope()
    val deadlineKey = vehicleId ?: 0
    val requestEndsAt = ControlRequestDeadlines.endsAt[deadlineKey]
    LaunchedEffect(holder.takeoverAllowed) {
        if (holder.takeoverAllowed == true) ControlRequestDeadlines.endsAt.remove(deadlineKey)
    }
    Text(
        text = line,
        style = MaterialTheme.typography.bodyMedium,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
    takeoverLine(holder)?.let { takeover ->
        Text(
            text = takeover,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 24.dp),
        )
    }
    ControlSectionTitle(holder)
    requestEndsAt?.let { endsAt ->
        RequestCountdown(endsAt) { ControlRequestDeadlines.endsAt.remove(deadlineKey) }
    }
    AllowTakeoverBox(holder, onRefusal)
    controlWaitLine(holder)?.takeIf { requestEndsAt == null }?.let { waiting ->
        Text(
            text = waiting,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
    }
    acquireLabel(holder)?.let { label ->
        TextButton(
            enabled = acquireEnabled(holder, requestEndsAt != null),
            onClick = {
                scope.launch {
                    val ask = withContext(Dispatchers.Default) { askForControl(holder) }
                    onRefusal(ask.refusal)
                    if (ask.refusal == null && ask.timeoutSeconds > 0) {
                        ControlRequestDeadlines.endsAt[deadlineKey] = System.currentTimeMillis() + ask.timeoutSeconds * 1000L
                    }
                }
            },
            modifier = Modifier.padding(horizontal = 16.dp),
        ) { Text(label) }
    }
    if (requestTimeoutEditable(holder)) SettingFactRow(REQUEST_TIMEOUT_PATH, "Request timeout (sec)")
    SettingFactRow(GCS_SYSTEM_ID_PATH, "This GCS MAVLink system ID")
}

@Composable
private fun ControlSectionTitle(holder: ControlStation) {
    controlSectionTitle(holder)?.let { title ->
        Text(
            text = title,
            style = MaterialTheme.typography.titleSmall,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
    }
}

@Composable
private fun SettingFactRow(path: String, title: String) {
    val control by qgcPath(settingControl(path))
    val fact = remember(control) { control?.takeIf { it.optText("kind") == "object" }?.let(::factFromControl) } ?: return
    FactRow(fact, title = title, subtitle = factSubtitle(fact), fieldModifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 8.dp))
}

@Composable
private fun RequestCountdown(endsAt: Long, onDone: () -> Unit) {
    var now by remember(endsAt) { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(endsAt) {
        while (now < endsAt) {
            delay(COUNTDOWN_TICK_MS)
            now = System.currentTimeMillis()
        }
        onDone()
    }
    Text(
        text = requestSentLabel(endsAt - now),
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
}

private const val COUNTDOWN_TICK_MS = 100L

@Composable
private fun AllowTakeoverBox(holder: ControlStation, onRefusal: (String?) -> Unit, onRead: (Boolean?) -> Unit = {}) {
    val scope = rememberCoroutineScope()
    val served by qgcValue(ALLOW_TAKEOVER_SETTING)
    var typed by remember { mutableStateOf<Boolean?>(null) }
    val stored = served?.takeIf { it != JSONObject.NULL }?.let(::truthy)
    val allow = typed ?: stored
    LaunchedEffect(stored) {
        typed = null
        onRead(stored)
    }
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 4.dp),
    ) {
        Text("Allow takeover", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        Checkbox(
            checked = allow == true,
            enabled = allow != null && allowTakeoverEditable(holder),
            onCheckedChange = { wanted ->
                typed = wanted
                onRead(wanted)
                scope.launch {
                    val refusal = withContext(Dispatchers.Default) { saveAllowTakeover(wanted) }
                    onRefusal(refusal)
                    if (refusal != null) typed = null
                }
            },
        )
    }
}

@Composable
private fun InControlNote(holder: ControlStation, onRefusal: (String?) -> Unit) {
    val scope = rememberCoroutineScope()
    var allow by remember { mutableStateOf<Boolean?>(null) }
    inControlLine(holder)?.let { line ->
        Text(
            text = line,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
    }
    takeoverLine(holder)?.let { takeover ->
        Text(
            text = takeover,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 24.dp),
        )
    }
    ControlSectionTitle(holder)
    AllowTakeoverBox(holder, onRefusal) { allow = it }
    TextButton(
        enabled = takeoverChangeable(holder, allow),
        onClick = {
            val wanted = allow ?: return@TextButton
            scope.launch { onRefusal(withContext(Dispatchers.Default) { changeTakeover(wanted) }) }
        },
        modifier = Modifier.padding(horizontal = 16.dp),
    ) { Text("Change") }
    SettingFactRow(GCS_SYSTEM_ID_PATH, "This GCS MAVLink system ID")
}

internal const val MULTI_VEHICLE_PANEL_SETTING = "settings.appSettings.enableMultiVehiclePanel"
internal const val MULTI_VEHICLE_PANEL = "$MULTI_VEHICLE_PANEL_SETTING.rawValue"

internal fun panelToggleShown(fact: JSONObject?): Boolean = fact?.optBoolean("visible", true) != false

internal fun multiVehiclePanelEnabled(setting: JSONObject?): Boolean =
    setting?.takeIf { it.has("value") && !it.isNull("value") }?.optBoolean("value", true) ?: true

internal fun activeVehicleId(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("activeId") }?.optInt("activeId", -1)?.takeIf { it > 0 }

private val ROW_COMPASS_SIZE = 28.dp
private val ROW_HEADING_COLOUR = androidx.compose.ui.graphics.Color(0xFFEE3424)
internal const val DISARMED_ALPHA = 0.5f

internal fun rowCompassAlpha(armed: Boolean): Float = if (armed) 1f else DISARMED_ALPHA

private val ROW_HEADING_SHADE = androidx.compose.ui.graphics.Color(0xFFC72B27)

@Composable
private fun VehicleRowCompass(heading: Double, armed: Boolean) {
    val fill = MaterialTheme.colorScheme.surface
    val border = MaterialTheme.colorScheme.onSurface
    androidx.compose.foundation.Canvas(Modifier.size(ROW_COMPASS_SIZE).alpha(rowCompassAlpha(armed))) {
        val radius = size.minDimension / 2f
        drawCircle(fill, radius = radius)
        drawCircle(border, radius = radius - 0.5.dp.toPx(), style = androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))
        if (heading.isFinite()) {
            rotate(heading.toFloat()) {
                val half = radius / 3f
                val top = center.y - half
                val bottom = center.y + half
                val notch = center.y + half * 0.5f
                val right = androidx.compose.ui.graphics.Path().apply { moveTo(center.x, top); lineTo(center.x + half, bottom); lineTo(center.x, notch); close() }
                val left = androidx.compose.ui.graphics.Path().apply { moveTo(center.x, top); lineTo(center.x - half, bottom); lineTo(center.x, notch); close() }
                drawPath(right, ROW_HEADING_COLOUR)
                drawPath(left, ROW_HEADING_SHADE)
                drawPath(right, ROW_HEADING_COLOUR, style = androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))
                drawPath(left, ROW_HEADING_COLOUR, style = androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))
            }
        }
    }
}
