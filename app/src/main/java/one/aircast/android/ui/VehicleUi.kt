package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
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

private const val FALLBACK_TAKEOFF_ALTITUDE_METERS = 3.0
private const val INSTRUMENTS =
    "view.instruments(altitudeRelative,groundSpeed,distanceToHome,heading)"

internal data class GuidedAction(
    val name: String,
    val confirm: String,
    val destructive: Boolean,
    val run: () -> Unit,
)

internal data class GuidedAvailability(
    val takeoff: Boolean,
    val land: Boolean,
    val rtl: Boolean,
    val changeAltitude: Boolean,
)

internal fun guidedAvailability(
    armed: Boolean,
    flying: Boolean,
    guidedModeSupported: Boolean,
    takeoffSupported: Boolean,
    fixedWing: Boolean,
    flightMode: String,
    landFlightMode: String,
    rtlFlightMode: String,
): GuidedAvailability = GuidedAvailability(
    takeoff = takeoffSupported && !flying,
    land = guidedModeSupported && armed && !fixedWing && !flightMode.equals(landFlightMode, ignoreCase = true),
    rtl = guidedModeSupported && armed && flying && !flightMode.equals(rtlFlightMode, ignoreCase = true),
    changeAltitude = guidedModeSupported && armed && flying,
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
    var takeoffAltitude by remember { mutableStateOf(FALLBACK_TAKEOFF_ALTITUDE_METERS) }
    var takeoffLabel by remember { mutableStateOf("") }
    val flying by qgcBool("vehicle.flying")
    val guidedModeSupported by qgcBool("vehicle.guidedModeSupported")
    var altitudeTarget by remember { mutableStateOf<Double?>(null) }
    var altitudeSettled by remember { mutableStateOf<Double?>(null) }
    val altitudeJson by qgcPath(GUIDED_ALTITUDE)
    val altitudeRange = remember(altitudeJson) { guidedAltitude(altitudeJson) }
    val landFlightMode by qgcString("vehicle.landFlightMode")
    val rtlFlightMode by qgcString("vehicle.rtlFlightMode")
    val takeoffSupported by qgcBool("vehicle.takeoffVehicleSupported")
    val fixedWing by qgcBool("vehicle.fixedWing")
    val flightMode by qgcString("vehicle.flightMode")
    val can = guidedAvailability(
        armed, flying, guidedModeSupported, takeoffSupported, fixedWing, flightMode,
        landFlightMode, rtlFlightMode,
    )

    LaunchedEffect(available) {
        if (available) {
            withContext(Dispatchers.Default) {
                val meters = readTakeoffAltitudeMeters()
                takeoffAltitude = meters
                takeoffLabel = altitudeLabel(meters, verticalOf(meters), verticalUnits())
            }
        }
    }

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
            Button(
                onClick = {
                    pending = GuidedAction(
                        name = if (armed) "Disarm" else "Arm",
                        confirm = if (armed) {
                            "Disarming cuts the motors. In flight the aircraft will fall."
                        } else {
                            "Arming spins the propellers. Stand clear of the aircraft."
                        },
                        destructive = true,
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

            OutlinedButton(enabled = can.takeoff, onClick = {
                val altitude = takeoffAltitude
                pending = GuidedAction(
                    name = "Take off",
                    confirm = "The aircraft will climb to $takeoffLabel and hold.",
                    destructive = false,
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeTakeoff", altitude) }
                }
            }) { Text("Takeoff") }

            OutlinedButton(enabled = can.land, onClick = {
                pending = GuidedAction(
                    name = "Land",
                    confirm = "The aircraft will descend and land where it is now.",
                    destructive = false,
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeLand") }
                }
            }) { Text("Land") }

            OutlinedButton(enabled = can.rtl, onClick = {
                pending = GuidedAction(
                    name = "Return",
                    confirm = "The aircraft will fly back to its launch point and land.",
                    destructive = false,
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", false) }
                }
            }) { Text("RTL") }

            OutlinedButton(
                enabled = can.changeAltitude && altitudeRangeUsable(altitudeRange),
                onClick = {
                    altitudeTarget = altitudeRange?.current
                    altitudeSettled = altitudeRange?.current
                },
            ) { Text("Alt") }
        }

        TelemetryRow()
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

private fun verticalOf(meters: Double): Double? =
    (Qgc.invokeResult("units.metersToAppSettingsVerticalDistanceUnits", meters) as? Number)
        ?.toDouble()

private fun verticalUnits(): String? =
    Qgc.get("units").opt("appSettingsVerticalDistanceUnitsString")?.toString()

private fun readTakeoffAltitudeMeters(): Double =
    (Qgc.invokeResult("vehicle.minimumTakeoffAltitudeMeters") as? Number)?.toDouble()
        ?.takeIf { it > 0.0 }
        ?: FALLBACK_TAKEOFF_ALTITUDE_METERS

@Composable
private fun FlightModePicker(onRefusal: (String?) -> Unit) {
    val modes by qgcStrings("vehicle.flightModes")
    val current by qgcString("vehicle.flightMode")
    var expanded by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Row(verticalAlignment = Alignment.CenterVertically) {
        AssistChip(onClick = { expanded = true }, label = { Text(current.ifBlank { "Mode" }) })
        Icon(Icons.Default.KeyboardArrowDown, null)
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            modes.forEach { mode ->
                DropdownMenuItem(
                    text = { Text(mode) },
                    onClick = {
                        expanded = false
                        scope.attemptCommand(
                            action = mode,
                            report = onRefusal,
                            reached = { flightModeNow() == mode },
                        ) { Qgc.set("vehicle.flightMode", mode) }
                    },
                )
            }
        }
    }
    if (modes.isEmpty()) {
        TextButton(onClick = {}) { Text("No flight modes reported") }
    }
}
