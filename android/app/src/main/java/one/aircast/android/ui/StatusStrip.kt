package one.aircast.android.ui

import androidx.annotation.DrawableRes
import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainInOrder
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxWidth
import one.aircast.android.bridge.Fact
import androidx.compose.foundation.layout.Box
import androidx.compose.ui.layout.layout
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Surface
import androidx.compose.ui.text.font.FontWeight
import one.aircast.android.bridge.qgcFacts
import androidx.compose.foundation.layout.Column
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.offset
import androidx.compose.ui.unit.Dp
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.map.aircast
import one.aircast.map.optText
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

internal fun batteryReadings(view: JSONObject?): List<BatteryReading> =
    view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs")?.let { packs ->
        (0 until packs.length()).mapNotNull { packs.optJSONObject(it) }.mapNotNull { pack ->
            val lines = pack.optJSONArray("indicatorLines")?.let { lines -> (0 until lines.length()).map { lines.optString(it) } }.orEmpty().filter { it.isNotBlank() }
            lines.takeIf { it.isNotEmpty() }?.let {
                BatteryReading(
                    text = listOfNotNull(pack.optText("indicatorLabel").ifEmpty { null }, it.joinToString(" · ")).joinToString(" "),
                    level = batteryLevelOf(pack.optText("level")),
                )
            }
        }
    }.orEmpty()


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

internal const val NO_COUNT = "--"

