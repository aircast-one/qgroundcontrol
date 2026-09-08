package one.aircast.android.ui

import java.util.Locale
import kotlin.math.roundToInt

data class Obstacle(val metres: Double, val bearingDeg: Double)

private val SECTORS = listOf(
    "ahead", "ahead right", "right", "behind right",
    "behind", "behind left", "left", "ahead left",
)

internal fun bearingSector(bearingDeg: Double): String {
    val wrapped = ((bearingDeg % 360) + 360) % 360
    return SECTORS[(((wrapped + 22.5) % 360) / 45).toInt()]
}

internal fun nearestObstacle(
    distancesCm: List<Int>,
    incrementDeg: Double,
    angleOffsetDeg: Double,
    minCm: Int,
    maxCm: Int,
): Obstacle? {
    if (incrementDeg <= 0.0 || minCm <= 0 || maxCm <= minCm) {
        return null
    }
    return distancesCm.withIndex()
        .filter { (_, cm) -> cm in minCm..maxCm }
        .minByOrNull { (_, cm) -> cm }
        ?.let { (index, cm) ->
            Obstacle(
                metres = cm / 100.0,
                bearingDeg = ((angleOffsetDeg + index * incrementDeg) % 360 + 360) % 360,
            )
        }
}

internal fun obstacleLabel(obstacle: Obstacle?): String? = obstacle?.let {
    String.format(Locale.US, "%.1f m %s", it.metres, bearingSector(it.bearingDeg))
}

internal fun obstacleIsClose(obstacle: Obstacle?, minCm: Int): Boolean =
    obstacle != null && (obstacle.metres * 100).roundToInt() <= minCm * 2
