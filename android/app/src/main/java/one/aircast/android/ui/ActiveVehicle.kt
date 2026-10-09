package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import one.aircast.map.aircast
import androidx.compose.foundation.layout.width
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
import one.aircast.map.AircastSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import one.aircast.map.VehicleChoice
import org.json.JSONObject
import one.aircast.map.CHOOSER_TITLE
import one.aircast.map.VEHICLES_VIEW
import one.aircast.map.FleetBridge
import one.aircast.map.VehicleBridge
import one.aircast.map.VehicleChoices
import one.aircast.map.activeVehicleTitle
import one.aircast.map.optText
import one.aircast.map.lostVehicles
import one.aircast.map.lostVehiclesText
import one.aircast.map.linkDistinguishes
import one.aircast.map.vehicleChoiceLine
import one.aircast.map.vehicleFlightModePath
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.DropdownMenu
import androidx.compose.foundation.layout.Box
import one.aircast.map.vehicleTelemetryLine
import one.aircast.map.vehicleChoices
import one.aircast.map.silentSeconds
import androidx.compose.ui.graphics.Color

private const val STATUS_BAR_SCRIM_ALPHA = 0.55f

internal fun osdModeText(title: String): String = title.substringBefore(" \u00b7 ")

internal fun osdStatusNote(title: String): String? = title.substringAfter(" \u00b7 ", "").ifBlank { null }

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

internal fun readinessSubtitle(state: FlyState?, blocker: String?, failing: Int): String? =
    blocker?.let { listOfNotNull(state?.mode?.ifBlank { null }, chipBlocker(it) + if (failing > 1) " +${failing - 1}" else "").joinToString(" \u00b7 ") }

internal const val VEHICLE_CONNECT_COMPLETE = "vehicle.initialConnectComplete"
internal const val VEHICLE_LOAD_PROGRESS = "vehicle.loadProgress"

internal fun loadingProgress(complete: Any?, progress: Any?): Float? =
    (complete as? Boolean)?.takeIf { !it }?.let { ((progress as? Number)?.toFloat() ?: 0f).coerceIn(0f, 1f) }

