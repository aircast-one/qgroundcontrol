package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val CREATE_FROM_TEMPLATE = "plan.createFromTemplate"
private const val DISABLED_ALPHA = 0.5f
private val TEMPLATE_PANEL_WIDTH = 220.dp

internal data class PlanTemplatesState(val show: Boolean, val enabled: Boolean, val prompt: String, val names: List<String>)

internal fun planTemplates(view: JSONObject?): PlanTemplatesState? =
    view?.optJSONObject("templates")?.let {
        val names = it.optJSONArray("names")
        PlanTemplatesState(
            show = it.optBoolean("show"),
            enabled = it.optBoolean("enabled"),
            prompt = it.optText("prompt"),
            names = (0 until (names?.length() ?: 0)).map { index -> names!!.optString(index) },
        )
    }

@Composable
fun PlanTemplates(planStatus: JSONObject?, centre: Pair<Double, Double>?, onRefused: (String) -> Unit, modifier: Modifier = Modifier) {
    val state = planTemplates(planStatus)?.takeIf { it.show } ?: return
    val scope = rememberCoroutineScope()
    Surface(
        modifier.width(TEMPLATE_PANEL_WIDTH),
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
        shape = MaterialTheme.shapes.medium,
    ) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Plan Templates", style = MaterialTheme.typography.titleSmall)
            Text(state.prompt, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            state.names.forEach { name ->
                OutlinedButton(
                    onClick = {
                        scope.launch {
                            val (lat, lon) = centre ?: return@launch onRefused("The map has not reported its centre yet.")
                            withContext(Dispatchers.IO) { Qgc.refusalOf(CREATE_FROM_TEMPLATE, name, lat, lon) }?.let(onRefused)
                        }
                    },
                    enabled = state.enabled,
                    modifier = Modifier.fillMaxWidth().alpha(if (state.enabled) 1f else DISABLED_ALPHA),
                ) { Text(name) }
            }
        }
    }
}
