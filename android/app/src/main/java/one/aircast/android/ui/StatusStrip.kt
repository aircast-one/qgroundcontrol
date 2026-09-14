package one.aircast.android.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcPath
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


internal data class RcCell(val text: String, val lost: Boolean)

internal fun rcCell(state: FlyState?): RcCell? {
    if (state == null || !state.rcSupported || state.rcSignalText.isBlank()) return null
    return RcCell("${state.rcSignalText} RC", state.rcSignal == 0)
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

@Composable
fun StatusReadingsInline(modifier: Modifier = Modifier) {
    val available = hasVehicle()
    if (!available) return

    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val live = state?.staleNotice.isNullOrBlank()

    val batteryJson by qgcPath(BATTERY)
    val battery = remember(batteryJson) { batteryReading(batteryJson) }
    val linksJson by qgcPath(VEHICLE_LINKS)
    val links = remember(linksJson) { linkCell(vehicleLinks(linksJson)) }
    val satellites by qgcString("$GPS.count")
    val lock by qgcDouble("$GPS.lock")
    val fix = fixLevel(lock)

    Row(
        modifier.alpha(if (live) 1f else 0.45f).horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        battery?.let { InlineCell(it.text, batteryLevelColour(it.level)) }
        fix?.let { InlineCell("${satsText(it, satellites)} sats", gpsColour(it)) }
        rcCell(state)?.let { InlineCell(it.text, if (it.lost) CRITICAL else Color.Unspecified) }
        links?.let { InlineCell(it.text, if (it.degraded) CAUTION else Color.Unspecified) }
    }
}

@Composable
private fun InlineCell(text: String, colour: Color) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = if (colour == Color.Unspecified) MaterialTheme.colorScheme.onSurfaceVariant else colour,
        maxLines = 1,
    )
}
