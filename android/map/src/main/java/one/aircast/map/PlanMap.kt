package one.aircast.map

import android.content.Context
import androidx.compose.runtime.Composable
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
    onCentre: ((Double, Double) -> Unit)? = null,
    itemPanel: (@Composable (Int, TrackPoint?, () -> Unit) -> Unit)? = null,
    header: (@Composable (PlanBar) -> Unit)? = null,
    primary: (@Composable (PlanUpload) -> Unit)? = null,
    routeSettings: (@Composable () -> Unit)? = null,
    fitKey: Int = 0,
    onTemplates: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val style = remember(context) { planMapStyle(context) }

    Surface(modifier, color = MaterialTheme.colorScheme.surface) {
        PlanMapContent(style, onCentre, itemPanel, header, primary, routeSettings, fitKey, onTemplates)
    }
}
