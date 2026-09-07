package one.aircast.mapspike

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

private const val MAX_BAR_DP = 140

@Composable
fun ScaleBarView(latitude: Double, zoom: Double, modifier: Modifier = Modifier) {
    val density = LocalDensity.current
    val maxPixels = with(density) { MAX_BAR_DP.dp.toPx() }.toDouble()
    val scale = mapScale(latitude, zoom, maxPixels) ?: return
    val barDp = with(density) { scale.pixels.toFloat().toDp() }

    Column(modifier) {
        Text(
            scale.label,
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
