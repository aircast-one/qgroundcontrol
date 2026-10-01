package one.aircast.android.ui

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
    SectionHeader("Link Status")
    if (rows.isEmpty()) {
        Text("Not Connected", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 20.dp))
    }
    rows.forEach { (label, value) ->
        Text("$label  $value", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp))
    }
}
