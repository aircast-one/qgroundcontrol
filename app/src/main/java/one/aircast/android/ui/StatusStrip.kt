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
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

private const val BATTERY = "view.battery"
private const val GPS = "vehicle.gps"



internal enum class BatteryLevel { Normal, Caution, Warning, Critical }

internal data class BatteryReading(val text: String, val level: BatteryLevel)

internal fun batteryLevelOf(name: String?): BatteryLevel = when (name) {
    "critical" -> BatteryLevel.Critical
    "warning" -> BatteryLevel.Warning
    "caution" -> BatteryLevel.Caution
    else -> BatteryLevel.Normal
}

internal fun batteryReading(view: JSONObject?): BatteryReading? {
    if (view == null || !view.optBoolean("available")) return null
    val primary = view.optString("text")
    if (primary.isBlank()) return null
    val secondary = view.optJSONArray("packs")
        ?.optJSONObject(0)
        ?.optString("secondaryText")
        ?.takeIf { it.isNotBlank() && it != primary }
    return BatteryReading(
        text = listOfNotNull(primary, secondary).joinToString(" · "),
        level = batteryLevelOf(view.optString("level")),
    )
}


internal fun rcSignalText(supportsRadio: Boolean, rssi: Int?): String? =
    if (!supportsRadio || rssi == null || rssi <= 0 || rssi > 100) null else "$rssi%"


@Composable
fun StatusStrip(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    if (!available) return

    val batteryJson by qgcPath(BATTERY)
    val rcRssi by qgcDouble("vehicle.rcRSSI", Double.NaN)
    val supportsRadio by qgcBool("vehicle.supportsRadio")
    val battery = remember(batteryJson) { batteryReading(batteryJson) }
    val satellites by qgcString("$GPS.count")
    val hdop by qgcString("$GPS.hdop")
    val lock by qgcString("$GPS.lock")

    Row(
        modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 12.dp, vertical = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(20.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        battery?.let { reading ->
            StatusCell("Battery", reading.text, batteryLevelColour(reading.level))
        }
        satellites.ifBlank { null }?.let { StatusCell("Sats", it, gpsColour(lock)) }
        hdop.ifBlank { null }?.let { StatusCell("HDOP", it, Color.Unspecified) }
        rcSignalText(supportsRadio, rcRssi.takeIf { !it.isNaN() }?.toInt())?.let {
            StatusCell("RC", it, Color.Unspecified)
        }
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


private fun batteryLevelColour(level: BatteryLevel): Color = when (level) {
    BatteryLevel.Normal -> Color.Unspecified
    BatteryLevel.Caution -> Color(0xFFFFD54F)
    BatteryLevel.Warning -> Color(0xFFFFB74D)
    BatteryLevel.Critical -> Color(0xFFFF5252)
}

private fun gpsColour(lock: String?): Color = when {
    lock == null -> Color.Unspecified
    lock.contains("No", ignoreCase = true) || lock.contains("None", ignoreCase = true) -> Color(0xFFE57373)
    else -> Color.Unspecified
}
