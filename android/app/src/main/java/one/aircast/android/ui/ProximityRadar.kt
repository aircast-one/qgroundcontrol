package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.offset
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

internal const val PROXIMITY_VIEW = "view.proximityRadar"
private const val SECTOR_SWEEP_DEG = 45f
private const val SECTOR_GAP_DEG = 360f / 100f
private const val ARC_START_OFFSET_DEG = -90f - 22.5f
private val RADAR_COLOUR = Color(1f, 0f, 0f, 0.5f)

internal data class RadarSector(val bearing: Int, val meters: Double?, val text: String)

internal data class ProximityRadar(val range: Double, val sectors: List<RadarSector>)

internal fun proximityRadar(view: JSONObject?): ProximityRadar? = view?.takeIf { it.optBoolean("shown") }?.let {
    val listed = it.optJSONArray("sectors")
    ProximityRadar(
        range = it.optDouble("rangeMeters", 6.0),
        sectors = (0 until (listed?.length() ?: 0)).mapNotNull { index -> listed?.optJSONObject(index) }.map { sector ->
            RadarSector(sector.optInt("bearing"), if (sector.isNull("meters")) null else sector.optDouble("meters"), sector.optString("text"))
        },
    )
}

@Composable
internal fun ProximityRadarOverlay(modifier: Modifier) {
    val json by qgcPath(PROXIMITY_VIEW)
    val radar = remember(json) { proximityRadar(json) } ?: return
    val density = LocalDensity.current
    BoxWithConstraints(modifier) {
        val widthPx = constraints.maxWidth.toFloat()
        val heightPx = constraints.maxHeight.toFloat()
        val shortest = min(widthPx, heightPx)
        val ratio = (shortest / 2f) / radar.range.toFloat()
        val (scaleX, scaleY) = (widthPx / shortest) to (heightPx / shortest)
        Canvas(Modifier.matchParentSize()) {
            radar.sectors.forEach { sector ->
                val radius = (sector.meters ?: return@forEach).toFloat() * ratio
                val size = Size(radius * 2 * scaleX, radius * 2 * scaleY)
                drawArc(
                    color = RADAR_COLOUR,
                    startAngle = ARC_START_OFFSET_DEG + sector.bearing + SECTOR_GAP_DEG,
                    sweepAngle = SECTOR_SWEEP_DEG - 2 * SECTOR_GAP_DEG,
                    useCenter = false,
                    topLeft = Offset(center.x - size.width / 2, center.y - size.height / 2),
                    size = size,
                    style = Stroke(width = widthPx / 100f),
                )
            }
        }
        radar.sectors.forEach { sector ->
            val meters = sector.meters ?: return@forEach
            val angle = -PI / 2 + PI / 180 * sector.bearing
            val x = cos(angle) * meters * ratio * scaleX
            val y = sin(angle) * meters * ratio * scaleY
            Text(
                sector.text,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.Bold,
                color = Color.White,
                modifier = Modifier.align(Alignment.Center).offset(
                    x = with(density) { x.toFloat().toDp() },
                    y = with(density) { y.toFloat().toDp() },
                ),
            )
        }
    }
}
