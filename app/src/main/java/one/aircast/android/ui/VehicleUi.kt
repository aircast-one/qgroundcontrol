package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import kotlin.math.roundToInt
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcFacts
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcStrings

private const val FALLBACK_TAKEOFF_ALTITUDE_METERS = 3.0

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
)

internal fun guidedAvailability(
    armed: Boolean,
    flying: Boolean,
    guidedModeSupported: Boolean,
    takeoffSupported: Boolean,
    fixedWing: Boolean,
    flightMode: String,
): GuidedAvailability = GuidedAvailability(
    takeoff = takeoffSupported && !flying,
    land = guidedModeSupported && armed && !fixedWing && !flightMode.equals("Land", ignoreCase = true),
    rtl = guidedModeSupported && armed && flying && !flightMode.equals("RTL", ignoreCase = true),
)

internal fun telemetryLabel(fact: Fact): String = fact.description.ifBlank { fact.name }

internal fun telemetryValue(fact: Fact): String =
    if (fact.units.isBlank()) fact.valueString else "${fact.valueString} ${fact.units}"

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

@Composable
fun TelemetryRow(modifier: Modifier = Modifier) {
    val facts by qgcFacts("vehicle.vehicle")
    val shown = remember(facts) {
        listOf("altitudeRelative", "groundSpeed", "distanceToHome", "heading")
            .mapNotNull { name -> facts.firstOrNull { it.name == name } }
    }

    if (shown.isEmpty()) return

    Row(modifier.fillMaxWidth().padding(8.dp), horizontalArrangement = Arrangement.SpaceEvenly) {
        shown.forEach { fact ->
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    telemetryValue(fact),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    telemetryLabel(fact),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
fun FlightActions(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    val armed by qgcBool("vehicle.armed")
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    var takeoffAltitude by remember { mutableStateOf(FALLBACK_TAKEOFF_ALTITUDE_METERS) }
    var takeoffLabel by remember { mutableStateOf("") }
    val flying by qgcBool("vehicle.flying")
    val guidedModeSupported by qgcBool("vehicle.guidedModeSupported")
    val takeoffSupported by qgcBool("vehicle.takeoffVehicleSupported")
    val fixedWing by qgcBool("vehicle.fixedWing")
    val flightMode by qgcString("vehicle.flightMode")
    val can = guidedAvailability(
        armed, flying, guidedModeSupported, takeoffSupported, fixedWing, flightMode,
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

        FlightModePicker()

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
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
                    ) { offMainDetached { Qgc.set("vehicle.armed", !armed) } }
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
                ) { offMainDetached { Qgc.invoke("vehicle.guidedModeTakeoff", altitude) } }
            }) { Text("Takeoff") }

            OutlinedButton(enabled = can.land, onClick = {
                pending = GuidedAction(
                    name = "Land",
                    confirm = "The aircraft will descend and land where it is now.",
                    destructive = false,
                ) { offMainDetached { Qgc.invoke("vehicle.guidedModeLand") } }
            }) { Text("Land") }

            OutlinedButton(enabled = can.rtl, onClick = {
                pending = GuidedAction(
                    name = "Return",
                    confirm = "The aircraft will fly back to its launch point and land.",
                    destructive = false,
                ) { offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", false) } }
            }) { Text("RTL") }
        }

        TelemetryRow()
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
// label keeps the number sent to the aircraft raw, which is what guidedModeTakeoff wants.
// If either half of the conversion is unavailable both are dropped, so the figure and the
// unit beside it can never come from different systems.
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
private fun FlightModePicker() {
    val modes by qgcStrings("vehicle.flightModes")
    val current by qgcString("vehicle.flightMode")
    var expanded by remember { mutableStateOf(false) }

    Row(verticalAlignment = Alignment.CenterVertically) {
        AssistChip(onClick = { expanded = true }, label = { Text(current.ifBlank { "Mode" }) })
        Icon(Icons.Default.KeyboardArrowDown, null)
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            modes.forEach { mode ->
                DropdownMenuItem(
                    text = { Text(mode) },
                    onClick = {
                        expanded = false
                        offMainDetached { Qgc.set("vehicle.flightMode", mode) }
                    },
                )
            }
        }
    }
    if (modes.isEmpty()) {
        TextButton(onClick = {}) { Text("No flight modes reported") }
    }
}
