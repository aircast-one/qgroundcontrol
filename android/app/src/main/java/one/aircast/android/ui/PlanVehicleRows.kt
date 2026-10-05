package one.aircast.android.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

private const val APP_SETTINGS = "settings.appSettings"
private val OFFLINE_CLASSES = listOf("offlineEditingFirmwareClass" to "Firmware", "offlineEditingVehicleClass" to "Vehicle")

internal fun choosesPlanVehicle(connected: Boolean, plan: JSONObject?): Boolean =
    !connected && plan?.optBoolean("hasMissionItems") != true

@Composable
internal fun PlanVehicleRows() {
    val plan by qgcPath("view.plan")
    val choosing = choosesPlanVehicle(hasVehicle(), plan)
    var reloads by remember { mutableIntStateOf(0) }
    var facts by remember { mutableStateOf(emptyList<Pair<String, Fact>>()) }
    LaunchedEffect(choosing, reloads) {
        facts = if (!choosing) emptyList() else withContext(Dispatchers.Default) {
            OFFLINE_CLASSES.mapNotNull { (name, label) -> Qgc.get("$APP_SETTINGS.$name")?.let { label to Qgc.fact(APP_SETTINGS, it.put("name", name)) } }
        }
    }
    when (choosing) {
        true -> facts.forEach { (label, fact) -> FactRow(fact, title = label) { reloads++ } }
        false -> plan?.optJSONObject("planningFor")?.let { vehicle ->
            listOf("Firmware" to vehicle.optText("firmware"), "Vehicle" to vehicle.optText("type")).filter { it.second.isNotBlank() }.forEach { (label, value) ->
                Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp)) {
                    Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(value, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
    }
}