@Composable
internal fun rememberVehicleLoading(): Float? {
    val complete by qgcValue(VEHICLE_CONNECT_COMPLETE)
    val progress by qgcValue(VEHICLE_LOAD_PROGRESS)
    return loadingProgress(complete, progress)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VehicleStateChip(modifier: Modifier = Modifier) {
    val navigation = LocalAppNavigation.current
    val flyScreen = LocalFlyScreenState.current
    val flyJson by qgcPath(FLY_STATE)
    val fly = remember(flyJson) { flyState(flyJson) }
    val vehiclesJson by qgcPath(VEHICLES_VIEW)
    val choices = remember(vehiclesJson) { vehicleChoices(vehiclesJson) }
    val controlJson by qgcPath(OPERATOR_CONTROL_VIEW)
    val station = remember(controlJson) { controlStation(controlJson) }
    val taken = controlIsElsewhere(station)
    val lost = fly?.contactLost == true
    val silentFor = silentSeconds(lost)
    val linksJson by qgcPath(VEHICLE_LINKS)
    val failsafe = remember(linksJson) { vehicleLinks(linksJson)?.failsafe }
    var lostMenu by remember { mutableStateOf(false) }
    val offlineJson by qgcPath(OFFLINE_STATUS_VIEW)
    val warningsJson by qgcPath(WARNINGS)
    val blocker = remember(warningsJson, fly) { armingBlocker(warningsJson)?.takeIf { fly?.connected == true && !fly.armed } }
    val failing = remember(warningsJson) { armingChecks(warningsJson).orEmpty().size }
    val subtitle = readinessSubtitle(fly, blocker, failing) ?: vehicleSubtitle(fly, remember(offlineJson) { offlineMainStatus(offlineJson) })
    var why by remember { mutableStateOf(false) }
    var picking by remember { mutableStateOf(false) }
    val panelJson by qgcPath(MULTI_VEHICLE_PANEL)
    val panelEnabled = multiVehiclePanelEnabled(panelJson)
    val panelFact by qgcPath(MULTI_VEHICLE_PANEL_SETTING)
    var offline by remember { mutableStateOf(false) }
    var statusSettings by remember { mutableStateOf(false) }
    var modeMenu by remember { mutableStateOf(false) }
    OpenOnRequest("status") { statusSettings = true }
    OpenOnRequest("modes") { modeMenu = true }
    val disconnected = fly?.connected != true
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }

    androidx.compose.foundation.layout.Box(modifier) {
    FlightModeMenu(expanded = modeMenu && !disconnected, onDismiss = { modeMenu = false }, onStatus = { statusSettings = true }, onMessages = { why = true })
    val tone = if (blocker != null) ChipTone.Error else chipTone(fly, lost)
    val statusBar = !flyIsPortrait()
    androidx.compose.material3.Surface(
        shape = MaterialTheme.shapes.small,
        color = if (statusBar) {
            Color.Transparent
        } else {
            osdBackdrop(
                when (tone) {
                    ChipTone.Error -> MaterialTheme.colorScheme.errorContainer
                    ChipTone.Neutral -> MaterialTheme.colorScheme.surfaceContainerHigh
                    ChipTone.Warning -> MaterialTheme.aircast.warningContainer
                    ChipTone.Success -> MaterialTheme.aircast.successContainer
                },
            )
        },
        contentColor = if (statusBar) {
            MaterialTheme.aircast.outdoorForeground
        } else {
            when (tone) {
                ChipTone.Error -> osdTint(MaterialTheme.colorScheme.onErrorContainer, MaterialTheme.colorScheme.error)
                ChipTone.Neutral -> MaterialTheme.colorScheme.onSurface
                ChipTone.Warning -> MaterialTheme.aircast.warning
                ChipTone.Success -> MaterialTheme.aircast.success
            }
        },
        onClick = {
            when {
                lost -> lostMenu = true
                choices.ambiguous || taken -> picking = true
                disconnected -> offline = true
                blocker != null -> why = true
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
        val title = if (lost) signalLostTitle(silentFor, failsafe, compact = !statusBar) else activeVehicleTitle(choices, subtitle)
        Text(
            text = if (statusBar) osdModeText(title) else title,
            style = if (statusBar) MaterialTheme.typography.titleMedium else MaterialTheme.typography.labelLarge,
            maxLines = 1,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = false),
        )
        if (statusBar) osdStatusNote(title)?.let { note ->
            androidx.compose.material3.Surface(
                shape = MaterialTheme.shapes.small,
                color = Color.Black.copy(alpha = STATUS_BAR_SCRIM_ALPHA),
                contentColor = when (tone) {
                    ChipTone.Error -> MaterialTheme.colorScheme.error
                    ChipTone.Neutral -> MaterialTheme.aircast.outdoorForeground
                    ChipTone.Warning -> MaterialTheme.aircast.warning
                    ChipTone.Success -> MaterialTheme.aircast.success
                },
            ) {
                Text(note, style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis, modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp))
            }
        }
        if (flyScreen.pendingMode != null) {
            androidx.compose.material3.CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
        } else if (choices.ambiguous || taken) {
            val alarm = lostVehiclesText(lostVehicles(choices)) ?: controlLine(station).takeIf { taken }
            Icon(
                painter = painterResource(if (alarm == null) R.drawable.ic_arrow_drop_down else R.drawable.ic_warning),
                contentDescription = alarm ?: "Choose which vehicle to fly",
            )
        } else if (!disconnected) {
            Icon(
                painterResource(R.drawable.ic_arrow_drop_down),
                contentDescription = if (lost) SIGNAL_LOST else "Change flight mode",
                modifier = Modifier.clickable(role = androidx.compose.ui.semantics.Role.Button) { if (lost) lostMenu = true else modeMenu = true },
            )
        }
        rememberVehicleLoading()?.takeUnless { lost }?.let { progress ->
            Column(Modifier.padding(start = 8.dp)) {
                Text("Loading vehicle", style = MaterialTheme.typography.labelSmall)
                androidx.compose.material3.LinearProgressIndicator(progress = { progress }, modifier = Modifier.width(72.dp))
            }
        }
    }
    }
    androidx.compose.material3.DropdownMenu(expanded = lostMenu && lost, onDismissRequest = { lostMenu = false }) {
        androidx.compose.material3.DropdownMenuItem(
            text = { Text(silentFor?.let(::silenceText) ?: SIGNAL_LOST, color = MaterialTheme.colorScheme.onSurfaceVariant) },
            onClick = {},
            enabled = false,
        )
        androidx.compose.material3.DropdownMenuItem(
            text = { Text(LOST_LINK_HINT, style = MaterialTheme.typography.bodySmall, modifier = Modifier.widthIn(max = 280.dp)) },
            onClick = {},
            enabled = false,
        )
        androidx.compose.material3.HorizontalDivider()
        androidx.compose.material3.DropdownMenuItem(
            text = { Text("Disconnect", color = MaterialTheme.colorScheme.error) },
            onClick = {
                lostMenu = false
                scope.launch { flyScreen.refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(CLOSE_VEHICLE) } }
            },
        )
    }
    }

    androidx.compose.runtime.LaunchedEffect(disconnected) { if (!disconnected) offline = false }
    if (offline) {
        OfflineStatusSheet { offline = false }
    }
    if (statusSettings && !disconnected) {
        VehicleStatusSheet { statusSettings = false }
    }
    if (why) {
        VehicleMessagesSheet { why = false }
    }

    if (picking) {
        AircastSheet(
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
                modifier = Modifier.clickable { picking = false; navigation.settingsPage = CONNECTIONS_PAGE },
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
    val vehiclesJson by qgcPath(one.aircast.map.VEHICLES_VIEW)
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
        Row(
            Modifier
                .fillMaxWidth()
                .clickable(enabled = !choice.active) {
                    scope.launch {
                        val switched = withContext(Dispatchers.Default) { VehicleBridge.askFor(choice.id) }
                        onRefusal(if (switched) null else VehicleBridge.lastRefusal ?: "That vehicle did not take control.")
                        if (switched) onSwitched()
                    }
                }
                .heightIn(min = 72.dp)
                .padding(start = 16.dp, end = 24.dp, top = 8.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (selectable) {
                Checkbox(
                    checked = choice.selected,
                    onCheckedChange = { wanted -> scope.launch { withContext(Dispatchers.Default) { FleetBridge.setSelected(choice.id, wanted) } } },
                )
            }
            Column(Modifier.weight(1f)) {
                Text(choice.name, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(vehicleChoiceLine(choice, distinguishes), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                vehicleTelemetryLine(choice)?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis) }
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (choice.flightModes.isNotEmpty() && choice.index >= 0) VehicleModeMenu(choice, scope, onRefusal)
                VehicleRowCompass(choice.heading, choice.armed)
                if (choice.active) Icon(Icons.Default.Check, contentDescription = "Flying this one")
            }
        }
        HorizontalDivider()
    } }
}

