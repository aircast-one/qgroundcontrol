package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
import one.aircast.android.bridge.qgcFacts
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcStrings

private const val INSTRUMENTS =
    "view.instruments(altitudeRelative,groundSpeed,distanceToHome,heading)"

internal data class GuidedAction(
    val name: String,
    val confirm: String,
    val destructive: Boolean,
    val run: () -> Unit,
)


internal data class Instrument(val label: String, val reading: String)

internal fun instruments(view: JSONObject?): List<Instrument> {
    val items = view?.optJSONArray("items") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.takeIf { !it.optBoolean("missing") }?.let { item ->
            val units = item.optString("units")
            val value = item.optString("value")
            Instrument(
                label = item.optString("label"),
                reading = if (units.isBlank()) value else "$value $units",
            )
        }
    }
}

internal fun vehicleSubtitle(
    available: Boolean,
    communicationLost: Boolean,
    flightMode: String,
    armed: Boolean,
): String = when {
    !available -> "No vehicle"
    communicationLost -> "Communication lost"
    else -> listOfNotNull(
        flightMode.ifBlank { null },
        if (armed) "Armed" else "Disarmed",
    ).joinToString(" · ")
}

@Composable
fun VehicleTitle() {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    val flightMode by qgcString("vehicle.flightMode")
    val armed by qgcBool("vehicle.armed")
    val communicationLost by qgcBool("vehicle.vehicleLinkManager.communicationLost")

    Column {
        Text("Aircast", style = MaterialTheme.typography.titleMedium)
        Text(
            text = vehicleSubtitle(available, communicationLost, flightMode, armed),
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

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TelemetryRow(modifier: Modifier = Modifier) {
    val view by qgcPath(INSTRUMENTS)
    val shown = remember(view) { instruments(view) }

    if (shown.isEmpty()) return

    FlowRow(
        modifier.fillMaxWidth().padding(8.dp),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        shown.forEach { instrument ->
            Column(
                Modifier.padding(horizontal = 6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    instrument.reading,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    instrument.label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun FlightActions(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    val armed by qgcBool("vehicle.armed")
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var takeoffTarget by remember { mutableStateOf<Double?>(null) }
    var takeoffSettled by remember { mutableStateOf<Double?>(null) }
    var takeoffRange by remember { mutableStateOf<GuidedTakeoff?>(null) }
    val flying by qgcBool("vehicle.flying")
    val guidedModeSupported by qgcBool("vehicle.guidedModeSupported")
    var altitudeTarget by remember { mutableStateOf<Double?>(null) }
    var altitudeSettled by remember { mutableStateOf<Double?>(null) }
    var altitudeRange by remember { mutableStateOf<GuidedAltitude?>(null) }
    var speedTarget by remember { mutableStateOf<Double?>(null) }
    var speedSettled by remember { mutableStateOf<Double?>(null) }
    var speedRange by remember { mutableStateOf<GuidedSpeed?>(null) }
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val offers = remember(actionsJson) { guidedOffers(actionsJson) }

    if (!available) {
        Text("Connect a vehicle to enable flight controls.", modifier.padding(16.dp))
        return
    }

    Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
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

        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            val armAction = offers[if (armed) "disarm" else "arm"]
            Button(
                enabled = armAction?.blocked != true,
                onClick = {
                    blockedReasonFor(armAction)?.let { refusal = it; return@Button }
                    pending = GuidedAction(
                        name = armAction?.title ?: if (armed) "Disarm" else "Arm",
                        confirm = armAction?.prompt?.ifBlank { null } ?: if (armed) {
                            "Disarming cuts the motors. In flight the aircraft will fall."
                        } else {
                            "Arming spins the propellers. Stand clear of the aircraft."
                        },
                        destructive = armAction?.destructive ?: true,
                    ) {
                        scope.attemptCommand(
                            action = if (armed) "Disarm" else "Arm",
                            report = { refusal = it },
                            reached = { armedNow() == !armed },
                        ) { Qgc.set("vehicle.armed", !armed) }
                    }
                },
                colors = if (armed) ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                else ButtonDefaults.buttonColors(),
            ) { Text(if (armed) "Disarm" else "Arm") }

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

            OutlinedButton(enabled = offers["rtl"]?.ready == true, onClick = {
                pending = GuidedAction(
                    name = "Return",
                    confirm = "The aircraft will fly back to its launch point and land.",
                    destructive = false,
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", false) }
                }
            }) { Text("RTL") }

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

            OutlinedButton(
                enabled = offers["changeAltitude"]?.ready == true,
                onClick = {
                    scope.launch {
                        val fresh = withContext(Dispatchers.Default) {
                            guidedAltitude(Qgc.get(GUIDED_ALTITUDE))
                        }
                        if (!altitudeRangeUsable(fresh)) {
                            refusal = "This vehicle did not report an altitude range."
                            return@launch
                        }
                        altitudeRange = fresh
                        altitudeTarget = fresh?.current
                        altitudeSettled = fresh?.current
                    }
                },
            ) { Text("Alt") }
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
            probe = withContext(Dispatchers.Default) { guidedAltitude(Qgc.get(guidedAltitudePath(at))) }
        }
        AlertDialog(
            onDismissRequest = { altitudeTarget = null },
            title = { Text("Change altitude") },
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
                }
            },
            confirmButton = {
                TextButton(
                    enabled = probe?.sends == true,
                    onClick = {
                        altitudeTarget = null
                        offMainDetached {
                            val fresh = guidedAltitude(Qgc.get(guidedAltitudePath(target)))
                            if (fresh?.sends == true) {
                                Qgc.invoke("vehicle.guidedModeChangeAltitude", fresh.deltaMeters, false)
                            }
                        }
                    },
                ) { Text("Change") }
            },
            dismissButton = {
                TextButton(onClick = { altitudeTarget = null }) { Text("Cancel") }
            },
        )
    }

    pending?.let { action ->
        AlertDialog(
            onDismissRequest = { pending = null },
            title = { Text(action.name) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
                    Text(action.confirm)
                    SlideToConfirm(
                        label = "Slide to ${action.name.lowercase()}",
                        destructive = action.destructive,
                    ) {
                        action.run()
                        pending = null
                    }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { pending = null }) { Text("Cancel") } },
        )
    }
}

// The command takes metres and the operator may be reading feet. Converting only the
private fun armedNow(): Boolean = Qgc.get("vehicle.armed").opt("value") == true

private fun flightModeNow(): String =
    Qgc.get("vehicle.flightMode").opt("value")?.toString().orEmpty()

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

internal fun altitudeLabel(meters: Double, converted: Double?, unit: String?): String =
    if (converted != null && !unit.isNullOrBlank()) {
        "${converted.roundToInt()} $unit"
    } else {
        "${meters.roundToInt()} m"
    }

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
            val shown = if (showFolded) modes.everyday + modes.folded else modes.everyday
            shown.forEach { mode ->
                DropdownMenuItem(
                    text = { Text(mode.name) },
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
