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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
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
import one.aircast.mapspike.vehicleChoices

internal data class MvAction(
    val id: String,
    val title: String,
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
    val subtitle = vehicleSubtitle(fly)
    var picking by remember { mutableStateOf(false) }
    val panelJson by qgcPath(MULTI_VEHICLE_PANEL)
    val panelEnabled = multiVehiclePanelEnabled(panelJson)
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
            lost -> MaterialTheme.colorScheme.errorContainer
            disconnected -> MaterialTheme.colorScheme.surfaceContainerHigh
            else -> MaterialTheme.aircast.successContainer
        },
        contentColor = when {
            lost -> MaterialTheme.colorScheme.onErrorContainer
            disconnected -> MaterialTheme.colorScheme.onSurface
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
            val distinguishes = linkDistinguishes(choices.choices)
            val selectionBox: @Composable (VehicleChoice) -> Unit = { choice ->
                Checkbox(
                    checked = choice.selected,
                    onCheckedChange = { wanted ->
                        scope.launch {
                            withContext(Dispatchers.Default) { FleetBridge.setSelected(choice.id, wanted) }
                        }
                    },
                )
            }
            choices.choices.forEach { choice ->
                ListItem(
                    headlineContent = { Text(choice.name) },
                    supportingContent = { Text(vehicleChoiceLine(choice, distinguishes)) },
                    leadingContent = selectionBox.takeIf { panelEnabled }?.let { box -> { box(choice) } },
                    trailingContent = {
                        if (choice.active) {
                            Icon(Icons.Default.Check, contentDescription = "Flying this one")
                        }
                    },
                    modifier = Modifier.clickable(enabled = !choice.active) {
                        scope.launch {
                            val switched = withContext(Dispatchers.Default) {
                                VehicleBridge.askFor(choice.id)
                            }
                            refusal = if (switched) null else VehicleBridge.lastRefusal
                                ?: "That vehicle did not take control."
                            if (switched) picking = false
                        }
                    },
                )
                HorizontalDivider()
            }
            refusal?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp, vertical = 12.dp),
                )
            }
            ControlHolderNote(station) { message -> refusal = message }
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
        text = fleetHeading(choices.selectedCount),
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
        ) { Text("Select All") }
        TextButton(
            enabled = choices.canDeselectAll,
            onClick = { scope.launch { withContext(Dispatchers.Default) { FleetBridge.deselectAll() } } },
        ) { Text("Deselect All") }
    }
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
            supportingContent = { Text(fleetActionLine(action)) },
            modifier = Modifier.clickable(enabled = action.ready) { confirming = action },
        )
        if (confirming?.id == action.id) {
            ConfirmTrack(
                action = GuidedAction(
                    name = action.title,
                    confirm = fleetConfirm(choices.selectedCount),
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

internal fun fleetTargetText(selectedCount: Int): String = when (selectedCount) {
    1 -> "1 aircraft"
    else -> "$selectedCount aircraft"
}

internal fun fleetHeading(selectedCount: Int): String = when (selectedCount) {
    0 -> "Select aircraft to command together"
    else -> "Command ${fleetTargetText(selectedCount)} together"
}

internal fun fleetActionLine(action: MvAction): String =
    mvReasonFor(action) ?: action.prompt.ifBlank { action.title }

internal fun fleetConfirm(selectedCount: Int): String = "This commands ${fleetTargetText(selectedCount)}."

internal fun fleetIsDestructive(action: MvAction): Boolean = action.id != "mvPause"

@Composable
private fun ControlHolderNote(station: ControlStation?, onRefusal: (String?) -> Unit) {
    val holder = station ?: return
    if (holder.inControl == true) {
        InControlNote(holder, onRefusal)
        return
    }
    val line = controlLine(holder) ?: return
    val scope = rememberCoroutineScope()
    var requestEndsAt by remember(holder.holderSystemId) { mutableStateOf<Long?>(null) }
    Text(
        text = line,
        style = MaterialTheme.typography.bodyMedium,
        color = when {
            controlIsElsewhere(holder) -> MaterialTheme.colorScheme.error
            else -> MaterialTheme.colorScheme.onSurfaceVariant
        },
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
    var asked by remember(holder.holderSystemId) { mutableStateOf<String?>(null) }
    takeoverLine(holder)?.let { takeover ->
        Text(
            text = takeover,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 24.dp),
        )
    }
    requestEndsAt?.let { endsAt ->
        RequestCountdown(endsAt) {
            requestEndsAt = null
            asked = null
        }
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
        when (val sent = asked) {
            null -> TextButton(
                onClick = {
                    scope.launch {
                        val ask = withContext(Dispatchers.Default) { askForControl(holder) }
                        onRefusal(ask.refusal)
                        asked = if (ask.refusal == null) label else null
                        requestEndsAt = if (ask.refusal == null && ask.timeoutSeconds > 0) System.currentTimeMillis() + ask.timeoutSeconds * 1000L else null
                    }
                },
                modifier = Modifier.padding(horizontal = 16.dp),
            ) { Text(label) }
            else -> SentNotice(
                name = sent,
                onDismiss = { asked = null },
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
        }
    }
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
    AllowTakeoverBox(holder, onRefusal) { allow = it }
    TextButton(
        enabled = takeoverChangeable(holder, allow),
        onClick = {
            val wanted = allow ?: return@TextButton
            scope.launch { onRefusal(withContext(Dispatchers.Default) { changeTakeover(wanted) }) }
        },
        modifier = Modifier.padding(horizontal = 16.dp),
    ) { Text("Change") }
}

internal const val MULTI_VEHICLE_PANEL = "settings.appSettings.enableMultiVehiclePanel.rawValue"

internal fun multiVehiclePanelEnabled(setting: JSONObject?): Boolean =
    setting?.takeIf { it.has("value") && !it.isNull("value") }?.optBoolean("value", true) ?: true

internal fun activeVehicleId(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("activeId") }?.optInt("activeId", -1)?.takeIf { it > 0 }
