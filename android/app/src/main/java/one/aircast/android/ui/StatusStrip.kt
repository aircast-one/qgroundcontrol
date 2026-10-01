package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.TextButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject

private const val BATTERY = "view.battery"
private const val GPS_VIEW = "view.gps"



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
    FixLevel.TwoD -> if (count.isBlank()) "2D only" else "$count sats · 2D only"
    FixLevel.Good -> if (count.isBlank()) "" else "$count sats"
}

@Composable
private fun batteryLevelColour(level: BatteryLevel): Color = when (level) {
    BatteryLevel.Normal -> Color.Unspecified
    BatteryLevel.Caution -> MaterialTheme.aircast.warning
    BatteryLevel.Warning -> MaterialTheme.aircast.alert
    BatteryLevel.Critical -> MaterialTheme.colorScheme.error
}

@Composable
private fun gpsColour(fix: FixLevel): Color = when (fix) {
    FixLevel.None -> MaterialTheme.colorScheme.error
    FixLevel.TwoD -> MaterialTheme.aircast.warning
    FixLevel.Good -> Color.Unspecified
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StatusReadingsInline(modifier: Modifier = Modifier) {
    val available = hasVehicle()
    if (!available) {
        Row(modifier) { RtkIndicatorCell() }
        return
    }

    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val live = state?.staleNotice.isNullOrBlank()

    val batteryJson by qgcPath(BATTERY)
    val battery = remember(batteryJson) { batteryReading(batteryJson) }
    val linksJson by qgcPath(VEHICLE_LINKS)
    val links = remember(linksJson) { linkCell(vehicleLinks(linksJson)) }
    val gpsJson by qgcPath(GPS_VIEW)
    val gps = remember(gpsJson) { gpsStatus(gpsJson) }
    val fix = fixLevel(gps?.lock ?: Double.NaN)
    val satellites = gps?.satellites?.toString() ?: ""
    var detail by remember { mutableStateOf<StripDetail?>(null) }
    var batterySettings by remember { mutableStateOf(false) }

    Row(
        modifier.alpha(if (live) 1f else 0.45f).horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        battery?.let {
            InlineCell(it.text, batteryLevelColour(it.level)) { detail = StripDetail.Battery }
        }
        fix?.let { level ->
            satsText(level, satellites).ifBlank { null }?.let {
                InlineCell(it, gpsColour(level)) { detail = StripDetail.Gps }
            }
        }
        rcCell(state)?.let { InlineCell(it.text, if (it.lost) MaterialTheme.colorScheme.error else Color.Unspecified) }
        overrideCell(state)?.let { InlineCell(it.text, MaterialTheme.aircast.warning) }
        telemetryCell(state)?.let { InlineCell(it, Color.Unspecified) { detail = StripDetail.Telemetry } }
        links?.let {
            InlineCell(it.text, if (it.degraded) MaterialTheme.aircast.warning else Color.Unspecified) { detail = StripDetail.Links }
        }
        AircastLinkCell()
        EscIndicatorCell()
        JoystickIndicatorCell()
        RemoteIdIndicatorCell()
        GpsResilienceCell()
        RtkIndicatorCell()
        GcsBatteryCell()
        GimbalIndicatorCell()
    }

    detail?.let { shown ->
        val rows = when (shown) {
            StripDetail.Battery -> listOfNotNull(totalDraw(batteryJson)?.let { DetailRow("Total draw", it) }) + batteryDetail(batteryJson)
            StripDetail.Gps -> gpsDetail(fix, gps)
            StripDetail.Telemetry -> telemetryDetail(state?.telemetry)
            StripDetail.Links -> linkDetail(
                vehicleLinks(linksJson),
                linkNames(linksJson),
                linksJson?.optText("primary"),
            )
        }
        InstrumentSheet(instrumentTitle(shown), rows, action = if (shown == StripDetail.Battery) {
            {
                if (batteryReturnOffered(batteryJson)) BatteryReturnButton { detail = null }
                TextButton(onClick = { detail = null; batterySettings = true }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Battery Failsafes") }
                TextButton(onClick = { detail = null; AppNavigation.setupPage = POWER_SETUP_PAGE }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Vehicle Power: Configure") }
            }
        } else {
            null
        }) { detail = null }
    }

    if (batterySettings) {
        ModalBottomSheet(onDismissRequest = { batterySettings = false }) {
            ParameterForm(BATTERY_SETTINGS_PAGE)
        }
    }
}

internal const val BATTERY_SETTINGS_PAGE = "Battery Settings"
internal const val POWER_SETUP_PAGE = "Power"

internal enum class StripDetail { Battery, Gps, Links, Telemetry }

internal fun instrumentTitle(instrument: StripDetail): String = when (instrument) {
    StripDetail.Battery -> "Battery"
    StripDetail.Gps -> "GPS"
    StripDetail.Links -> "Links to this aircraft"
    StripDetail.Telemetry -> "Telemetry radio"
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun InstrumentSheet(title: String, rows: List<DetailRow>, action: (@Composable () -> Unit)? = null, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
        action?.invoke()
        when {
            rows.isEmpty() -> Text(
                text = "The vehicle has not reported anything else about this yet.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 12.dp),
            )
            else -> rows.forEach { row ->
                ListItem(
                    headlineContent = { Text(row.label) },
                    trailingContent = { Text(row.value) },
                )
            }
        }
        FootNote("Readings come from the aircraft and stop updating when it stops answering.")
    }
}

@Composable
private fun InlineCell(text: String, colour: Color, onClick: (() -> Unit)? = null) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = if (colour == Color.Unspecified) MaterialTheme.colorScheme.onSurfaceVariant else colour,
        maxLines = 1,
        modifier = if (onClick == null) Modifier else Modifier.clickable { onClick() },
    )
}

internal data class OverrideCell(val text: String)

internal fun overrideCell(state: FlyState?): OverrideCell? =
    state?.takeIf { it.rcOverride == true }?.let { OverrideCell("RC override") }

internal fun telemetryCell(state: FlyState?): String? =
    state?.telemetry?.let { "${it.localRssiDbm} dBm" }

internal fun telemetryDetail(link: TelemetryLink?): List<DetailRow> = when (link) {
    null -> emptyList()
    else -> listOfNotNull(
        DetailRow("This station", "${link.localRssiDbm} dBm"),
        link.remoteRssiDbm?.let { DetailRow("The vehicle's radio", "$it dBm") },
        link.localNoise?.let { DetailRow("Noise here", "$it") },
        link.remoteNoise?.let { DetailRow("Noise at the vehicle", "$it") },
        link.receiveErrors?.let { DetailRow("Packets lost", "$it") },
        link.errorsFixed?.let { DetailRow("Errors fixed", "$it") },
        link.txBuffer?.let { DetailRow("TX buffer", "$it") },
    )
}

@Composable
private fun BatteryReturnButton(onClosed: () -> Unit) {
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val rtl = remember(actionsJson) { guidedOffers(actionsJson)["rtl"] }
    var confirming by remember { mutableStateOf(false) }
    if (rtl?.shown != true) return
    androidx.compose.material3.TextButton(
        enabled = rtl.ready,
        onClick = { confirming = true },
        modifier = Modifier.padding(horizontal = 12.dp),
    ) { Text("Return", color = MaterialTheme.colorScheme.error) }
    if (confirming) {
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text("Return") },
            text = { Text("The aircraft will fly back to its launch point and land.") },
            confirmButton = {
                androidx.compose.material3.TextButton(onClick = {
                    confirming = false
                    onClosed()
                    one.aircast.android.bridge.offMainDetached { one.aircast.android.bridge.Qgc.invoke("vehicle.guidedModeRTL", false) }
                }) { Text("Return") }
            },
            dismissButton = { androidx.compose.material3.TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
    }
}
