package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
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
import one.aircast.map.aircast
import one.aircast.map.optText

internal fun sentIsStillShowing(name: String?, snapshotAtSend: String?, live: String?): Boolean =
    name != null && snapshotAtSend != null && snapshotAtSend == live

internal fun sentText(name: String): String = "Sent · $name"

internal const val LAND_FROM = "view.instruments(altitudeRelative)"

internal fun landFrom(view: org.json.JSONObject?): Pair<String, String>? =
    view?.optJSONArray("items")?.optJSONObject(0)?.takeIf { !it.optBoolean("missing") }
        ?.let { it.optString("value") to it.optString("units") }
        ?.takeIf { it.first.isNotBlank() && it.first != "\u2014" }

internal val MISSION_ACTIONS = setOf("startMission", "continueMission")
internal val HOLD_CONFIRM_ACTIONS = MISSION_ACTIONS + "land"

internal const val MISSION_ON_DRONE = "The mission already on the drone"

internal data class MissionIdentity(val title: String, val facts: String, val warning: String?)

internal fun missionIdentity(plan: org.json.JSONObject?, summary: org.json.JSONObject?): MissionIdentity = when {
    plan?.optBoolean("hasMissionItems") != true ->
        MissionIdentity(MISSION_ON_DRONE, "", "It isn't open in Plan, so its route can't be shown here.")
    plan.optBoolean("dirty") ->
        MissionIdentity(MISSION_ON_DRONE, "", "${planTitle(plan.optText("file"))} has changes that aren't on the drone. Upload it first to fly it.")
    else -> MissionIdentity(
        planTitle(plan.optText("file")),
        listOfNotNull(
            planStatusText(plan).ifBlank { null },
            one.aircast.map.summaryRow(summary, "Distance"),
            one.aircast.map.summaryRow(summary, "Time"),
        ).joinToString(" \u00b7 "),
        null,
    )
}

@Composable
private fun MissionIdentityCard() {
    val planJson by one.aircast.android.bridge.qgcPath("view.plan")
    val summaryJson by one.aircast.android.bridge.qgcPath("view.missionSummary")
    val mission = remember(planJson, summaryJson) { missionIdentity(planJson, summaryJson) }
    androidx.compose.material3.Surface(shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surfaceContainerHigh) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(mission.title, style = MaterialTheme.typography.titleMedium)
            if (mission.facts.isNotBlank()) Text(mission.facts, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            mission.warning?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning) }
        }
    }
}

internal fun slideLabel(name: String): String = name.ifBlank { null }?.let { "Slide to ${it.lowercase()}" } ?: "Slide to confirm"

@Composable
internal fun ConfirmTrack(
    action: GuidedAction,
    onSent: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var optionChecked by remember(action) { mutableStateOf(false) }
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(sentenceCase(action.name), style = MaterialTheme.typography.titleLarge)
        if (action.confirm.isNotBlank()) {
            Text(action.confirm, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (action.offerId in MISSION_ACTIONS) MissionIdentityCard()
        if (action.offerId == "land") {
            val fromJson by one.aircast.android.bridge.qgcPath(LAND_FROM)
            landFrom(fromJson)?.let { (value, units) -> FactTile("FROM", value, units) }
        }
        action.option?.let { option ->
            Row(
                Modifier.fillMaxWidth().clickable { optionChecked = !optionChecked },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Checkbox(checked = optionChecked, onCheckedChange = { optionChecked = it })
                Text(option.label, style = MaterialTheme.typography.labelLarge)
            }
        }
        val confirm = {
            action.option?.run?.invoke(optionChecked) ?: action.run()
            onSent()
        }
        if (action.offerId in HOLD_CONFIRM_ACTIONS) {
            HoldToConfirm(label = holdLabel(action.name), destructive = action.destructive, modifier = Modifier.padding(top = 8.dp), onConfirm = confirm)
        } else {
            SlideToConfirm(label = slideLabel(action.name), destructive = action.destructive, modifier = Modifier.padding(top = 8.dp), onConfirm = confirm)
        }
        TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
    }
}

@Composable
internal fun SlideOrCancel(label: String, onConfirm: () -> Unit, onCancel: () -> Unit) {
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        SlideToConfirm(label = slideLabel(label), modifier = Modifier.padding(top = 8.dp), onConfirm = onConfirm)
        TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
    }
}

@Composable
internal fun SentNotice(name: String, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    Text(
        text = sentText(name),
        style = MaterialTheme.typography.bodyMedium,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.primary,
        modifier = modifier.fillMaxWidth().clickable { onDismiss() },
    )
}

@Composable
private fun FactTile(label: String, value: String, units: String) {
    Column(
        Modifier
            .background(MaterialTheme.colorScheme.surfaceContainerHigh, MaterialTheme.shapes.medium)
            .padding(horizontal = 12.dp, vertical = 6.dp),
    ) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(value, style = MaterialTheme.typography.titleLarge)
            if (units.isNotBlank()) Text(units, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
