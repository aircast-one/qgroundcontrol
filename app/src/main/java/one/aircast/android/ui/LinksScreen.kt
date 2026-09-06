package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

private const val LINKS_PATH = "links.linkConfigurations"

data class LinkRow(val index: Int, val name: String, val summary: String, val connected: Boolean)

@Composable
fun LinksScreen(modifier: Modifier = Modifier) {
    val json by qgcPath(LINKS_PATH)
    val rows = linkRows(json)

    Column(modifier.fillMaxSize()) {
        Text("Comm Links", style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(16.dp))

        if (rows.isEmpty()) {
            Text("No links configured. UDP autoconnect still applies.", Modifier.padding(horizontal = 16.dp))
            return@Column
        }

        LazyColumn {
            itemsIndexed(rows) { _, row ->
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(row.name, style = MaterialTheme.typography.bodyLarge)
                        Text(
                            row.summary.ifBlank { if (row.connected) "Connected" else "Not connected" },
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                    if (row.connected) {
                        OutlinedButton(onClick = { Qgc.invoke("$LINKS_PATH.${row.index}.link.disconnect") }) {
                            Text("Disconnect")
                        }
                    } else {
                        Button(onClick = { Qgc.invoke("links.createConnectedLink", "@$LINKS_PATH.${row.index}") }) {
                            Text("Connect")
                        }
                    }
                }
                HorizontalDivider()
            }
        }
    }
}

private fun linkRows(json: JSONObject?): List<LinkRow> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        LinkRow(
            index = index,
            name = element.optString("name", "Link $index"),
            summary = element.optString("summary"),
            connected = element.optJSONArray("children")?.let { children ->
                (0 until children.length()).any { children.optString(it) == "link" }
            } ?: false,
        )
    }
}
