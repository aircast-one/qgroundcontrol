package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.foundation.clickable
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Fact
import kotlin.math.roundToInt
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcStrings
import one.aircast.mapspike.TelemetryNumber
import one.aircast.mapspike.optText

private const val GCS_POSITION = "view.gcsPosition"


internal data class GuidedAction(
    val name: String,
    val confirm: String,
    val destructive: Boolean,
    val run: () -> Unit,
)


internal data class Instrument(val label: String, val reading: String)

internal fun rowWidth(count: Int): Int = when {
    count <= 4 -> count
    else -> (count + 1) / 2
}

internal fun operatorDistance(view: JSONObject?): List<Instrument> =
    view?.optText("distanceToVehicleText")
        ?.takeIf { it.isNotBlank() }
        ?.let { listOf(Instrument(label = "From you", reading = it)) }
        ?: emptyList()

internal const val AWAITING_READING = "\u2014"

internal fun instrumentReading(item: JSONObject): String? {
    if (!item.optBoolean("missing")) {
        val units = item.optText("units")
        val value = item.optText("value")
        return if (units.isBlank()) value else "$value $units"
    }
    return AWAITING_READING.takeIf { item.optText("missingReason") == "notReported" }
}

internal fun instruments(view: JSONObject?): List<Instrument> {
    val items = view?.optJSONArray("items") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { item ->
            instrumentReading(item)?.let { Instrument(label = item.optText("label"), reading = it) }
        }
    }
}

