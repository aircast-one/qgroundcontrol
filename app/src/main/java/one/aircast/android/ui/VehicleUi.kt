package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcFacts
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcStrings

private const val TAKEOFF_ALTITUDE_METERS = 10.0

@Composable
fun VehicleTitle() {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    val flightMode by qgcString("vehicle.flightMode")
    val armed by qgcBool("vehicle.armed")

    Column {
        Text("Aircast", style = MaterialTheme.typography.titleMedium)
        Text(
            when {
                !available -> "No vehicle"
                else -> listOfNotNull(flightMode.ifBlank { null }, if (armed) "Armed" else "Disarmed").joinToString(" · ")
            },
            style = MaterialTheme.typography.bodySmall,
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
                Text(fact.valueString, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                Text(fact.name, style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}

@Composable
fun FlightActions(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    val armed by qgcBool("vehicle.armed")

    if (!available) {
        Text("Connect a vehicle to enable flight controls.", modifier.padding(16.dp))
        return
    }

    Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        FlightModePicker()

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = { Qgc.set("vehicle.armed", !armed) },
                colors = if (armed) ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                else ButtonDefaults.buttonColors(),
            ) { Text(if (armed) "Disarm" else "Arm") }

            OutlinedButton(onClick = { Qgc.invoke("vehicle.guidedModeTakeoff", TAKEOFF_ALTITUDE_METERS) }) {
                Text("Takeoff")
            }
            OutlinedButton(onClick = { Qgc.invoke("vehicle.guidedModeLand") }) { Text("Land") }
            OutlinedButton(onClick = { Qgc.invoke("vehicle.guidedModeRTL", false) }) { Text("RTL") }
        }

        TelemetryRow()
    }
}

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
                        Qgc.set("vehicle.flightMode", mode)
                    },
                )
            }
        }
    }
    if (modes.isEmpty()) {
        TextButton(onClick = {}) { Text("No flight modes reported") }
    }
}
