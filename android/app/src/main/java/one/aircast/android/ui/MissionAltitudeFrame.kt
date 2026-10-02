package one.aircast.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.MISSION_CONTEXT
import one.aircast.mapspike.altitudeModesPath
import one.aircast.mapspike.altitudeModesView
import one.aircast.mapspike.choosable
import one.aircast.mapspike.refusalFor
import org.json.JSONObject

private const val PLAN_VIEW_PATH = "view.plan"
private const val GLOBAL_ALTITUDE_MODE = "plan.missionController.globalAltitudeMode"

internal fun globalAltitudeFrame(plan: JSONObject?): Int? =
    plan?.takeIf { it.has("globalAltitudeFrame") && !it.isNull("globalAltitudeFrame") }?.optInt("globalAltitudeFrame")

@Composable
internal fun MissionAltitudeFrame() {
    val plan by qgcPath(PLAN_VIEW_PATH)
    val current = globalAltitudeFrame(plan) ?: return
    val json by qgcPath(altitudeModesPath(MISSION_CONTEXT, current))
    val view = altitudeModesView(json)
    val picks = choosable(view)
    var open by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("Altitude frame", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        Box {
            TextButton(onClick = { open = true }, enabled = picks.size > 1) { Text(picks.firstOrNull { it.current }?.title ?: "Frame") }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                picks.forEach { offer ->
                    DropdownMenuItem(
                        text = {
                            Column {
                                Text(offer.title)
                                (refusalFor(view, offer.raw) ?: offer.help).takeIf { it.isNotBlank() }?.let { Text(it, style = MaterialTheme.typography.labelSmall) }
                            }
                        },
                        enabled = offer.enabled,
                        onClick = {
                            open = false
                            offMainDetached { Qgc.set(GLOBAL_ALTITUDE_MODE, offer.raw) }
                        },
                    )
                }
            }
        }
    }
}
