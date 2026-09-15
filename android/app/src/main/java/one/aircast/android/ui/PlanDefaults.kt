package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import org.json.JSONObject

internal val PLAN_DEFAULT_KEYS = listOf("altitude", "cruise", "hover")

internal fun planDefaults(view: JSONObject?): List<Fact> {
    val served = view?.optJSONObject("defaults") ?: return emptyList()
    return PLAN_DEFAULT_KEYS.mapNotNull { key ->
        served.optJSONObject(key)?.let(::factFromControl)
    }
}

internal fun planDefaultsNote(facts: List<Fact>): String =
    if (facts.isEmpty()) {
        "This core does not report the plan's defaults."
    } else {
        "New mission items start at this height, and a plan edited with no vehicle " +
            "connected is flown at these speeds."
    }

@Composable
internal fun PlanDefaultsDialog(view: JSONObject?, onDismiss: () -> Unit) {
    val facts = planDefaults(view)

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Plan defaults") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(planDefaultsNote(facts), style = MaterialTheme.typography.bodySmall)
                facts.forEach { fact -> FactRow(fact) {} }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Done") } },
    )
}
