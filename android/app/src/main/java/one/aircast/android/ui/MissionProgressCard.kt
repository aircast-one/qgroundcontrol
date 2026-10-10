package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
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
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.optText
import org.json.JSONObject

private const val MISSION_PROGRESS = "view.missionProgress"
private val CARD_MAX_WIDTH = 360.dp

internal data class MissionProgress(val current: Int, val last: Int, val fraction: Float, val distance: String, val skipTo: Int?)

internal fun missionProgress(view: JSONObject?): MissionProgress? = view?.takeIf { it.optBoolean("shown") }?.let {
    MissionProgress(
        current = it.optInt("current"),
        last = it.optInt("last"),
        fraction = it.optDouble("fraction", 0.0).toFloat(),
        distance = listOf(it.optText("distanceToNext"), it.optText("distanceUnits")).filter { part -> part.isNotBlank() }.joinToString(" "),
        skipTo = it.optInt("skipTo").takeIf { _ -> it.optBoolean("canSkip") },
    )
}

internal fun missionProgressLine(progress: MissionProgress): String =
    listOf("To waypoint ${progress.current} of ${progress.last}", progress.distance).filter { it.isNotBlank() }.joinToString(" · ")

@Composable
fun MissionProgressCard(modifier: Modifier = Modifier) {
    val view by qgcPath(MISSION_PROGRESS)
    val progress = missionProgress(view) ?: return
    var skipTarget by remember(rememberActiveVehicleId()) { mutableStateOf<Int?>(null) }
    Surface(modifier.widthIn(max = CARD_MAX_WIDTH), shape = MaterialTheme.shapes.medium, color = osdBackdrop(MaterialTheme.colorScheme.surfaceContainerHigh)) {
        Column(Modifier.padding(AircastSpace.s3), verticalArrangement = Arrangement.spacedBy(AircastSpace.s2)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(missionProgressLine(progress), style = MaterialTheme.typography.labelLarge, modifier = Modifier.weight(1f))
                progress.skipTo?.let { next -> TextButton(onClick = { skipTarget = next }) { Text("Skip") } }
            }
            LinearProgressIndicator(progress = { progress.fraction }, modifier = Modifier.fillMaxWidth())
        }
    }
    skipTarget?.let { next -> SetWaypointSheet(next) { skipTarget = null } }
}
