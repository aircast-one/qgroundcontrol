package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import java.util.Locale

internal const val GPS_RTK_VIEW = "view.gpsRtk"
private const val RTK_POLL_MS = 1000L

internal data class RtkStatus(val active: Boolean, val valid: Boolean, val satellites: Int?, val durationS: Double?, val accuracyM: Double?)

private fun JSONObject.number(key: String): Double? = if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun rtkStatus(view: JSONObject?): RtkStatus? =
    view?.takeIf { it.optBoolean("connected") }?.let {
        RtkStatus(
            active = it.optBoolean("active"),
            valid = it.optBoolean("valid"),
            satellites = it.number("numSatellites")?.toInt(),
            durationS = it.number("currentDuration"),
            accuracyM = it.number("currentAccuracy"),
        )
    }

internal fun rtkRows(status: RtkStatus): List<Pair<String, String>> =
    listOfNotNull(
        "Satellites" to (status.satellites?.toString() ?: ""),
        "Duration" to "${String.format(Locale.US, "%.0f", status.durationS ?: 0.0)} s",
        status.accuracyM?.takeIf { it > 0 }?.let { (if (status.valid) "Accuracy" else "Current Accuracy") to "${String.format(Locale.US, "%.1f", it)} m" },
    )

internal fun rtkHeadline(status: RtkStatus): String = if (status.active) "Survey-in Active" else "RTK Streaming"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RtkIndicatorCell() {
    var status by remember { mutableStateOf<RtkStatus?>(null) }
    var open by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        while (isActive) {
            status = withContext(Dispatchers.Default) { rtkStatus(Qgc.get(GPS_RTK_VIEW)) }
            delay(RTK_POLL_MS)
        }
    }

    val shown = status ?: return
    Text("RTK", style = MaterialTheme.typography.labelMedium, modifier = Modifier.clickable { open = true })

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text("RTK GPS Status", style = MaterialTheme.typography.titleSmall)
                Text(rtkHeadline(shown), style = MaterialTheme.typography.bodyMedium)
                rtkRows(shown).forEach { (label, value) -> Text("$label  $value", style = MaterialTheme.typography.bodySmall) }
            }
        }
    }
}
