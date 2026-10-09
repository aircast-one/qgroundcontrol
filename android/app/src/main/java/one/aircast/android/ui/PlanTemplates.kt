package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import one.aircast.map.optText
import org.json.JSONObject

internal const val CREATE_FROM_TEMPLATE = "plan.createFromTemplate"
private const val BLANK_LABEL = "Blank mission"

internal data class PlanTemplatesState(
    val enabled: Boolean,
    val names: List<String>,
    val blank: String,
)

internal fun planTemplates(view: JSONObject?): PlanTemplatesState? =
    view?.optJSONObject("templates")?.let {
        val names = it.optJSONArray("names")
        PlanTemplatesState(
            enabled = it.optBoolean("enabled"),
            names = (0 until (names?.length() ?: 0)).map { index -> names!!.optString(index) },
            blank = it.optText("blank"),
        )
    }

internal fun templateChoices(state: PlanTemplatesState?): List<Pair<String?, String>> =
    listOf<Pair<String?, String>>(null to BLANK_LABEL) +
        state?.names.orEmpty().filter { it != state?.blank }.map { it to sentenceCase(it) }

@Composable
internal fun NewPlanDialog(state: PlanTemplatesState?, replacing: Boolean, onDismiss: () -> Unit, onPick: (String?) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("New plan") },
        text = {
            Column {
                Text(
                    if (replacing) "Replaces the plan you are editing. Templates start at the map centre." else "Templates start at the map centre.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                templateChoices(state).forEach { (name, label) ->
                    TextButton(enabled = name == null || state?.enabled == true, onClick = { onPick(name) }) { Text(label) }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
