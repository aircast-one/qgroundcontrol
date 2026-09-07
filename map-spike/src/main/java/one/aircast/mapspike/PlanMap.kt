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

// The map has to be reachable from the app's own navigation, not only from the
// standalone activity, and its setup has to survive being entered more than
// once. MapLibre must be initialised before the tile source replaces the HTTP
// client, and the client may only be replaced once, so the style is resolved a
// single time and reused.
private var installedStyle: String? = null

// Only a style backed by QGC's own tiles is worth keeping. Entering the tab
// before QGC has opened its cache falls back to OSM, and remembering that would
// pin the fallback for the life of the process even once the cache is there.
@Synchronized
private fun planMapStyle(context: Context): String =
    installedStyle ?: run {
        MapLibre.getInstance(context)
        installQgcTileSource(context).also {
            if (it != OSM_RASTER_STYLE) {
                installedStyle = it
            }
        }
    }

@Composable
fun PlanMapScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val style = remember(context) { planMapStyle(context) }

    // Fills what the caller gives it rather than sizing itself, so a host can
    // put it in a Column under its own header and hand it a weight. Colours come
    // from the app's theme; wrapping one here would ignore the user's setting.
    DisposableEffect(Unit) {
        onDispose { MapBridge.release() }
    }

    Surface(modifier, color = MaterialTheme.colorScheme.surface) {
        MapSpikeScreen(style)
    }
}
