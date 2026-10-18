package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

internal const val LENS_VIEW = "view.lens"
internal const val LENS_MEASURE = "camera.measureLens"
internal const val LENS_FORGET = "camera.forgetLens"
internal const val APPROXIMATE_AIM = "Approximate aim · measure the lens in camera details"

internal data class LensReading(
    val summary: String,
    val approximate: Boolean,
    val measuring: Boolean,
    val progress: String,
    val error: String?,
    val canMeasure: Boolean,
    val reason: String?,
    val measuredZooms: List<Double>,
)

internal fun lensReading(view: JSONObject?): LensReading? {
    if (view == null || view.optText("class") != "Lens") return null
    val samples = view.optJSONArray("samples")
    return LensReading(
        summary = view.optText("summary"),
        approximate = view.optBoolean("approximate"),
        measuring = view.optBoolean("measuring"),
        progress = view.optText("progress"),
        error = view.optText("error").ifBlank { null },
        canMeasure = view.optBoolean("canMeasure"),
        reason = view.optText("reason").ifBlank { null },
        measuredZooms = (0 until (samples?.length() ?: 0)).mapNotNull { samples?.optJSONObject(it)?.optDouble("zoom") },
    )
}

internal fun measuredText(zooms: List<Double>): String? =
    zooms.takeIf { it.isNotEmpty() }?.sorted()?.joinToString(prefix = "Measured at ", postfix = " zoom") { "${it.toInt()}%" }

@Composable
internal fun LensSection(modifier: Modifier = Modifier) {
    val json by qgcPath(LENS_VIEW)
    val lens = remember(json) { lensReading(json) } ?: return
    Column(modifier) {
        SectionHeader("Lens")
        Text(lens.summary, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
        measuredText(lens.measuredZooms)?.let { Text(it, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 16.dp)) }
        if (lens.measuring) Text(lens.progress, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
        lens.error?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)) }
        TextButton(onClick = { offMainDetached { Qgc.invoke(LENS_MEASURE) } }, enabled = lens.canMeasure, modifier = Modifier.padding(horizontal = 4.dp)) {
            Text(if (lens.measuring) "Measuring…" else "Measure the lens")
        }
        if (!lens.measuring) lens.reason?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp)) }
        if (lens.measuredZooms.isNotEmpty() && !lens.measuring) {
            TextButton(onClick = { offMainDetached { Qgc.invoke(LENS_FORGET) } }, modifier = Modifier.padding(horizontal = 4.dp)) { Text("Forget measurements") }
        }
    }
}
