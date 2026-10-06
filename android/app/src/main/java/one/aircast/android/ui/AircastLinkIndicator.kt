package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import org.json.JSONArray
import org.json.JSONObject

internal const val AIRCAST_LINK_VIEW = "view.aircastLink"
private const val SIGNAL_MAXIMUM = 100.0

internal data class AircastLink(
    val qualityText: String,
    val signalText: String,
    val network: String,
    val modem: String,
    val bitrateText: String,
    val qualityHistory: List<Double>,
    val bitrateHistory: List<Double>,
)

private fun numbers(array: JSONArray?): List<Double> = (0 until (array?.length() ?: 0)).map { array?.optDouble(it) ?: -1.0 }

internal fun aircastLink(view: JSONObject?): AircastLink? = view?.takeIf { it.optBoolean("shown") }?.let {
    AircastLink(
        qualityText = it.optString("qualityText"),
        signalText = it.optString("signalText"),
        network = it.optString("network"),
        modem = it.optString("modem"),
        bitrateText = it.optString("bitrateText"),
        qualityHistory = numbers(it.optJSONArray("qualityHistory")),
        bitrateHistory = numbers(it.optJSONArray("bitrateHistory")),
    )
}

internal fun sparkRuns(values: List<Double>, maximum: Double): List<List<Pair<Float, Float>>> {
    if (values.size < 2) return emptyList()
    val peak = (if (maximum > 0) maximum else values.maxOrNull() ?: 0.0).takeIf { it > 0 } ?: 1.0
    val step = 1f / (values.size - 1)
    return values.withIndex()
        .fold(listOf(emptyList<Pair<Float, Float>>())) { runs, (index, value) ->
            when {
                value < 0 -> if (runs.last().isEmpty()) runs else runs + listOf(emptyList())
                else -> runs.dropLast(1) + listOf(runs.last() + (index * step to (1 - (minOf(value, peak) / peak)).toFloat()))
            }
        }
        .filter { it.size >= 2 }
}

@Composable
private fun Sparkline(values: List<Double>, maximum: Double) {
    val colour = MaterialTheme.aircast.success
    Canvas(Modifier.fillMaxWidth().height(48.dp)) {
        sparkRuns(values, maximum).forEach { run ->
            val path = Path().apply {
                run.forEachIndexed { index, (x, y) ->
                    val at = Offset(x * size.width, y * (size.height - 1) + 0.5f)
                    if (index == 0) moveTo(at.x, at.y) else lineTo(at.x, at.y)
                }
            }
            drawPath(path, colour, style = Stroke(width = 1.5.dp.toPx()))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun AircastLinkCell() {
    val json by qgcPath(AIRCAST_LINK_VIEW)
    val link = remember(json) { aircastLink(json) } ?: return
    var open by remember { mutableStateOf(false) }

    Text(
        "${link.qualityText} ${link.bitrateText}",
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = 1,
        modifier = Modifier.clickable { open = true },
    )

    if (open) {
        AircastSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text("Cellular link", style = MaterialTheme.typography.titleSmall)
                listOf("Signal:" to link.signalText, "Network:" to link.network, "Modem:" to link.modem, "Video bitrate:" to link.bitrateText)
                    .forEach { (label, value) -> Text("$label  $value", style = MaterialTheme.typography.bodySmall) }
                Text("Signal, up to the last hour", style = MaterialTheme.typography.labelSmall)
                Sparkline(link.qualityHistory, SIGNAL_MAXIMUM)
                Text("Video bitrate, up to the last hour", style = MaterialTheme.typography.labelSmall)
                Sparkline(link.bitrateHistory, 0.0)
            }
        }
    }
}
