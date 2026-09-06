package one.aircast.android.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcFacts
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcValue
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

private const val BATTERIES = "vehicle.batteries"
private const val GPS = "vehicle.gps"

@Composable
fun StatusStrip(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    if (!available) return

    val gps by qgcFacts(GPS)
    val batteryJson by qgcPath(BATTERIES)
    val rcRssi by qgcValue("vehicle.rcRSSI")

    val battery = remember(batteryJson) { firstBattery(batteryJson) }
    val satellites = remember(gps) { gps.firstOrNull { it.name == "count" }?.valueString }
    val hdop = remember(gps) { gps.firstOrNull { it.name == "hdop" }?.valueString }
    val lock = remember(gps) { gps.firstOrNull { it.name == "lock" }?.valueString }

    Row(
        modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 12.dp, vertical = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(20.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        battery?.let { cells ->
            StatusCell("Battery", cells.first, cells.second)
        }
        satellites?.let { StatusCell("Sats", it, gpsColour(lock)) }
        hdop?.let { StatusCell("HDOP", it, Color.Unspecified) }
        rcRssi?.toString()?.takeIf { it != "0" }?.let { StatusCell("RC", it, Color.Unspecified) }
    }
}

@Composable
private fun StatusCell(label: String, value: String, colour: Color) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            value,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.Bold,
            color = if (colour == Color.Unspecified) MaterialTheme.colorScheme.onSurface else colour,
        )
        Text(label, style = MaterialTheme.typography.labelSmall)
    }
}

private fun firstBattery(json: JSONObject?): Pair<String, Color>? {
    val elements = json?.optJSONArray("elements") ?: return null
    val first = elements.optJSONObject(0) ?: return null
    val facts = Qgc.facts("", first)
    val percent = facts.firstOrNull { it.name == "percentRemaining" }
    val voltage = facts.firstOrNull { it.name == "voltage" }
    if (percent == null && voltage == null) return null

    val text = listOfNotNull(percent?.valueString, voltage?.valueString).joinToString(" · ")
    return text to batteryColour(percent)
}

private fun batteryColour(percent: Fact?): Color {
    val value = percent?.valueString?.filter { it.isDigit() || it == '.' }?.toDoubleOrNull()
        ?: return Color.Unspecified
    return when {
        value <= 20 -> Color(0xFFE57373)
        value <= 40 -> Color(0xFFFFB74D)
        else -> Color.Unspecified
    }
}

private fun gpsColour(lock: String?): Color = when {
    lock == null -> Color.Unspecified
    lock.contains("No", ignoreCase = true) || lock.contains("None", ignoreCase = true) -> Color(0xFFE57373)
    else -> Color.Unspecified
}
