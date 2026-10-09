package one.aircast.map

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowLeft
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.json.JSONObject

internal data class PlanStat(val label: String, val value: String)

internal fun planStats(itemCount: Int, items: List<MissionItem>, summary: JSONObject?): List<PlanStat> = listOfNotNull(
    PlanStat("Items", itemCount.toString()),
    summaryRow(summary, "Distance")?.let { PlanStat("Distance", it) },
    summaryRow(summary, "Time")?.let { PlanStat("Time", it.removePrefix(NO_HOURS)) },
    highestAltitude(items)?.let { PlanStat("Max alt", it) },
)

internal fun highestAltitude(items: List<MissionItem>): String? =
    items.filter { it.index != HOME_ITEM && !it.altitude.isNaN() && it.altitudeText.isNotBlank() }
        .maxByOrNull { it.altitude }?.altitudeText

internal fun neighbourItem(items: List<MissionItem>, index: Int, step: Int): Int? =
    if (step < 0) items.filter { it.index < index }.maxOfOrNull { it.index }
    else items.filter { it.index > index }.minOfOrNull { it.index }

private const val NO_HOURS = "00:"
internal const val TAP_TO_ADD = "Tap the map to add a waypoint"
internal const val FIRST_TAP_SETS_HOME = "The first tap also sets home."

internal fun selectionTitle(selected: MapHit, items: List<MissionItem>): String = when (selected) {
    is MapHit.FenceVertex -> "Fence corner"
    is MapHit.Circle, is MapHit.CircleCentre, is MapHit.CircleRadius -> "Circular fence"
    is MapHit.ShapeCentre -> if (selected.fence) "Fence" else sentenceCase(patternName(selected.owner, items))
    is MapHit.ShapeRadius -> if (selected.fence) "Fence" else sentenceCase(patternName(selected.owner, items))
    is MapHit.SurveyVertex -> "${sentenceCase(patternName(selected.item, items))} corner"
    is MapHit.Rally -> "Rally point ${selected.index + 1}"
    MapHit.BreachReturn -> "Breach return point"
    is MapHit.LandingPlace -> "Landing"
    else -> "Selected"
}

@Composable
internal fun PlanStatStrip(stats: List<PlanStat>, onList: (() -> Unit)?, profileShown: Boolean?, onProfile: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(start = 12.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        stats.forEach { stat ->
            Column(Modifier.weight(1f).padding(end = 8.dp)) {
                Text(stat.label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
                Text(stat.value, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        onList?.let { IconButton(onClick = it) { Icon(Icons.AutoMirrored.Filled.List, contentDescription = "Show the plan as a list") } }
        profileShown?.let { shown ->
            IconButton(onClick = onProfile) {
                Icon(
                    if (shown) Icons.Filled.KeyboardArrowDown else Icons.Filled.KeyboardArrowUp,
                    contentDescription = if (shown) "Hide the terrain profile" else "Show the terrain profile",
                )
            }
        }
    }
}

@Composable
internal fun EmptyMissionStrip(homeSet: Boolean, onTemplates: (() -> Unit)?, onDownload: (() -> Unit)?) {
    Column(Modifier.fillMaxWidth().padding(start = 12.dp, end = 12.dp, bottom = 4.dp)) {
        Text(TAP_TO_ADD, style = MaterialTheme.typography.titleMedium)
        if (!homeSet) {
            Text(FIRST_TAP_SETS_HOME, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (onTemplates != null || onDownload != null) Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            onTemplates?.let { TextButton(onClick = it) { Text("Templates") } }
            onDownload?.let { TextButton(onClick = it) { Text("Download from vehicle") } }
        }
    }
}

@Composable
internal fun SelectionHeader(
    title: String,
    detail: String?,
    onPrevious: (() -> Unit)?,
    onNext: (() -> Unit)?,
    onDone: () -> Unit,
) {
    Row(Modifier.fillMaxWidth().padding(bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        val stepping = onPrevious != null || onNext != null
        if (stepping) IconButton(onClick = { onPrevious?.invoke() }, enabled = onPrevious != null) {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowLeft, contentDescription = "Previous item")
        }
        Column(Modifier.weight(1f).padding(start = if (stepping) 0.dp else 12.dp)) {
            Text(title, style = MaterialTheme.typography.titleLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
            detail?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2) }
        }
        if (stepping) IconButton(onClick = { onNext?.invoke() }, enabled = onNext != null) {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = "Next item")
        }
        Button(onClick = onDone, contentPadding = ButtonDefaults.ButtonWithIconContentPadding, modifier = Modifier.padding(end = 4.dp)) {
            Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(ButtonDefaults.IconSize))
            Spacer(Modifier.width(ButtonDefaults.IconSpacing))
            Text("Done")
        }
    }
}
