package one.aircast.mapspike

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

private const val MAX_BAR_DP = 140

@Composable
fun ScaleBarView(latitude: Double, zoom: Double, modifier: Modifier = Modifier) {
    val density = LocalDensity.current
    val maxPixels = with(density) { MAX_BAR_DP.dp.toPx() }.toDouble()
    val across = metresAcross(latitude, zoom, maxPixels)
    val view by produceState<JSONObject?>(null, across) {
        value = across?.let {
            withContext(Dispatchers.Default) {
                runCatching { JSONObject(QGCBridge.get("view.mapScale($it)")) }.getOrNull()
            }
        }
    }
    val scale = mapScaleBar(view, maxPixels) ?: return
    val barDp = with(density) { scale.pixels.toFloat().toDp() }

    Column(modifier) {
        Text(
            scale.text,
            style = MaterialTheme.typography.labelSmall,
            color = Color.White,
            modifier = Modifier.padding(bottom = 2.dp),
        )
        Column(
            Modifier.width(barDp).background(Color.White.copy(alpha = 0.9f))
                .padding(vertical = 2.dp),
        ) {}
    }
}
