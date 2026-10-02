package one.aircast.android.ui

import androidx.compose.ui.text.style.TextAlign

import androidx.compose.ui.Alignment

import androidx.compose.foundation.shape.CircleShape

import androidx.compose.foundation.layout.widthIn

import androidx.compose.foundation.layout.FlowRow

import androidx.compose.foundation.layout.ExperimentalLayoutApi

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
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
private val TEMPLATE_PANEL_WIDTH = 380.dp

private val POINTER_VERBS = Regex("""\b([Cc])lick""")

internal fun touchWording(prompt: String): String =
    POINTER_VERBS.replace(prompt) { if (it.groupValues[1] == "C") "Tap" else "tap" }

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

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun PlanTemplates(planStatus: JSONObject?, centre: Pair<Double, Double>?, onRefused: (String) -> Unit, modifier: Modifier = Modifier) {
    val state = planTemplates(planStatus)?.takeIf { it.show } ?: return
    val scope = rememberCoroutineScope()
    Surface(
        modifier.widthIn(max = TEMPLATE_PANEL_WIDTH).fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceContainer,
        shape = MaterialTheme.shapes.extraLarge,
    ) {
        Column(
            Modifier.padding(horizontal = 24.dp, vertical = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("No mission yet", style = MaterialTheme.typography.titleLarge)
            Text(touchWording(state.prompt), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                state.names.forEach { name ->
                    Surface(
                        onClick = {
                            scope.launch {
                                val (lat, lon) = centre ?: return@launch onRefused("The map has not reported its centre yet.")
                                withContext(Dispatchers.IO) { Qgc.refusalOf(CREATE_FROM_TEMPLATE, name, lat, lon) }?.let(onRefused)
                            }
                        },
                        enabled = state.enabled,
                        modifier = Modifier.alpha(if (state.enabled) 1f else DISABLED_ALPHA),
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                    ) {
                        Text(name, style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp))
                    }
                }
            }
        }
    }
}
