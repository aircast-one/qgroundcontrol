package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
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
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

private const val OBSTACLE_PATH = "view.obstacle"
private val ARC_SIZE = 48.dp
private const val SWEEP_PADDING = 0.85f

internal fun arcSweep(increment: Double): Float = (increment * SWEEP_PADDING).toFloat()

internal const val NEAREST_VISIBLE_FRACTION = 0.12f

internal fun arcRadiusFraction(metres: Double, ceiling: Double): Float = when {
    ceiling <= 0.0 -> 0f
    else -> kotlin.math.sqrt((metres / ceiling).coerceIn(0.0, 1.0)).toFloat()
        .coerceAtLeast(NEAREST_VISIBLE_FRACTION)
}

internal fun sampleIsClose(metres: Double, floorMetres: Double): Boolean =
    metres < floorMetres * 2.0

internal enum class ArcTone { Alarm, Calm, Stale }

internal fun arcTone(near: Boolean, stale: Boolean): ArcTone = when {
    stale -> ArcTone.Stale
    near -> ArcTone.Alarm
    else -> ArcTone.Calm
}

@Composable
fun ObstacleArc(modifier: Modifier = Modifier) {
    val view by qgcPath(OBSTACLE_PATH)
    val ring = remember(view) { obstacleRing(view) }

    if (ring == null) {
        return
    }

    val alarm = MaterialTheme.colorScheme.error
    val calm = MaterialTheme.colorScheme.onSurfaceVariant
    val faded = calm.copy(alpha = 0.45f)

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.80f),
        shape = MaterialTheme.shapes.small,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Canvas(Modifier.size(ARC_SIZE)) {
                val centre = Offset(size.width / 2f, size.height / 2f)
                val full = size.minDimension / 2f
                drawCircle(calm.copy(alpha = 0.25f), radius = full, center = centre, style = Stroke(width = 2f))
                drawCircle(calm.copy(alpha = 0.6f), radius = 3f, center = centre)
                ring.samples.forEach { sample ->
                    val reach = full * arcRadiusFraction(sample.metres, ring.maxMetres)
                    val near = sampleIsClose(sample.metres, ring.floorMetres)
                    drawArc(
                        color = when (arcTone(near, ring.stale)) {
                            ArcTone.Alarm -> alarm
                            ArcTone.Calm -> calm
                            ArcTone.Stale -> faded
                        },
                        startAngle = (sample.bearingDegrees - 90.0).toFloat() - arcSweep(ring.incrementDegrees) / 2f,
                        sweepAngle = arcSweep(ring.incrementDegrees),
                        useCenter = false,
                        topLeft = Offset(centre.x - reach, centre.y - reach),
                        size = Size(reach * 2f, reach * 2f),
                        style = Stroke(width = if (near) 10f else 6f),
                    )
                }
            }
            if (ring.stale) {
                Text(
                    text = "Last seen",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
