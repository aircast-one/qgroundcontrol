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
import one.aircast.mapspike.optText

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
    val primary = view.optText("text")
    if (primary.isBlank()) return null
    val secondary = view.optJSONArray("packs")
        ?.optJSONObject(0)
        ?.optText("secondaryText")
        ?.takeIf { it.isNotBlank() && it != primary }
    return BatteryReading(
        text = listOfNotNull(primary, secondary).joinToString(" · "),
        level = batteryLevelOf(view.optText("level")),
    )
}


internal fun rcSignalText(supportsRadio: Boolean, rssi: Int?): String? = when {
    !supportsRadio || rssi == null || rssi > 100 -> null
    rssi == 0 -> "No signal"
    else -> "$rssi%"
}

internal enum class FixLevel { None, TwoD, Good }

internal fun fixLevel(lock: Double): FixLevel? = when {
    lock.isNaN() -> null
    lock < 2 -> FixLevel.None
    lock < 3 -> FixLevel.TwoD
    else -> FixLevel.Good
}

internal fun satsText(fix: FixLevel, count: String): String = when (fix) {
    FixLevel.None -> "No fix"
    FixLevel.TwoD -> if (count.isBlank()) "2D only" else "$count · 2D only"
    FixLevel.Good -> count
}


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
    val lock by qgcDouble("$GPS.lock")
    val fix = fixLevel(lock)

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
        fix?.let { StatusCell("Sats", satsText(it, satellites), gpsColour(it)) }
        if (fix != FixLevel.None) {
            hdop.ifBlank { null }?.let { StatusCell("HDOP", it, Color.Unspecified) }
        }
        val rssi = rcRssi.takeIf { !it.isNaN() }?.toInt()
        rcSignalText(supportsRadio, rssi)?.let {
            StatusCell("RC", it, if (rssi == 0) CRITICAL else Color.Unspecified)
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


private val CAUTION = Color(0xFFFFD54F)
private val WARNING = Color(0xFFFFB74D)
private val CRITICAL = Color(0xFFFF5252)

private fun batteryLevelColour(level: BatteryLevel): Color = when (level) {
    BatteryLevel.Normal -> Color.Unspecified
    BatteryLevel.Caution -> CAUTION
    BatteryLevel.Warning -> WARNING
    BatteryLevel.Critical -> CRITICAL
}

private fun gpsColour(fix: FixLevel): Color = when (fix) {
    FixLevel.None -> CRITICAL
    FixLevel.TwoD -> CAUTION
    FixLevel.Good -> Color.Unspecified
}
