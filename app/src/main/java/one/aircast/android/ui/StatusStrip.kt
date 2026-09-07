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
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcFacts
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

private const val BATTERIES = "vehicle.batteries"
private const val GPS = "vehicle.gps"
private const val BATTERY_SETTINGS = "settings.batteryIndicatorSettings"

private const val CHARGE_UNDEFINED = 0
private const val CHARGE_OK = 1
private const val CHARGE_LOW = 2
private const val CHARGE_CRITICAL = 3
private const val CHARGE_UNHEALTHY = 6

private const val PERCENT_ROUNDS_TO_FULL = 98.9

internal enum class BatteryLevel { Normal, Caution, Warning, Critical }

internal fun batteryLevel(
    chargeState: Int?,
    percentRemaining: Double?,
    threshold1: Int,
    threshold2: Int,
): BatteryLevel = when (chargeState) {
    CHARGE_OK -> BatteryLevel.Normal
    CHARGE_LOW -> BatteryLevel.Warning
    in CHARGE_CRITICAL..CHARGE_UNHEALTHY -> BatteryLevel.Critical
    CHARGE_UNDEFINED -> when {
        percentRemaining == null || percentRemaining.isNaN() -> BatteryLevel.Normal
        percentRemaining > threshold1 -> BatteryLevel.Normal
        percentRemaining > threshold2 -> BatteryLevel.Caution
        else -> BatteryLevel.Warning
    }
    else -> BatteryLevel.Normal
}

internal fun rcSignalText(supportsRadio: Boolean, rssi: Int?): String? =
    if (!supportsRadio || rssi == null || rssi <= 0 || rssi > 100) null else "$rssi%"

internal fun batteryText(
    percentValue: Double?,
    percentString: String?,
    percentUnits: String,
    voltageString: String?,
    voltageUnits: String,
): String? {
    val percent = when {
        percentValue == null || percentValue.isNaN() -> null
        percentValue > PERCENT_ROUNDS_TO_FULL -> "100%"
        percentString.isNullOrBlank() -> null
        else -> percentString + percentUnits
    }
    val voltage = voltageString?.takeIf { it.isNotBlank() }?.plus(voltageUnits)
    val parts = listOfNotNull(percent, voltage)
    return if (parts.isEmpty()) null else parts.joinToString(" · ")
}

@Composable
fun StatusStrip(modifier: Modifier = Modifier) {
    val available by qgcBool("vehicles.activeVehicleAvailable")
    if (!available) return

    val gps by qgcFacts(GPS)
    val batteryJson by qgcPath(BATTERIES)
    val rcRssi by qgcDouble("vehicle.rcRSSI", Double.NaN)
    val supportsRadio by qgcBool("vehicle.supportsRadio")
    val threshold1 by qgcDouble("$BATTERY_SETTINGS.threshold1", 80.0)
    val threshold2 by qgcDouble("$BATTERY_SETTINGS.threshold2", 60.0)

    val battery = remember(batteryJson, threshold1, threshold2) {
        firstBattery(batteryJson, threshold1.toInt(), threshold2.toInt())
    }
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

private fun firstBattery(json: JSONObject?, threshold1: Int, threshold2: Int): Pair<String, Color>? {
    val elements = json?.optJSONArray("elements") ?: return null
    val first = elements.optJSONObject(0) ?: return null
    val facts = Qgc.facts("", first)
    val percent = facts.firstOrNull { it.name == "percentRemaining" }
    val voltage = facts.firstOrNull { it.name == "voltage" }
    val chargeState = facts.firstOrNull { it.name == "chargeState" }

    val text = batteryText(
        percentValue = factDouble(percent),
        percentString = percent?.valueString,
        percentUnits = percent?.units.orEmpty(),
        voltageString = voltage?.valueString,
        voltageUnits = voltage?.units.orEmpty(),
    ) ?: return null

    val level = batteryLevel(factDouble(chargeState)?.toInt(), factDouble(percent), threshold1, threshold2)
    return text to batteryLevelColour(level)
}

private fun factDouble(fact: Fact?): Double? = when (val value = fact?.value) {
    is Number -> value.toDouble()
    is String -> value.toDoubleOrNull()
    else -> null
}

private fun batteryLevelColour(level: BatteryLevel): Color = when (level) {
    BatteryLevel.Normal -> Color.Unspecified
    BatteryLevel.Caution -> Color(0xFFFFD54F)
    BatteryLevel.Warning -> Color(0xFFFFB74D)
    BatteryLevel.Critical -> Color(0xFFE57373)
}

private fun gpsColour(lock: String?): Color = when {
    lock == null -> Color.Unspecified
    lock.contains("No", ignoreCase = true) || lock.contains("None", ignoreCase = true) -> Color(0xFFE57373)
    else -> Color.Unspecified
}