internal fun selectedIds(choices: VehicleChoices): List<Int> = choices.choices.filter { it.selected }.map { it.id }.sorted()

internal fun fleetHeading(selectedIds: List<Int>): String = "Vehicles Selected: ${selectedIds.joinToString(", ").ifEmpty { "-" }}"

internal fun fleetActionLine(action: MvAction): String =
    mvReasonFor(action) ?: action.prompt.ifBlank { action.title }

internal fun fleetIsDestructive(action: MvAction): Boolean = action.id != "mvPause"

@Composable
private fun ControlHolderNote(station: ControlStation?, vehicleId: Int?, onRefusal: (String?) -> Unit) {
    val flyScreen = LocalFlyScreenState.current
    val holder = station ?: return
    if (holder.inControl == true) {
        InControlNote(holder, onRefusal)
        return
    }
    val line = holderLine(holder) ?: return
    val scope = rememberCoroutineScope()
    val deadlineKey = vehicleId ?: 0
    val requestEndsAt = flyScreen.controlRequestDeadlines[deadlineKey]
    LaunchedEffect(holder.takeoverAllowed) {
        if (holder.takeoverAllowed == true) flyScreen.controlRequestDeadlines.remove(deadlineKey)
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
        RequestCountdown(endsAt) { flyScreen.controlRequestDeadlines.remove(deadlineKey) }
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
                        flyScreen.controlRequestDeadlines[deadlineKey] = System.currentTimeMillis() + ask.timeoutSeconds * 1000L
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