internal fun satsText(fix: FixLevel, count: String): String = when (fix) {
    FixLevel.None -> "No fix"
    FixLevel.TwoD -> if (count.isBlank()) "2D only" else "$count sats · 2D only"
    FixLevel.Good -> if (count.isBlank()) NO_COUNT else "$count sats"
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
fun StatusPill(modifier: Modifier = Modifier) {
    val rtk = rememberRtkStatus()
    val gcsBattery = rememberGcsBattery()
    if (!statusPillShown(hasVehicle(), rtk != null, gcsBattery != null)) return
    Surface(
        modifier = modifier,
        shape = CircleShape,
        color = osdBackdrop(Color.Black.copy(alpha = 0.45f)),
        contentColor = MaterialTheme.aircast.outdoorForeground,
    ) {
        StatusReadingsInline(rtk, gcsBattery, Modifier.padding(end = STRIP_GAP, top = 6.dp, bottom = 6.dp))
    }
}

private val STRIP_GAP = 14.dp

private fun Modifier.leadingGap(gap: Dp): Modifier = layout { measurable, constraints ->
    val placeable = measurable.measure(constraints.offset(horizontal = -gap.roundToPx()))
    val offset = if (placeable.width > 0) gap.roundToPx() else 0
    layout(placeable.width + offset, placeable.height) { placeable.place(offset, 0) }
}

internal fun statusPillShown(vehicle: Boolean, rtk: Boolean, gcsBattery: Boolean): Boolean = vehicle || rtk || gcsBattery

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun StatusReadingsInline(rtk: RtkStatus?, gcsBattery: GcsBatteryReading?, modifier: Modifier = Modifier) {
    val navigation = LocalAppNavigation.current
    val available = hasVehicle()
    if (!available) {
        Row(modifier, verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.leadingGap(STRIP_GAP)) { RtkIndicatorCell(rtk) }
            Box(Modifier.leadingGap(STRIP_GAP)) { GcsBatteryCell(gcsBattery) }
        }
        return
    }

    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val live = state?.staleNotice.isNullOrBlank()

    val batteryJson by qgcPath(BATTERY)
    val batteries = remember(batteryJson) { batteryReadings(batteryJson) }
    val linksJson by qgcPath(VEHICLE_LINKS)
    val links = remember(linksJson) { linkCell(vehicleLinks(linksJson)) }
    val gpsJson by qgcPath(GPS_VIEW)
    val gps = remember(gpsJson) { gpsStatus(gpsJson) }
    val fix = fixLevel(gps?.lock ?: Double.NaN)
    val satellites = gps?.satellites?.toString() ?: ""
    var detail by remember { mutableStateOf<StripDetail?>(null) }
    var batterySettings by remember { mutableStateOf(false) }
    var batteryDisplay by remember { mutableStateOf(false) }
    var rtkSettings by remember { mutableStateOf(false) }
    val layout = LocalFlyScreenState.current.layout
    val setupJson by qgcPath(SETUP)
    val hasPowerSetup = remember(setupJson) { setupComponents(setupJson).any { it.name == POWER_SETUP_PAGE } }

    Row(
        modifier.alpha(if (live) 1f else 0.45f).horizontalScroll(rememberScrollState()),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val cells: List<Pair<String, @Composable () -> Unit>> = listOf(
            "battery" to { batteries.forEach { InlineCell(it.text, batteryLevelColour(it.level), R.drawable.ic_battery_5_bar) { detail = StripDetail.Battery } } },
            "gps" to {
                gps?.let {
                    InlineCell(fix?.let { satsText(it, satellites) } ?: NO_COUNT, fix?.let { gpsColour(it) } ?: Color.Unspecified, R.drawable.ic_satellite_alt) { detail = StripDetail.Gps }
                }
            },
            "rc" to { rcCell(state)?.let { InlineCell(it.text, if (it.lost) MaterialTheme.colorScheme.error else Color.Unspecified, R.drawable.ic_gamepad) { detail = StripDetail.Rc } } },
            "rcOverride" to { overrideCell(state)?.let { InlineCell(it.text, MaterialTheme.aircast.warning) { offMainInOrder { Qgc.invoke(CLEAR_RC_OVERRIDES) } } } },
            "telemetry" to { telemetryCell(state)?.let { InlineCell(it, Color.Unspecified, R.drawable.ic_sensors) { detail = StripDetail.Telemetry } } },
            "links" to { links?.let { InlineCell(it.text, if (it.degraded) MaterialTheme.aircast.warning else Color.Unspecified, R.drawable.ic_signal_cellular_alt) { detail = StripDetail.Links } } },
            "aircastLink" to { AircastLinkCell() },
            "esc" to { EscIndicatorCell() },
            "joystick" to { JoystickIndicatorCell() },
            "remoteId" to { RemoteIdIndicatorCell() },
            "gpsResilience" to { GpsResilienceCell() },
            "rtk" to { RtkIndicatorCell(rtk) },
            "gcsBattery" to { GcsBatteryCell(gcsBattery) },
            "gimbal" to { GimbalIndicatorCell() },
            "supportForwarding" to { SupportForwardingCell() },
        )
        val byKey = cells.toMap()
        val keys = orderedKeys(cells.map { it.first }, layout.indicatorOrder)
        keys.forEach { key ->
            LayoutWidget("indicator-$key", movable = false) {
                Row(Modifier.leadingGap(STRIP_GAP), verticalAlignment = Alignment.CenterVertically) {
                    if (layout.editing) TextButton(onClick = { movedKey(keys, key, -1)?.let { layout.saveIndicatorOrder(it) } }) { Text("\u2039") }
                    byKey[key]?.invoke()
                    if (layout.editing) TextButton(onClick = { movedKey(keys, key, 1)?.let { layout.saveIndicatorOrder(it) } }) { Text("\u203A") }
                }
            }
        }
    }

    detail?.let { shown ->
        val rows = when (shown) {
            StripDetail.Battery -> listOfNotNull(totalDraw(batteryJson)?.let { DetailRow("Total draw", it) }) + batteryDetail(batteryJson)
            StripDetail.Gps -> gpsDetail(gps)
            StripDetail.Telemetry -> telemetryDetail(state?.telemetry)
            StripDetail.Rc -> rcDetail(state)
            StripDetail.Links -> linkDetail(
                vehicleLinks(linksJson),
                linkNames(linksJson),
                linksJson?.optText("primary"),
            )
        }
        InstrumentSheet(instrumentTitle(shown), rows, headline = if (shown == StripDetail.Battery) batteryHeadline(batteryJson) else null, action = if (shown == StripDetail.Battery) {
            {
                if (batteryReturnOffered(batteryJson)) BatteryReturnButton { detail = null }
                TextButton(onClick = { detail = null; batterySettings = true }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Battery failsafes") }
                TextButton(onClick = { detail = null; batteryDisplay = true }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Battery display") }
                if (hasPowerSetup && advancedUiShown()) TextButton(onClick = { detail = null; navigation.setupPage = POWER_SETUP_PAGE }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Vehicle power: configure") }
            }
        } else if (shown == StripDetail.Gps) {
            {
                TextButton(onClick = { detail = null; rtkSettings = true }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("RTK GPS settings") }
            }
        } else {
            null
        }) { detail = null }
    }

    if (rtkSettings) {
        ModalBottomSheet(onDismissRequest = { rtkSettings = false }) {
            RtkSettingsSheetContent(rtk)
        }
    }

    if (batterySettings) {
        ModalBottomSheet(onDismissRequest = { batterySettings = false }) {
            indicatorParameterWait(setupJson)?.let {
                Text(it, Modifier.padding(horizontal = 24.dp, vertical = 16.dp))
            } ?: ParameterForm(BATTERY_SETTINGS_PAGE)
        }
    }

    if (batteryDisplay) {
        ModalBottomSheet(onDismissRequest = { batteryDisplay = false }) {
            BatteryDisplaySettings()
        }
    }
}

internal const val BATTERY_SETTINGS_PAGE = "Battery Settings"

internal fun indicatorParameterWait(setup: JSONObject?): String? = when {
    parametersReady(setup) -> null
    setup?.optText("parametersReason") == "skipped" -> "Parameters not available"
    else -> "Waiting for parameters…"
}
private const val BATTERY_INDICATOR_SETTINGS = "settings.batteryIndicatorSettings"
internal val BATTERY_DISPLAY_FACTS = listOf("valueDisplay", "threshold1", "threshold2")

internal fun batteryDisplayFacts(facts: List<Fact>): List<Fact> =
    BATTERY_DISPLAY_FACTS.mapNotNull { name -> facts.firstOrNull { it.name == name } }

@Composable
private fun BatteryDisplaySettings() {
    val facts by qgcFacts(BATTERY_INDICATOR_SETTINGS)
    Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
        Text("Battery display", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
        batteryDisplayFacts(facts).forEach { FactRow(it) }
    }
}
internal const val CLEAR_RC_OVERRIDES = "vehicle.clearRcChannelOverrides"
internal const val POWER_SETUP_PAGE = "Power"

internal enum class StripDetail { Battery, Gps, Links, Telemetry, Rc }

internal fun rcDetail(state: FlyState?): List<DetailRow> =
    listOfNotNull(state?.rcSignalText?.ifBlank { null }?.let { DetailRow("RSSI", it) })

internal fun instrumentTitle(instrument: StripDetail): String = when (instrument) {
    StripDetail.Battery -> "Battery"
    StripDetail.Gps -> "Vehicle GPS status"
    StripDetail.Links -> "Links to this aircraft"
    StripDetail.Telemetry -> "Telemetry RSSI status"
    StripDetail.Rc -> "RC RSSI status"
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun InstrumentSheet(title: String, rows: List<DetailRow>, headline: BatteryHeadline? = null, action: (@Composable () -> Unit)? = null, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
        )
        headline?.let { worst ->
            Column(Modifier.padding(horizontal = 24.dp, vertical = 4.dp)) {
                Text(worst.text, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold, color = severityColour(worst.severity))
                if (worst.detail.isNotBlank()) Text(worst.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
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
                    headlineContent = { Text(row.label, color = if (row.severity == SEVERITY_SECONDARY) MaterialTheme.colorScheme.onSurfaceVariant else Color.Unspecified) },
                    trailingContent = { Text(row.value, color = severityColour(row.severity)) },
                )
            }
        }
        FootNote("Readings come from the aircraft and stop updating when it stops answering.")
    }
}

@Composable
private fun severityColour(severity: Int): Color = when {
    severity >= 2 -> MaterialTheme.colorScheme.error
    severity == 1 -> MaterialTheme.aircast.warning
    severity == SEVERITY_SECONDARY -> MaterialTheme.colorScheme.onSurfaceVariant
    else -> MaterialTheme.colorScheme.onSurface
}

@Composable
private fun InlineCell(text: String, colour: Color, @DrawableRes icon: Int? = null, onClick: (() -> Unit)? = null) {
    val tint = if (colour == Color.Unspecified) MaterialTheme.colorScheme.onSurfaceVariant else colour
    Row(
        if (onClick == null) Modifier else Modifier.clickable { onClick() },
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon?.let { Icon(painterResource(it), null, tint = tint, modifier = Modifier.size(18.dp)) }
        Text(text, style = MaterialTheme.typography.labelMedium, color = tint, maxLines = 1)
    }
}

internal data class OverrideCell(val text: String)

internal fun overrideCell(state: FlyState?): OverrideCell? =
    state?.takeIf { it.rcOverride == true }?.let { OverrideCell("RC override") }

internal fun telemetryCell(state: FlyState?): String? =
    state?.telemetry?.let { "${it.localRssiDbm} dBm" }

internal fun telemetryDetail(link: TelemetryLink?): List<DetailRow> = when (link) {
    null -> emptyList()
    else -> listOfNotNull(
        DetailRow("Local RSSI:", "${link.localRssiDbm} dBm"),
        link.remoteRssiDbm?.let { DetailRow("Remote RSSI:", "$it dBm") },
        link.receiveErrors?.let { DetailRow("RX Errors:", "$it") },
        link.errorsFixed?.let { DetailRow("Errors Fixed:", "$it") },
        link.txBuffer?.let { DetailRow("TX Buffer:", "$it") },
        link.localNoise?.let { DetailRow("Local Noise:", "$it") },
        link.remoteNoise?.let { DetailRow("Remote Noise:", "$it") },
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
            text = { Text("Return to the launch position of the vehicle") },
            confirmButton = {
                androidx.compose.material3.TextButton(onClick = {
                    confirming = false
                    onClosed()
                    one.aircast.android.bridge.offMainDetached { VehicleCommands.returnToLaunch(false) }
                }) { Text("Return") }
            },
            dismissButton = { androidx.compose.material3.TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
    }
}
