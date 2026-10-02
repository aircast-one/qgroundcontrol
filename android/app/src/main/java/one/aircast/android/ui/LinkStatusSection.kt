package one.aircast.android.ui

import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

internal const val LINK_STATUS_VIEW = "view.linkStatus"

internal fun linkStatusRows(view: JSONObject?): List<Pair<String, String>> {
    val rows = view?.optJSONArray("rows") ?: return emptyList()
    return (0 until rows.length()).mapNotNull { rows.optJSONObject(it) }.map { it.optString("label") to it.optString("value") }
}

@Composable
internal fun LinkStatusSection() {
    val json by qgcPath(LINK_STATUS_VIEW)
    val rows = remember(json) { linkStatusRows(json) }
    SectionHeader("Link Status (Current Vehicle)")
    if (rows.isEmpty()) {
        Text("Not connected", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp))
    }
    rows.forEach { (label, value) ->
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(label, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
            Text(value, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
