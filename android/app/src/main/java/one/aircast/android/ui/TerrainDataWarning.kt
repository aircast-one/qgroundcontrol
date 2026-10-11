package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.R
import one.aircast.android.bridge.qgcDerived
import one.aircast.map.aircast
import org.json.JSONObject

internal const val TERRAIN_DOWNLOAD = "view.terrainDownload"
internal const val TERRAIN_STALL_MS = 30_000L
internal const val TERRAIN_STALLED_TITLE = "Terrain data isn't loading"
internal const val TERRAIN_STALLED_DETAIL = "Terrain following may not work. It needs internet and terrain coverage for this area."
private val TERRAIN_DATA_WARNING_MAX_WIDTH = 420.dp
private val TERRAIN_DATA_WARNING_CORNER = 12.dp

internal data class TerrainLoad(val loaded: Long, val pending: Long) {
    val waiting: Boolean get() = pending > 0L
}

internal fun terrainLoad(view: JSONObject?): TerrainLoad = TerrainLoad(view?.optLong("loaded") ?: 0L, view?.optLong("pending") ?: 0L)

@Composable
fun TerrainDataWarning(modifier: Modifier = Modifier) {
    val load by qgcDerived(TERRAIN_DOWNLOAD, ::terrainLoad)
    var stalled by remember { mutableStateOf(false) }
    var dismissed by remember { mutableStateOf(false) }
    LaunchedEffect(load.waiting, load.loaded) {
        stalled = false
        if (!load.waiting) dismissed = false
        if (load.waiting) {
            delay(TERRAIN_STALL_MS)
            stalled = true
        }
    }
    if (!stalled || dismissed) return
    Surface(
        modifier = modifier.widthIn(max = TERRAIN_DATA_WARNING_MAX_WIDTH),
        shape = RoundedCornerShape(TERRAIN_DATA_WARNING_CORNER),
        color = MaterialTheme.aircast.warningContainer,
        contentColor = MaterialTheme.colorScheme.onSurface,
    ) {
        Row(
            Modifier.padding(start = 12.dp, top = 8.dp, bottom = 8.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Icon(painterResource(R.drawable.ic_warning), contentDescription = null, tint = MaterialTheme.aircast.warning, modifier = Modifier.size(24.dp))
            Column(Modifier.weight(1f, fill = false)) {
                Text(TERRAIN_STALLED_TITLE, style = MaterialTheme.typography.titleSmall)
                Text(TERRAIN_STALLED_DETAIL, style = MaterialTheme.typography.bodySmall)
            }
            IconButton(onClick = { dismissed = true }) { Icon(painterResource(R.drawable.ic_close), contentDescription = "Dismiss") }
        }
    }
}
