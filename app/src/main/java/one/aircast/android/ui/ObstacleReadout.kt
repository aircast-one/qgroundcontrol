package one.aircast.android.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcPath
import org.json.JSONArray

private const val AVOIDANCE = "vehicle.objectAvoidance"

private fun distancesFrom(json: org.json.JSONObject?): List<Int> {
    val raw = json?.opt("value") as? JSONArray ?: return emptyList()
    return (0 until raw.length()).map { raw.optInt(it, 65535) }
}

@Composable
fun ObstacleReadout(modifier: Modifier = Modifier) {
    val available by qgcBool("$AVOIDANCE.available")
    val distancesJson by qgcPath("$AVOIDANCE.distances")
    val increment by qgcDouble("$AVOIDANCE.increment")
    val angleOffset by qgcDouble("$AVOIDANCE.angleOffset")
    val minDistance by qgcDouble("$AVOIDANCE.minDistance")
    val maxDistance by qgcDouble("$AVOIDANCE.maxDistance")

    if (!available) {
        return
    }

    val nearest = nearestObstacle(
        distancesCm = distancesFrom(distancesJson),
        incrementDeg = increment,
        angleOffsetDeg = angleOffset,
        minCm = minDistance.toInt(),
        maxCm = maxDistance.toInt(),
    )
    val label = obstacleLabel(nearest) ?: return

    Surface(
        modifier = modifier,
        color = if (obstacleIsClose(nearest, minDistance.toInt())) {
            MaterialTheme.colorScheme.errorContainer
        } else {
            MaterialTheme.colorScheme.surfaceVariant
        },
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
        )
    }
}
