package one.aircast.mapspike

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

private val TERRAIN_COLOUR = Color(0xFF8D6E63)
private val PLANNED_COLOUR = Color(0xFF4FC3F7)
private val COLLISION_COLOUR = Color.Red
private val MISSING_COLOUR = Color.Yellow
private val PATTERN_COLOUR = Color.Green.copy(alpha = 0.5f)
private const val MARKER_TAP_SLOP_PX = 48f
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

internal fun terrainRuns(profile: TerrainProfile, width: Float, height: Float): List<List<Offset>> {
    val distance = profile.distance.takeIf { it > 0.0 && profile.drawable } ?: return emptyList()
    return profile.points
        .fold(listOf(emptyList<Offset>())) { runs, point ->
            val ground = point.terrain
            if (ground == null) {
                if (runs.last().isEmpty()) runs else runs + listOf(emptyList())
            } else {
                val at = Offset((point.distance / distance * width).toFloat(), height - ((ground - profile.lowest) / profile.span * height).toFloat())
                runs.dropLast(1) + listOf(runs.last() + at)
            }
        }
        .filter { it.size >= 2 }
}

internal fun missingSpans(profile: TerrainProfile, width: Float): List<Pair<Float, Float>> {
    val distance = profile.distance.takeIf { it > 0.0 } ?: return emptyList()
    return profile.points.zipWithNext()
        .filter { (from, to) -> from.terrain == null || to.terrain == null }
        .map { (from, to) -> (from.distance / distance * width).toFloat() to (to.distance / distance * width).toFloat() }
}

internal fun collisionSegments(profile: TerrainProfile, planned: List<Offset>): List<Pair<Offset, Offset>> =
    profile.points.zip(planned).zipWithNext()
        .filter { (from, to) -> from.first.collision && to.first.collision }
        .map { (from, to) -> from.second to to.second }

internal fun markerX(distance: Double, profile: TerrainProfile, width: Float): Float =
    profile.distance.takeIf { it > 0.0 }?.let { (distance / it * width).toFloat() } ?: 0f

internal fun tappedSequence(profile: TerrainProfile, width: Float, tapX: Float): Int? =
    profile.markers
        .flatMap { marker -> listOfNotNull(marker.distance, marker.endDistance).map { markerX(it, profile, width) to marker.sequence } }
        .minByOrNull { (x, _) -> kotlin.math.abs(x - tapX) }
        ?.takeIf { (x, _) -> kotlin.math.abs(x - tapX) <= MARKER_TAP_SLOP_PX }
        ?.second
        ?: profile.markers.firstOrNull { marker ->
            marker.endDistance?.let { end -> tapX in markerX(marker.distance, profile, width)..markerX(end, profile, width) } == true
        }?.sequence

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

internal fun heightRange(profile: TerrainProfile): String =
    if (profile.flat) "${profile.lowestText} AMSL" else "${profile.bandText} AMSL"

internal fun profileLabel(profile: TerrainProfile): String =
    terrainWarning(profile.clearance)?.let { "$it " } .orEmpty() +
    heightRange(profile) +
        " \u00b7 ${profile.distanceText}" +
        when {
            !profile.hasTerrain -> " \u00b7 ground height unknown"
            profile.terrainCoverage <= 0.0 -> " \u00b7 ground height at points, none along the route"
            profile.terrainCoverage < FULL_TERRAIN ->
                " \u00b7 ground height for ${(profile.terrainCoverage * 100).toInt().coerceAtLeast(1)}% " +
                    "of the route"
            else -> ""
        }

internal const val ELEVATION_PROVIDER = "settings.flightMapSettings.elevationMapProvider.rawValue"
internal const val SHOW_MISSION_ITEM_STATUS = "settings.planViewSettings.showMissionItemStatus"

internal fun missionItemStatusShown(setting: org.json.JSONObject?): Boolean =
    setting?.takeIf { it.has("value") && !it.isNull("value") }?.optBoolean("value", true) ?: true

internal fun elevationCredit(notice: String): String? = notice.takeIf { it.isNotBlank() }?.let { "Powered by $it" }