@Composable
fun VehicleTitle() {
    val json by qgcPath(FLY_STATE)
    val fly = remember(json) { flyState(json) }
    val communicationLost = fly?.contactLost == true

    Column {
        Text("Aircast", style = MaterialTheme.typography.titleMedium)
        Text(
            text = vehicleSubtitle(fly),
            style = MaterialTheme.typography.bodySmall,
            fontWeight = if (communicationLost) FontWeight.Bold else FontWeight.Normal,
            color = if (communicationLost) {
                MaterialTheme.colorScheme.error
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun TelemetryRow(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    var chosen by remember { mutableStateOf(readChosen(context)) }
    var choosing by remember { mutableStateOf(false) }
    val view by qgcPath(instrumentsPath(chosen))
    val gcsJson by qgcPath(GCS_POSITION)
    val shown = remember(view, gcsJson, chosen) {
        (if (showsInstruments(chosen)) instruments(view) else emptyList()) + operatorDistance(gcsJson)
    }
    val stateJson by qgcPath(FLY_STATE)
    val stale = remember(stateJson) { flyState(stateJson)?.staleNotice.orEmpty() }
    val silent = stale.isNotBlank()

    if (shown.isEmpty()) return

    if (silent) {
        Text(
            text = stale,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        )
    }

    if (choosing) {
        InstrumentSheet(
            chosen = chosen,
            onToggle = { name ->
                chosen = withInstrument(chosen, name)
                writeChosen(context, chosen)
            },
            onDismiss = { choosing = false },
        )
    }

    FlowRow(
        modifier
            .fillMaxWidth()
            .padding(8.dp)
            .alpha(if (silent) 0.45f else 1f),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalArrangement = Arrangement.spacedBy(6.dp),
        maxItemsInEachRow = rowWidth(shown.size + 1),
    ) {
        shown.forEach { instrument ->
            Column(
                Modifier.padding(horizontal = 6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    instrument.reading,
                    style = TelemetryNumber,
                )
                Text(
                    instrument.label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Column(
            Modifier
                .clickable { choosing = true }
                .padding(horizontal = 6.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = Icons.Default.Settings,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "Readings",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun FlightActions(modifier: Modifier = Modifier) {
    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val available = state?.connected == true
    val armed = state?.armed == true
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    var sentName by remember { mutableStateOf<String?>(null) }
    var sentSnapshot by remember { mutableStateOf<String?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var takeoffTarget by remember { mutableStateOf<Double?>(null) }
    var takeoffSettled by remember { mutableStateOf<Double?>(null) }
    var takeoffRange by remember { mutableStateOf<GuidedTakeoff?>(null) }
    var altitudeTarget by remember { mutableStateOf<Double?>(null) }
    var altitudeSettled by remember { mutableStateOf<Double?>(null) }
    var altitudeRange by remember { mutableStateOf<GuidedAltitude?>(null) }
    var speedTarget by remember { mutableStateOf<Double?>(null) }
    var speedSettled by remember { mutableStateOf<Double?>(null) }
    var speedRange by remember { mutableStateOf<GuidedSpeed?>(null) }
    var altitudePauses by remember { mutableStateOf(false) }
    var showMore by remember { mutableStateOf(false) }
    var showChecklist by remember { mutableStateOf(false) }
    var checklistTicked by rememberSaveable { mutableStateOf(setOf<String>()) }
    val preflightJson by qgcPath(PREFLIGHT)
    val checks = remember(preflightJson) { preflight(preflightJson) }
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val offers = remember(actionsJson) { guidedOffers(actionsJson) }
    val extras = remember(offers) { moreActions(offers) }
    val resumeFrom = remember(actionsJson) { resumeFromSequence(actionsJson) }

    if (!available) {
        Text("Connect a vehicle to enable flight controls.", modifier.padding(16.dp))
        return
    }

    val openAltitude: (Boolean) -> Unit = { pauses ->
        altitudePauses = pauses
        scope.launch {
            val fresh = withContext(Dispatchers.Default) { guidedAltitude(Qgc.get(GUIDED_ALTITUDE)) }
            if (altitudeRangeUsable(fresh)) {
                altitudeRange = fresh
                altitudeTarget = fresh?.current
                altitudeSettled = fresh?.current
            } else {
                refusal = "This vehicle did not report an altitude range."
            }
        }
    }

    Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        EmergencyStopButton(
            offer = emergencyStopOffer(offers),
            onConfirm = { action -> pending = action },
        )

        VehicleMessageBanner()

        refusal?.let { message ->
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        FlightModePicker { refusal = it }

        val liveActions = actionsJson?.toString()
        val confirming = pending
        val showingSent = sentIsStillShowing(sentName, sentSnapshot, liveActions)

        if (confirming != null) {
            ConfirmTrack(
                action = confirming,
                onSent = {
                    sentName = confirming.name
                    sentSnapshot = liveActions
                    pending = null
                },
                onCancel = { pending = null },
            )
        } else if (showingSent) {
            SentNotice(sentName.orEmpty(), onDismiss = { sentName = null })
        }

        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            val armAction = offers[if (armed) "disarm" else "arm"]
            Offered(armAction) {
                Button(
                    enabled = armAction?.ready == true,
                    onClick = {
                        pending = GuidedAction(
                            name = armAction?.title ?: if (armed) "Disarm" else "Arm",
                            confirm = armAction?.prompt?.ifBlank { null } ?: if (armed) {
                                "Disarming cuts the motors. In flight the aircraft will fall."
                            } else {
                                "Arming spins the propellers. Stand clear of the aircraft."
                            },
                            destructive = armAction?.destructive ?: true,
                        ) {
                            val target = !armed
                            scope.attemptCommand(
                                action = if (target) "Arm" else "Disarm",
                                report = { refusal = it },
                                reached = { armedNow() == target },
                            ) { Qgc.set("vehicle.armed", target) }
                        }
                    },
                    colors = if (armed) ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                    else ButtonDefaults.buttonColors(),
                ) { Text(if (armed) "Disarm" else "Arm") }
            }

            Offered(offers["takeoff"]) {
                OutlinedButton(
                    enabled = offers["takeoff"]?.ready == true,
                    onClick = {
                        scope.launch {
                            val fresh = withContext(Dispatchers.Default) {
                                guidedTakeoff(Qgc.get(GUIDED_TAKEOFF))
                            }
                            if (!takeoffRangeUsable(fresh)) {
                                refusal = "This vehicle did not report a takeoff height range."
                                return@launch
                            }
                            takeoffRange = fresh
                            takeoffTarget = fresh?.initial
                            takeoffSettled = fresh?.initial
                        }
                    },
                ) { Text(offers["takeoff"]?.title ?: "Takeoff") }
            }

            Offered(offers["land"]) {
                OutlinedButton(enabled = offers["land"]?.ready == true, onClick = {
                    pending = GuidedAction(
                        name = offers["land"]?.title ?: "Land",
                        confirm = offers["land"]?.prompt?.ifBlank { null }
                            ?: "The aircraft will descend and land where it is now.",
                        destructive = false,
                    ) {
                        offMainDetached { Qgc.invoke("vehicle.guidedModeLand") }
                    }
                }) { Text("Land") }
            }

            Offered(offers["rtl"]) {
                OutlinedButton(enabled = offers["rtl"]?.ready == true, onClick = {
                    pending = GuidedAction(
                        name = "Return",
                        confirm = "The aircraft will fly back to its launch point and land.",
                        destructive = false,
                    ) {
                        offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", false) }
                    }
                }) { Text("RTL") }
            }

            Offered(offers["changeSpeed"]) {
                OutlinedButton(
                    enabled = offers["changeSpeed"]?.ready == true,
                    onClick = {
                        scope.launch {
                            val fresh = withContext(Dispatchers.Default) {
                                guidedSpeed(Qgc.get(GUIDED_SPEED))
                            }
                            if (!speedRangeUsable(fresh)) {
                                refusal = "This vehicle did not report a speed range."
                                return@launch
                            }
                            speedRange = fresh
                            speedTarget = fresh?.initial
                            speedSettled = fresh?.initial
                        }
                    },
                ) { Text("Speed") }
            }

            Offered(offers["changeAltitude"]) {
                OutlinedButton(
                    enabled = offers["changeAltitude"]?.ready == true,
                    onClick = { openAltitude(false) },
                ) { Text("Alt") }
            }

            OutlinedButton(onClick = { showMore = true }) { Text("Actions") }
        }

        primaryBlockedReason(offers)?.let { reason ->
            Text(
                text = reason,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        TelemetryRow()
    }

    speedTarget?.let { target ->
        var probe by remember(speedTarget != null) { mutableStateOf<GuidedSpeed?>(null) }
        LaunchedEffect(speedSettled) {
            val at = speedSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) { guidedSpeed(Qgc.get(guidedSpeedPath(at))) }
        }
        AlertDialog(
            onDismissRequest = { speedTarget = null },
            title = { Text(speedRange?.label ?: "Speed") },
            text = {
                Column {
                    Text(probe?.sentence ?: "")
                    Slider(
                        value = target.toFloat(),
                        onValueChange = { speedTarget = it.toDouble() },
                        onValueChangeFinished = { speedSettled = speedTarget },
                        valueRange = (speedRange?.minimum ?: 0.0).toFloat()..
                            (speedRange?.maximum ?: 0.0).toFloat(),
                    )
                    rangeLabel(speedRange?.minimum, speedRange?.maximum, speedRange?.unit.orEmpty())
                        ?.let { RangeHint(it) }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = probe != null,
                    onClick = {
                        speedTarget = null
                        offMainDetached {
                            val fresh = guidedSpeed(Qgc.get(guidedSpeedPath(target)))
                            val method = fresh?.command
                            if (method != null) {
                                Qgc.invoke("vehicle.$method", fresh.targetMetersSecond)
                            }
                        }
                    },
                ) { Text("Set") }
            },
            dismissButton = {
                TextButton(onClick = { speedTarget = null }) { Text("Cancel") }
            },
        )
    }

    takeoffTarget?.let { target ->
        var probe by remember(takeoffTarget != null) { mutableStateOf<GuidedTakeoff?>(null) }
        LaunchedEffect(takeoffSettled) {
            val at = takeoffSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) { guidedTakeoff(Qgc.get(guidedTakeoffPath(at))) }
        }
        AlertDialog(
            onDismissRequest = { takeoffTarget = null },
            title = { Text(takeoffRange?.label?.ifBlank { null } ?: "Takeoff") },
            text = {
                Column {
                    Text(probe?.sentence ?: "")
                    Slider(
                        value = target.toFloat(),
                        onValueChange = { takeoffTarget = it.toDouble() },
                        onValueChangeFinished = { takeoffSettled = takeoffTarget },
                        valueRange = (takeoffRange?.minimum ?: 0.0).toFloat()..
                            (takeoffRange?.maximum ?: 0.0).toFloat(),
                    )
                    rangeLabel(takeoffRange?.minimum, takeoffRange?.maximum, takeoffRange?.unit.orEmpty())
                        ?.let { RangeHint(it) }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = probe != null,
                    onClick = {
                        takeoffTarget = null
                        offMainDetached {
                            val fresh = guidedTakeoff(Qgc.get(guidedTakeoffPath(target)))
                            if (fresh != null) {
                                Qgc.invoke("vehicle.guidedModeTakeoff", fresh.targetMeters)
                            }
                        }
                    },
                ) { Text("Take off") }
            },
            dismissButton = {
                TextButton(onClick = { takeoffTarget = null }) { Text("Cancel") }
            },
        )
    }

    altitudeTarget?.let { target ->
        var probe by remember(altitudeTarget != null) { mutableStateOf<GuidedAltitude?>(null) }
        LaunchedEffect(altitudeSettled) {
            val at = altitudeSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) {
                guidedAltitude(Qgc.get(guidedAltitudePath(at, altitudePauses)))
            }
        }
        AlertDialog(
            onDismissRequest = { altitudeTarget = null },
            title = { Text(if (altitudePauses) "Pause" else "Change altitude") },
            text = {
                Column {
                    Text(probe?.sentence ?: "")
                    Slider(
                        value = target.toFloat(),
                        onValueChange = { altitudeTarget = it.toDouble() },
                        onValueChangeFinished = { altitudeSettled = altitudeTarget },
                        valueRange = (altitudeRange?.minimum ?: 0.0).toFloat()..
                            (altitudeRange?.maximum ?: 0.0).toFloat(),
                    )
                    rangeLabel(altitudeRange?.minimum, altitudeRange?.maximum, altitudeRange?.unit.orEmpty())
                        ?.let { RangeHint(it) }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = probe?.sends == true,
                    onClick = {
                        altitudeTarget = null
                        val pauses = altitudePauses
                        offMainDetached {
                            val fresh = guidedAltitude(Qgc.get(guidedAltitudePath(target, pauses)))
                            if (fresh?.sends == true) {
                                Qgc.invoke("vehicle.guidedModeChangeAltitude", fresh.deltaMeters, pauses)
                            }
                        }
                    },
                ) { Text(if (altitudePauses) "Pause" else "Change") }
            },
            dismissButton = {
                TextButton(onClick = { altitudeTarget = null }) { Text("Cancel") }
            },
        )
    }

    if (showMore) {
        AlertDialog(
            onDismissRequest = { showMore = false },
            title = { Text("Actions") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    val checklistPast = checklistOffered(armed)
                    if (preflightOffered(preflightJson)) TextButton(
                        onClick = {
                            showMore = false
                            showChecklist = true
                        },
                        enabled = checklistPast == null,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(Modifier.fillMaxWidth()) {
                            Text(
                                text = "Pre-Flight Checklist",
                                fontWeight = FontWeight.Bold,
                                color = when (checklistPast) {
                                    null -> MaterialTheme.colorScheme.primary
                                    else -> MaterialTheme.colorScheme.onSurfaceVariant
                                },
                            )
                            Text(
                                text = checklistPast ?: preflightSummary(checks, checklistTicked),
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }

                    extras.forEach { offer ->
                        TextButton(
                            enabled = offer.ready,
                            onClick = {
                                showMore = false
                                if (offer.id == PAUSE) {
                                    openAltitude(true)
                                } else {
                                    guidedCommand(offer.id, resumeFrom)?.let { command ->
                                        pending = GuidedAction(
                                            name = offer.title,
                                            confirm = offer.prompt,
                                            destructive = offer.destructive,
                                            run = command,
                                        )
                                    }
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Column(Modifier.fillMaxWidth()) {
                                Text(
                                    text = offer.title,
                                    fontWeight = FontWeight.Bold,
                                    color = if (offer.destructive) MaterialTheme.colorScheme.error
                                    else MaterialTheme.colorScheme.primary,
                                )
                                Text(
                                    text = blockedReasonFor(offer) ?: offer.prompt,
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { showMore = false }) { Text("Close") } },
        )
    }

    if (showChecklist) {
        AlertDialog(
            onDismissRequest = { showChecklist = false },
            title = { Text("Pre-Flight Checklist") },
            text = {
                PreflightScreen(
                    modifier = Modifier.fillMaxWidth(),
                    ticked = checklistTicked,
                    onTicked = { checklistTicked = it },
                )
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { showChecklist = false }) { Text("Close") } },
        )
    }

}

@Composable
private fun RangeHint(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun Offered(offer: GuidedOffer?, content: @Composable () -> Unit) {
    if (offer?.shown == true) {
        content()
    }
}

private fun armedNow(): Boolean = flyState(Qgc.get(FLY_STATE))?.armed == true

private fun flightModeNow(): String = flyState(Qgc.get(FLY_STATE))?.mode.orEmpty()

private fun CoroutineScope.attemptCommand(
    action: String,
    report: (String?) -> Unit,
    reached: () -> Boolean,
    call: () -> Unit,
) {
    launch {
        report(null)
        withContext(Dispatchers.Default) { call() }
        val confirmed = withTimeoutOrNull(COMMAND_SETTLE_MS) {
            while (!withContext(Dispatchers.Default) { reached() }) {
                delay(200)
            }
            true
        } == true
        report(commandRefusal(action, confirmed))
    }
}

internal const val COMMAND_SETTLE_MS = 4000L

internal fun commandRefusal(action: String, confirmed: Boolean): String? =
    if (confirmed) null else "$action was not confirmed by the aircraft."

@Composable
private fun FlightModePicker(onRefusal: (String?) -> Unit) {
    val json by qgcPath(FLIGHT_MODES)
    val modes = remember(json) { flightModesView(json) }
    var expanded by remember { mutableStateOf(false) }
    var showFolded by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf<FlightModeOption?>(null) }
    val scope = rememberCoroutineScope()

    if (modes == null) {
        TextButton(onClick = {}, enabled = false) { Text("No flight modes reported") }
        return
    }

    fun send(mode: FlightModeOption) {
        scope.attemptCommand(
            action = mode.name,
            report = onRefusal,
            reached = { flightModeNow() == mode.name },
        ) { Qgc.set("vehicle.flightMode", mode.name) }
    }

    fun choose(mode: FlightModeOption) {
        expanded = false
        showFolded = false
        if (mode.needsConfirm) confirming = mode else send(mode)
    }

    confirming?.let { mode ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("Switch to ${mode.name}?") },
            text = { Text(mode.summary.ifBlank { "This mode changes how the aircraft responds." }) },
            confirmButton = {
                TextButton(onClick = { confirming = null; send(mode) }) { Text("Switch") }
            },
            dismissButton = {
                TextButton(onClick = { confirming = null }) { Text("Cancel") }
            },
        )
    }

    Row(verticalAlignment = Alignment.CenterVertically) {
        AssistChip(
            onClick = { expanded = true },
            enabled = modes.canSet,
            label = { Text(modes.current.ifBlank { "Mode" }) },
        )
        Icon(Icons.Default.KeyboardArrowDown, null)
        DropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false; showFolded = false },
        ) {
            modeHeading(modes)?.let { heading ->
                Text(
                    heading,
                    Modifier.padding(horizontal = 16.dp, vertical = 8.dp).widthIn(max = 280.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            val shown = if (showFolded) modes.everyday + modes.folded else modes.everyday
            shown.forEach { mode ->
                DropdownMenuItem(
                    text = {
                        Column(Modifier.widthIn(max = 280.dp)) {
                            Text(mode.name, style = MaterialTheme.typography.bodyMedium)
                            mode.summary.ifBlank { null }?.let {
                                Text(
                                    it,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    },
                    trailingIcon = if (mode.current) {
                        { Icon(Icons.Default.Check, null) }
                    } else {
                        null
                    },
                    onClick = { choose(mode) },
                )
            }
            if (modes.folded.isNotEmpty() && !showFolded) {
                DropdownMenuItem(
                    text = { Text("More modes") },
                    onClick = { showFolded = true },
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun InstrumentSheet(
    chosen: List<String>,
    onToggle: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var groups by remember { mutableStateOf(emptyList<InstrumentGroup>()) }
    val connected = hasVehicle()
    LaunchedEffect(connected) {
        groups = withContext(Dispatchers.Default) {
            listOfNotNull(vehicleOwnGroup(Qgc.get(VEHICLE_FACTS))) + instrumentGroups(Qgc.get(INSTRUMENT_GROUPS))
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        SectionHeader("Readings on the flight screen")
        FootNote(instrumentChoiceNote(chosen))
        if (groups.isEmpty()) {
            FootNote(emptyCatalogueText(connected))
            return@ModalBottomSheet
        }
        LazyColumn(Modifier.fillMaxWidth()) {
            groups.forEach { group ->
                item(key = "head${group.group}") { SectionHeader(group.title) }
                items(group.facts, key = { it.path }) { fact ->
                    val picked = fact.path in chosen
                    val choosable = picked || chosen.size < MOST_INSTRUMENTS
                    ListItem(
                        headlineContent = { Text(fact.label) },
                        trailingContent = {
                            Checkbox(checked = picked, onCheckedChange = null, enabled = choosable)
                        },
                        colors = ListItemDefaults.colors(
                            headlineColor = when {
                                choosable -> MaterialTheme.colorScheme.onSurface
                                else -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                            },
                        ),
                        modifier = Modifier.clickable(enabled = choosable) { onToggle(fact.path) },
                    )
                }
            }
        }
    }
}

