package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import one.aircast.android.bridge.PlanCommands
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal const val MISSION_COMPLETE_PATH = "view.missionComplete"

internal data class MissionComplete(
    val id: Long,
    val imagesTaken: Int,
    val resumeFromWaypoint: Int?,
    val batteryWarning: Boolean,
    val title: String = COMPLETE_TITLE,
    val detail: String = "",
    val resumeLabel: String = "",
)

private const val COMPLETE_TITLE = "Flight plan complete"
private const val BATTERY_SWAP = "If you are changing batteries for Resume Mission do not disconnect from the vehicle."

internal fun missionComplete(view: JSONObject?): MissionComplete? =
    view?.takeIf { it.optBoolean("open") }?.let {
        MissionComplete(
            id = it.optLong("id"),
            imagesTaken = it.optInt("imagesTaken"),
            resumeFromWaypoint = if (it.isNull("resumeFromWaypoint")) null else it.optInt("resumeFromWaypoint"),
            batteryWarning = it.optBoolean("batteryWarning"),
            title = it.optText("title").ifBlank { COMPLETE_TITLE },
            detail = it.optText("detail"),
            resumeLabel = it.optText("resumeLabel"),
        )
    }

internal fun imagesTakenText(count: Int): String? = if (count == 0) null else "$count Images Taken"

@Composable
fun MissionCompleteDialog() {
    val view by qgcPath(MISSION_COMPLETE_PATH)
    val notice = remember(view) { missionComplete(view) } ?: return
    val close = { offMainDetached { PlanCommands.dismissMissionComplete(notice.id) } }
    val remove = {
        offMainDetached {
            PlanCommands.removeAllFromVehicle()
            PlanCommands.dismissMissionComplete(notice.id)
        }
    }
    val resume = { waypoint: Int ->
        offMainDetached {
            PlanCommands.resumeMission(waypoint)
            PlanCommands.dismissMissionComplete(notice.id)
        }
    }
    AlertDialog(
        onDismissRequest = close,
        title = { Text(notice.title) },
        confirmButton = { TextButton(onClick = close) { Text("Close") } },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (notice.detail.isNotBlank()) Text(notice.detail, style = MaterialTheme.typography.bodyMedium)
                imagesTakenText(notice.imagesTaken)?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
                notice.resumeFromWaypoint?.let { waypoint ->
                    Button(onClick = { resume(waypoint) }, modifier = Modifier.fillMaxWidth()) {
                        Text(notice.resumeLabel)
                    }
                }
                OutlinedButton(onClick = close, modifier = Modifier.fillMaxWidth()) { Text("Leave plan on vehicle") }
                TextButton(onClick = remove, modifier = Modifier.fillMaxWidth()) { Text("Remove plan from vehicle") }
                if (notice.batteryWarning) Text(BATTERY_SWAP, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning)
            }
        },
    )
}