@Composable
fun TerrainProfileView(
    profile: TerrainProfile,
    notice: String,
    modifier: Modifier = Modifier,
    selectedSequence: Int? = null,
    onSelect: (Int) -> Unit = {},
) {
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

    Text(profileLabel(profile), Modifier.fillMaxWidth().padding(horizontal = 8.dp), style = MaterialTheme.typography.labelSmall)
    if (profile.heightHeader.isNotBlank()) {
        androidx.compose.foundation.layout.Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
            Text("Elevation", Modifier.weight(1f), style = MaterialTheme.typography.labelSmall)
            Text(profile.heightHeader, style = MaterialTheme.typography.labelSmall)
        }
    }
    elevationCredit(notice)?.let {
        Text(it, Modifier.fillMaxWidth(), style = MaterialTheme.typography.labelSmall, textAlign = TextAlign.Center)
    }
    Surface(
        modifier.fillMaxWidth().height(126.dp),
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
    ) {
        Box {
            val measurer = rememberTextMeasurer()
            val labelStyle = MaterialTheme.typography.labelSmall
            val labelMargin = with(LocalDensity.current) { (profile.heightTicks.maxOfOrNull { measurer.measure(it, labelStyle).size.width } ?: 0).toDp() } + 8.dp
            val ink = MaterialTheme.colorScheme.onSurface
            val accent = MaterialTheme.colorScheme.primary
            val onAccent = MaterialTheme.colorScheme.onPrimary
            val paper = MaterialTheme.colorScheme.surface
            Canvas(
                Modifier.fillMaxWidth().height(110.dp).padding(start = labelMargin, end = 8.dp, top = 8.dp, bottom = 16.dp).pointerInput(profile) {
                    detectTapGestures { tap -> tappedSequence(profile, size.width.toFloat(), tap.x)?.let(onSelect) }
                },
            ) {
                val planned = profileOffsets(profile, size.width, size.height) { it.planned }

                terrainRuns(profile, size.width, size.height).forEach { run ->
                    drawPath(pathOf(groundOutline(run, size.height)).apply { close() }, TERRAIN_COLOUR.copy(alpha = 0.45f))
                    drawPath(pathOf(run), TERRAIN_COLOUR, style = Stroke(3f))
                }
                missingSpans(profile, size.width).forEach { (from, to) ->
                    drawLine(MISSING_COLOUR, Offset(from, size.height), Offset(to, size.height), strokeWidth = 9f)
                }
                profile.heightTicks.forEachIndexed { index, tick ->
                    val y = size.height - size.height * index / (profile.heightTicks.size - 1).coerceAtLeast(1)
                    drawLine(ink.copy(alpha = 0.3f), Offset(0f, y), Offset(size.width, y), strokeWidth = 1f)
                    val measured = measurer.measure(tick, labelStyle)
                    drawText(measured, ink, Offset(-measured.size.width - 4f, y - measured.size.height / 2f))
                }
                profile.distanceTicks.forEachIndexed { index, tick ->
                    val measured = measurer.measure(tick, labelStyle)
                    val x = size.width * index / (profile.distanceTicks.size - 1).coerceAtLeast(1)
                    drawText(measured, ink, Offset((x - measured.size.width / 2f).coerceIn(0f, (size.width - measured.size.width).coerceAtLeast(0f)), size.height))
                }
                if (planned.size >= 2) {
                    drawPath(pathOf(planned), PLANNED_COLOUR, style = Stroke(3f))
                }
                collisionSegments(profile, planned).forEach { (from, to) -> drawLine(COLLISION_COLOUR, from, to, strokeWidth = 9f) }

                profile.markers.forEach { marker ->
                    val start = markerX(marker.distance, profile, size.width)
                    val labels = listOfNotNull(start to marker.label, marker.endDistance?.let { markerX(it, profile, size.width) to (marker.lastSequence ?: marker.sequence).toString() })
                    marker.endDistance?.let { end ->
                        val band = measurer.measure(marker.pattern, labelStyle)
                        val right = markerX(end, profile, size.width)
                        drawRect(PATTERN_COLOUR, Offset(start, size.height - band.size.height), Size(right - start, band.size.height.toFloat()))
                        drawText(band, ink, Offset((start + right - band.size.width) / 2f, size.height - band.size.height))
                    }
                    labels.forEach { (x, text) ->
                        drawLine(ink, Offset(x, 0f), Offset(x, size.height), strokeWidth = 1f)
                        val measured = measurer.measure(text, labelStyle)
                        val radius = maxOf(measured.size.width, measured.size.height) / 2f + 3f
                        val centre = Offset(x, size.height - radius)
                        val chosen = marker.sequence == selectedSequence
                        drawCircle(if (chosen) accent else ink.copy(alpha = 0.8f), radius, centre)
                        drawText(measured, if (chosen) onAccent else paper, centre - Offset(measured.size.width / 2f, measured.size.height / 2f))
                    }
                }
            }

        }
    }
}

