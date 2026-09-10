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

internal var installedStyle: String? = null

@Synchronized
internal fun planMapStyle(context: Context): String =
    installedStyle ?: run {
        MapBridge.start()
        MapLibre.getInstance(context)
        installQgcTileSource(context).also {
            if (it != OSM_RASTER_STYLE) {
                installedStyle = it
            }
        }
    }

@Composable
fun PlanMapScreen(
    modifier: Modifier = Modifier,
    onClear: (() -> Unit)? = null,
    onCentre: ((Double, Double) -> Unit)? = null,
) {
    val context = LocalContext.current
    val style = remember(context) { planMapStyle(context) }

    DisposableEffect(Unit) {
        onDispose { MapBridge.release() }
    }

    Surface(modifier, color = MaterialTheme.colorScheme.surface) {
        MapSpikeScreen(style, onClear, onCentre)
    }
}
