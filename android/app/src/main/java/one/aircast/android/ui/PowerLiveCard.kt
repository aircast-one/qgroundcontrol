package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

internal data class PowerTile(val label: String, val value: String, val units: String)

private val POWER_TILE_FACTS = listOf("voltage" to "VOLTAGE", "current" to "CURRENT", "mahConsumed" to "USED")

internal fun powerTiles(view: JSONObject?): List<PowerTile> {
    val facts = view?.takeIf { it.optBoolean("available") }?.optJSONArray("packs")?.optJSONObject(0)?.optJSONArray("facts") ?: return emptyList()
    val byName = (0 until facts.length()).mapNotNull { facts.optJSONObject(it) }.associateBy { it.optText("name") }
    return POWER_TILE_FACTS.mapNotNull { (name, label) ->
        byName[name]?.let { PowerTile(label, it.optText("valueString"), it.optText("units").let { units -> if (units == "v") "V" else units }) }
    }
}

@Composable
internal fun PowerLiveCard(modifier: Modifier = Modifier) {
    val json by qgcPath("view.battery")
    val tiles = remember(json) { powerTiles(json) }
    if (tiles.isEmpty()) return
    Surface(modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.large) {
        Row(Modifier.padding(16.dp), horizontalArrangement = Arrangement.SpaceBetween) {
            tiles.forEach { tile ->
                Column {
                    Text(tile.label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(tile.value, style = MaterialTheme.typography.titleLarge)
                        Text(tile.units, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(bottom = 3.dp))
                    }
                }
            }
        }
    }
}
