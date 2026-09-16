package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Warning
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.qgcPath
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
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }

    Row(
        modifier = modifier.let {
            if (choices.ambiguous || taken) it.clickable { picking = true } else it
        },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = activeVehicleTitle(choices, subtitle),
            style = MaterialTheme.typography.labelLarge,
            fontWeight = if (lost) FontWeight.Bold else FontWeight.Normal,
            color = if (lost) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface,
            maxLines = 1,
        )
        if (choices.ambiguous || taken) {
            val alarm = lostVehiclesText(lostVehicles(choices)) ?: controlLine(station).takeIf { taken }
            Icon(
                imageVector = if (alarm == null) Icons.Default.KeyboardArrowDown else Icons.Default.Warning,
                contentDescription = alarm ?: "Choose which vehicle to fly",
                tint = if (alarm == null) {
                    MaterialTheme.colorScheme.onSurfaceVariant
                } else {
                    MaterialTheme.colorScheme.error
                },
            )
        }
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
            choices.choices.forEach { choice ->
                ListItem(
                    headlineContent = { Text(choice.name) },
                    supportingContent = { Text(vehicleChoiceLine(choice, distinguishes)) },
                    leadingContent = {
                        Checkbox(
                            checked = choice.selected,
                            onCheckedChange = { wanted ->
                                scope.launch {
                                    withContext(Dispatchers.Default) { FleetBridge.setSelected(choice.id, wanted) }
                                }
                            },
                        )
                    },
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
            FleetControls(vehiclesJson, choices) { message -> refusal = message }
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
        ) { Text("Select all") }
        TextButton(
            enabled = choices.canDeselectAll,
            onClick = { scope.launch { withContext(Dispatchers.Default) { FleetBridge.deselectAll() } } },
        ) { Text("Deselect all") }
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
    val line = controlLine(holder) ?: return
    val scope = rememberCoroutineScope()
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
    controlWaitLine(holder)?.let { waiting ->
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
                        val refused = withContext(Dispatchers.Default) { askForControl(holder) }
                        onRefusal(refused)
                        asked = if (refused == null) label else null
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

internal fun activeVehicleId(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("activeId") }?.optInt("activeId", -1)?.takeIf { it > 0 }
