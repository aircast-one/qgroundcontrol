package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

internal const val TERRAIN_DOWNLOAD = "view.terrainDownload"
private const val TERRAIN_HIDE_AFTER_MS = 15_000L
private val TERRAIN_GREEN = Color(0xFF00C853)

internal data class TerrainLoad(val loaded: Long, val pending: Long, val fraction: Float)

internal fun terrainLoad(view: JSONObject?): TerrainLoad? =
    view?.takeIf { it.has("loaded") }?.let { TerrainLoad(it.optLong("loaded"), it.optLong("pending"), it.optDouble("fraction", 0.0).toFloat()) }

internal fun terrainShowsNow(load: TerrainLoad): Boolean = load.pending > 0 || load.loaded != 0L

@Composable
fun TerrainProgress(modifier: Modifier = Modifier) {
    val view by qgcPath(TERRAIN_DOWNLOAD)
    val load = terrainLoad(view) ?: return
    var visible by remember { mutableStateOf(false) }
    LaunchedEffect(load.loaded, load.pending) {
        if (terrainShowsNow(load)) visible = true
        if (load.pending == 0L && visible) {
            delay(TERRAIN_HIDE_AFTER_MS)
            visible = false
        }
    }
    if (!visible) return
    Surface(modifier, color = osdBackdrop(MaterialTheme.colorScheme.surface.copy(alpha = 0.85f)), shape = MaterialTheme.shapes.medium) {
        Column(Modifier.padding(6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Text("Terrain load progress", style = MaterialTheme.typography.labelSmall)
            Box(Modifier.width(160.dp).height(18.dp).border(1.dp, TERRAIN_GREEN)) {
                Box(Modifier.fillMaxHeight().fillMaxWidth(load.fraction).background(TERRAIN_GREEN), contentAlignment = Alignment.Center) {
                    if (load.pending == 0L) Text("Done", style = MaterialTheme.typography.labelSmall)
                }
            }
        }
    }
}
