package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import java.util.Locale

internal const val REMOTE_ID_GROUP = "remoteIDSettings"
internal const val GCS_LOCATION_BLOCK = "Ground Station Location"
private const val GCS_POSITION = "positionManager.gcsPosition"
private const val GCS_ACCURACY = "positionManager.gcsPositionHorizontalAccuracy"
private const val GCS_POLL_MS = 1000L

internal fun gcsPositionRows(position: JSONObject?, accuracy: JSONObject?): List<Pair<String, String>>? {
    val coordinate = position?.optJSONObject("value") ?: position
    if (coordinate?.optBoolean("valid") != true) return null
    val hdop = accuracy?.optDouble("value", Double.NaN) ?: Double.NaN
    return listOf(
        "Latitude" to String.format(Locale.US, "%.7f", coordinate.optDouble("latitude")),
        "Longitude" to String.format(Locale.US, "%.7f", coordinate.optDouble("longitude")),
        "HDOP" to if (hdop > 0) String.format(Locale.US, "%.1f m", hdop) else "N/A",
    )
}

@Composable
internal fun GcsPositionStatus() {
    var rows by remember { mutableStateOf<List<Pair<String, String>>?>(null) }
    LaunchedEffect(Unit) {
        while (true) {
            rows = withContext(Dispatchers.Default) { gcsPositionRows(Qgc.get(GCS_POSITION), Qgc.get(GCS_ACCURACY)) }
            delay(GCS_POLL_MS)
        }
    }
    val shown = rows ?: return
    SectionHeader("GCS position")
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        shown.forEach { (label, value) ->
            Row(Modifier.fillMaxWidth()) {
                Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                Text(value, style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}
