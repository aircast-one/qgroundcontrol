package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import one.aircast.map.AircastSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal const val ESC_PATH = "view.escs"

internal data class EscMotor(val title: String, val healthy: Boolean, val rows: List<Pair<String, String>>)

internal data class EscSummary(
    val onlineCount: Int,
    val healthy: Boolean,
    val healthText: String,
    val healthyMotorsText: String,
    val totalErrors: Long,
    val motors: List<EscMotor>,
)

internal fun escSummary(view: JSONObject?): EscSummary? =
    view?.takeIf { it.optBoolean("shown") }?.let {
        val listed = it.optJSONArray("motors")
        EscSummary(
            onlineCount = it.optInt("onlineCount"),
            healthy = it.optBoolean("healthy"),
            healthText = it.optText("healthText"),
            healthyMotorsText = it.optText("healthyMotorsText"),
            totalErrors = it.optLong("totalErrors"),
            motors = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
                listed!!.optJSONObject(index)?.let { motor ->
                    EscMotor(
                        title = motor.optText("title"),
                        healthy = motor.optBoolean("healthy"),
                        rows = listOf(
                            "RPM" to motor.optText("rpm"),
                            "Temp" to motor.optText("temperature"),
                            "Voltage" to motor.optText("voltage"),
                            "Current" to motor.optText("current"),
                            "Errors" to motor.optText("errors"),
                        ),
                    )
                }
            },
        )
    }

internal fun escCellText(summary: EscSummary): String = "ESC ${summary.onlineCount} ${summary.healthText}"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun EscIndicatorCell() {
    val view by qgcPath(ESC_PATH)
    val summary = remember(view) { escSummary(view) } ?: return
    var open by remember { mutableStateOf(false) }
    val good = MaterialTheme.aircast.success
    val bad = MaterialTheme.colorScheme.error

    Text(
        escCellText(summary),
        style = MaterialTheme.typography.labelMedium,
        color = if (summary.healthy) good else bad,
        maxLines = 1,
        modifier = Modifier.clickable { open = true },
    )

    if (open) {
        AircastSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("ESC Status Overview", style = MaterialTheme.typography.titleMedium)
                Text("Healthy Motors  ${summary.healthyMotorsText}", style = MaterialTheme.typography.bodyMedium)
                Text("Total Errors  ${summary.totalErrors}", style = MaterialTheme.typography.bodyMedium)
                LazyColumn(Modifier.heightIn(max = 420.dp)) {
                    items(summary.motors, key = { it.title }) { motor ->
                        Column(Modifier.padding(vertical = 6.dp)) {
                            Text(motor.title, style = MaterialTheme.typography.titleSmall, color = if (motor.healthy) good else bad)
                            motor.rows.filter { it.second.isNotBlank() }.forEach { (label, value) ->
                                Text("$label  $value", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            }
        }
    }
}
