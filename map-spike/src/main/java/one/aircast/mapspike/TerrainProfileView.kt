package one.aircast.mapspike

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.unit.dp

private val TERRAIN_COLOUR = Color(0xFF8D6E63)
private val PLANNED_COLOUR = Color(0xFF4FC3F7)
private const val FULL_TERRAIN = 0.98

internal fun profileOffsets(
    profile: TerrainProfile,
    width: Float,
    height: Float,
    value: (ProfilePoint) -> Double?,
): List<Offset> {
    if (!profile.drawable || width <= 0f || height <= 0f) {
        return emptyList()
    }
    val span = profile.span
    val distance = profile.distance.takeIf { it > 0.0 } ?: return emptyList()

    return profile.points.mapNotNull { point ->
        val height1 = value(point) ?: return@mapNotNull null
        val x = (point.distance / distance * width).toFloat()
        val y = height - ((height1 - profile.lowest) / span * height).toFloat()
        Offset(x, y)
    }
}

internal fun groundOutline(terrain: List<Offset>, height: Float): List<Offset> =
    if (terrain.size < 2) {
        emptyList()
    } else {
        terrain + Offset(terrain.last().x, height) + Offset(terrain.first().x, height)
    }

private fun pathOf(points: List<Offset>): Path = Path().apply {
    points.forEachIndexed { index, offset ->
        if (index == 0) moveTo(offset.x, offset.y) else lineTo(offset.x, offset.y)
    }
}

internal fun profileLabel(profile: TerrainProfile): String =
    (if (profile.flat) {
        "${profile.lowest.toInt()} m AMSL"
    } else {
        "${profile.lowest.toInt()}\u2013${profile.highest.toInt()} m AMSL"
    }) +
        " \u00b7 ${(profile.distance / 1000).format2()} km" +
        when {
            !profile.hasTerrain -> " \u00b7 ground height unknown"
            profile.terrainCoverage < FULL_TERRAIN ->
                " \u00b7 ground height for ${(profile.terrainCoverage * 100).toInt().coerceAtLeast(1)}% " +
                    "of the route"
            else -> ""
        }

@Composable
fun TerrainProfileView(profile: TerrainProfile, modifier: Modifier = Modifier) {
    if (profile.points.isEmpty()) {
        return
    }
    if (!profile.drawable) {
        Text(
            "Plan a route with altitudes to see a profile.",
            modifier.fillMaxWidth().padding(vertical = 4.dp),
            style = MaterialTheme.typography.labelSmall,
        )
        return
    }

    if (profile.flat) {
        Text(
            profileLabel(profile),
            modifier.fillMaxWidth().padding(vertical = 4.dp),
            style = MaterialTheme.typography.labelSmall,
        )
        return
    }

    Surface(
        modifier.fillMaxWidth().height(110.dp),
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
    ) {
        Box {
            Canvas(Modifier.fillMaxWidth().height(110.dp).padding(8.dp)) {
                val terrain = profileOffsets(profile, size.width, size.height) { it.terrain }
                val planned = profileOffsets(profile, size.width, size.height) { it.planned }

                if (terrain.size >= 2) {
                    val ground = pathOf(groundOutline(terrain, size.height)).apply { close() }
                    drawPath(ground, TERRAIN_COLOUR.copy(alpha = 0.45f))
                    drawPath(pathOf(terrain), TERRAIN_COLOUR, style = androidx.compose.ui.graphics.drawscope.Stroke(3f))
                }
                if (planned.size >= 2) {
                    drawPath(pathOf(planned), PLANNED_COLOUR, style = androidx.compose.ui.graphics.drawscope.Stroke(3f))
                }
            }

            Text(
                profileLabel(profile),
                style = MaterialTheme.typography.labelSmall,
                modifier = Modifier.align(Alignment.TopStart).padding(8.dp),
            )
        }
    }
}

private fun Double.format2(): String = "%.2f".format(this)
