package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.map.optText
import org.json.JSONObject

internal val PLAN_DEFAULT_KEYS = listOf("altitude", "cruise", "hover", "ascent", "descent")

private val SPEEDS_SET_BY_FLIGHT_SPEED = setOf("cruise", "hover")

internal fun planDefaults(view: JSONObject?): List<Fact> {
    val served = view?.optJSONObject("defaults") ?: return emptyList()
    val flightSpeedSpecified = served.optJSONObject("flightSpeed")?.optBoolean("specified") == true
    return PLAN_DEFAULT_KEYS.mapNotNull { key ->
        served.optJSONObject(key)?.let(::factFromControl)?.let { fact ->
            if (flightSpeedSpecified && key in SPEEDS_SET_BY_FLIGHT_SPEED) {
                fact.copy(enabled = false, disabledReason = "The mission flight speed is used instead")
            } else {
                fact
            }
        }
    }
}

internal fun planDefaultsNote(view: JSONObject?, facts: List<Fact>): String =
    if (facts.isEmpty()) {
        "This core does not report the plan's defaults."
    } else {
        view?.optJSONObject("defaults")?.optText("speedNote").orEmpty()
    }

@Composable
internal fun PlanDefaultsDialog(view: JSONObject?, onDismiss: () -> Unit) {
    val facts = remember(view) { planDefaults(view) }
    val (altitude, speeds) = facts.partition { it.path.endsWith(".defaultMissionItemAltitude") }
    val flightSpeed = remember(view) { speedSectionOf(view?.optJSONObject("defaults")?.optJSONObject("flightSpeed")) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Plan defaults") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                planDefaultsNote(view, facts).takeIf { it.isNotBlank() }?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
                altitude.forEach { fact -> FactRow(fact) {} }
                flightSpeed?.let { speed ->
                    SpeedSectionRow(speed, withSlider = true) { written -> scope.launch { refusal = withContext(Dispatchers.Default) { written() } } }
                }
                refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
                speeds.forEach { fact -> FactRow(fact) {} }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Done") } },
    )
}
