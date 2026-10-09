package one.aircast.map

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

private val CHIP_SIZE = 40.dp
private const val FEET = "ft"
private const val ALTITUDE_CEILING_METRES = 500.0
private const val ALTITUDE_CEILING_FEET = 1640.0
private const val SPEED_STEP = 0.5
internal val SPEED_RANGE = 0.0..30.0

internal fun altitudeRange(units: String): ClosedFloatingPointRange<Double> =
    0.0..(if (units == FEET) ALTITUDE_CEILING_FEET else ALTITUDE_CEILING_METRES)

internal fun stripRows(rows: List<ItemRow>): List<ItemRow> = rows.filter { it.index != HOME_ITEM }

@Composable
internal fun WaypointStrip(rows: List<ItemRow>, altitudes: Map<Int, String>, conflicts: Set<Int>, selected: Int?, onPick: (Int) -> Unit, modifier: Modifier = Modifier) {
    val shown = stripRows(rows)
    val list = rememberLazyListState()
    LaunchedEffect(selected) {
        shown.indexOfFirst { it.index == selected }.takeIf { it >= 0 }?.let { list.animateScrollToItem(it) }
    }
    LazyRow(modifier, state = list, horizontalArrangement = Arrangement.spacedBy(8.dp), contentPadding = PaddingValues(horizontal = 4.dp)) {
        items(shown, key = { it.index }) { row ->
            val chosen = row.index == selected
            val ink = runCatching { Color(android.graphics.Color.parseColor(row.colour)) }.getOrDefault(MaterialTheme.colorScheme.primary)
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Surface(
                onClick = { onPick(row.index) },
                shape = CircleShape,
                color = if (row.readyForSave) ink else Color.Transparent,
                contentColor = if (row.readyForSave) MaterialTheme.colorScheme.surface else MaterialTheme.aircast.warning,
                border = when {
                    row.index in conflicts -> BorderStroke(3.dp, MaterialTheme.colorScheme.error)
                    chosen -> BorderStroke(3.dp, MaterialTheme.colorScheme.onSurface)
                    row.readyForSave -> null
                    else -> BorderStroke(1.dp, MaterialTheme.aircast.warning)
                },
                modifier = Modifier.size(CHIP_SIZE).semantics {
                    contentDescription = "${sentenceCase(row.name)} ${row.seal}" + if (row.index in conflicts) ", too close to the terrain" else ""
                    this.selected = chosen
                },
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text(row.seal, style = MaterialTheme.typography.labelLarge, maxLines = 1)
                }
            }
            Text(altitudes[row.index].orEmpty(), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
            }
        }
    }
}

@Composable
internal fun WaypointSettings(
    item: MissionItem,
    globalFrame: Int?,
    onWrite: (label: String?, work: () -> Boolean) -> Unit,
) {
    val json by mapPath("view.itemFacts(${item.index})")
    val units = item.altitudeEditUnits.ifBlank { "m" }
    Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp)) {
        if (!item.altitude.isNaN() && item.index != HOME_ITEM) {
            SettingStepper(
                label = "Altitude",
                value = item.altitude,
                unit = units,
                step = 1.0,
                note = item.altitudeFrameText.ifBlank { null },
                range = altitudeRange(units),
                slider = true,
                onSet = { metres -> onWrite(null) { PlanBridge.setAltitude(item.index, metres) } },
            )
        }
        waypointSpeed(json)?.let { speed ->
            SettingStepper(
                label = "Speed",
                value = speed.value,
                unit = speed.units,
                step = SPEED_STEP,
                note = if (speed.specified) null else "Mission speed",
                range = SPEED_RANGE,
                onSet = { wanted -> onWrite(null) { setOk(speed.specifyPath, settingJson("true")) && setOk(speed.path, settingJson("$wanted")) } },
                trailing = if (speed.specified) {
                    { FilterChip(selected = false, onClick = { onWrite("Using the mission speed") { setOk(speed.specifyPath, settingJson("false")) } }, label = { Text("Auto") }) }
                } else null,
            )
        }
        if (!item.altitude.isNaN() && itemReferenceShown(globalFrame) && item.index != HOME_ITEM) {
            Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Altitude mode", style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                AltitudeModePicker(
                    item = item,
                    onPick = { raw -> onWrite("Setting the altitude frame") { PlanBridge.setAltitudeMode(item.index, raw) } },
                    globalFrameMixed = itemReferenceSelectable(globalFrame),
                )
            }
        }
        if (item.index != HOME_ITEM) WaypointActions(item.index, waypointHold(json), waypointYaw(json))
    }
}
