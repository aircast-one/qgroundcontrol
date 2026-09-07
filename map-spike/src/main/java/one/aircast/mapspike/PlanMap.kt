package one.aircast.mapspike

import android.content.Context
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
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

@Synchronized
private fun planMapStyle(context: Context): String =
    installedStyle ?: run {
        MapBridge.start()
        MapLibre.getInstance(context)
        installQgcTileSource(context).also { installedStyle = it }
    }

@Composable
fun PlanMapScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val style = remember(context) { planMapStyle(context) }

    Surface(modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
        MapSpikeScreen(style)
    }
}
