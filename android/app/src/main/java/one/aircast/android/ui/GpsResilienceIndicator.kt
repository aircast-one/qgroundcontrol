package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal const val GPS_RESILIENCE_PATH = "view.gpsResilience"

internal data class ResilienceMark(val shown: Boolean, val colour: String)

internal data class ResilienceSection(val title: String, val rows: List<Pair<String, String>>)

internal data class GpsResilience(val interference: ResilienceMark, val authentication: ResilienceMark, val sections: List<ResilienceSection>)

private fun JSONObject.mark(key: String): ResilienceMark =
    optJSONObject(key)?.let { ResilienceMark(it.optBoolean("shown"), it.optText("colour")) } ?: ResilienceMark(false, "")

internal fun gpsResilience(view: JSONObject?): GpsResilience? =
    view?.takeIf { it.optBoolean("shown") }?.let {
        val listed = it.optJSONArray("sections")
        GpsResilience(
            interference = it.mark("interference"),
            authentication = it.mark("authentication"),
            sections = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
                listed!!.optJSONObject(index)?.let { section ->
                    val rows = section.optJSONArray("rows")
                    ResilienceSection(
                        section.optText("title"),
                        (0 until (rows?.length() ?: 0)).mapNotNull { r -> rows!!.optJSONObject(r)?.let { row -> row.optText("label") to row.optText("text") } },
                    )
                }
            },
        )
    }

@Composable
private fun markColour(colour: String): Color = when (colour) {
    "good" -> MaterialTheme.aircast.success
    "warning" -> MaterialTheme.aircast.warning
    "alert" -> MaterialTheme.aircast.alert
    "error" -> MaterialTheme.colorScheme.error
    else -> MaterialTheme.colorScheme.onSurfaceVariant
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GpsResilienceCell() {
    val view by qgcPath(GPS_RESILIENCE_PATH)
    val resilience = remember(view) { gpsResilience(view) } ?: return
    var open by remember { mutableStateOf(false) }

    Row(Modifier.clickable { open = true }, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        if (resilience.interference.shown) {
            Text("RF", style = MaterialTheme.typography.labelMedium, color = markColour(resilience.interference.colour))
        }
        if (resilience.authentication.shown) {
            Text("AUTH", style = MaterialTheme.typography.labelMedium, color = markColour(resilience.authentication.colour))
        }
    }

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                resilience.sections.forEach { section ->
                    Text(section.title, style = MaterialTheme.typography.titleSmall)
                    section.rows.forEach { (label, text) -> Text("$label  $text", style = MaterialTheme.typography.bodySmall) }
                }
            }
        }
    }
}
