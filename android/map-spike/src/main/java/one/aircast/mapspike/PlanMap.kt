package one.aircast.mapspike

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import org.maplibre.android.MapLibre

private var tileSourceInstalled = false

@Synchronized
internal fun planMapStyle(context: Context): String {
    if (!tileSourceInstalled) {
        MapBridge.start()
        MapLibre.getInstance(context)
        installQgcTileSource(context)
        tileSourceInstalled = true
    }
    return qgcRasterStyle(currentMapType())
}

@Composable
fun PlanMapScreen(
    modifier: Modifier = Modifier,
    onClear: (() -> Unit)? = null,
    onCentre: ((Double, Double) -> Unit)? = null,
    itemEditor: (@Composable (Int, TrackPoint?, () -> Unit) -> Unit)? = null,
    header: (@Composable (PlanUpload) -> Unit)? = null,
    fitKey: Int = 0,
) {
    val context = LocalContext.current
    val style = remember(context) { planMapStyle(context) }

    DisposableEffect(Unit) {
        onDispose { MapBridge.release() }
    }

    Surface(modifier, color = MaterialTheme.colorScheme.surface) {
        MapSpikeScreen(style, onClear, onCentre, itemEditor, header, fitKey)
    }
}
