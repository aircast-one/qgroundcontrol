package one.aircast.map

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.cos
import kotlin.math.sin

const val OBSTACLE_VIEW = "view.obstacle"
private const val SEGMENTS = 16
private const val LEVEL_METRES = 10.0
private const val SEGMENT_GAP_RAD = 0.03
private const val TEXT_STEP_METRES = 2.0
private val MAP_TEXT = 22.dp
private val VIDEO_TEXT = 10.dp
val TRUE_SCALE_PROBE = 100.dp
const val TRUE_SCALE_BELOW_METRES = 4.0

data class ObstacleOverlay(
    val ranges: List<Double>,
    val texts: List<String>,
    val increment: Double,
    val offset: Double,
    val maxMetres: Double,
)

fun obstacleOverlay(view: JSONObject?): ObstacleOverlay? {
    val drawn = view?.optJSONObject("overlay") ?: return null
    val ranges = drawn.optJSONArray("ranges") ?: return null
    val texts = drawn.optJSONArray("texts")
    val increment = view.optDouble("ringIncrement", Double.NaN).takeIf { it.isFinite() && it > 0.0 } ?: return null
    val maxMetres = drawn.optDouble("maxMetres", Double.NaN).takeIf { it.isFinite() && it > 0.0 } ?: return null
    return ObstacleOverlay(
        ranges = (0 until ranges.length()).map { ranges.optDouble(it, Double.NaN) },
        texts = (0 until ranges.length()).map { texts?.optString(it).orEmpty() },
        increment = increment,
        offset = view.optDouble("ringOffset", 0.0).takeIf { it.isFinite() } ?: 0.0,
        maxMetres = maxMetres,
    ).takeIf { it.ranges.isNotEmpty() }
}

fun rangeIndex(degrees: Double, overlay: ObstacleOverlay, heading: Double): Int {
    val len = overlay.ranges.size
    val wrapped = (degrees - overlay.offset - heading).mod(360.0)
    return (len + ceil(wrapped / overlay.increment).toInt()).mod(len)
}

data class GradientStop(val at: Float, val colour: Color)

private val MAP_STOPS = listOf(
    GradientStop(0f, Color(1f, 0f, 0f, 1f)),
    GradientStop(0.1f, Color(1f, 0f, 0f, 0.7f)),
    GradientStop(0.5f, Color(1f, 0.64f, 0f, 0.7f)),
    GradientStop(0.65f, Color(1f, 0.64f, 0f, 0.3f)),
    GradientStop(0.95f, Color(0f, 1f, 0f, 0.3f)),
    GradientStop(1f, Color(0f, 1f, 0f, 0f)),
)

private val VIDEO_STOPS = listOf(
    GradientStop(0f, Color(1f, 0f, 0f, 0.9f)),
    GradientStop(0.1f, Color(1f, 0f, 0f, 0.3f)),
    GradientStop(0.5f, Color(1f, 0.64f, 0f, 0.3f)),
    GradientStop(0.65f, Color(1f, 0.64f, 0f, 0.2f)),
    GradientStop(0.95f, Color(0f, 1f, 0f, 0.1f)),
    GradientStop(1f, Color(0f, 1f, 0f, 0f)),
)

fun twoCircleStops(from: Float, to: Float, stops: List<GradientStop>): List<GradientStop> =
    stops.map { GradientStop((from + it.at * (to - from)) / to, it.colour) }

private fun radial(centre: Offset, from: Float, to: Float, stops: List<GradientStop>): Brush =
    Brush.radialGradient(
        *twoCircleStops(from, to, stops).map { it.at to it.colour }.toTypedArray(),
        center = centre,
        radius = to,
    )

data class OverlayPoint(val outer: Offset, val inner: Offset, val range: Double, val index: Int)

data class MapOverlayShape(val gradientFrom: Float, val gradientTo: Float, val points: List<OverlayPoint>)

fun mapOverlayShape(
    overlay: ObstacleOverlay,
    centre: Offset,
    heightPx: Float,
    metresInProbe: Double,
    probePx: Float,
    heading: Double,
    mapBearing: Double,
): MapOverlayShape {
    val maxRadius = 0.9f * heightPx / 2f
    val minRadius = maxRadius * 0.2f
    val pixelsPerMetre = probePx / metresInProbe
    val trueScale = metresInProbe < TRUE_SCALE_BELOW_METRES
    val gradientFrom = if (trueScale) 0f else minRadius
    val gradientTo = if (trueScale) (overlay.maxMetres * pixelsPerMetre).toFloat() else maxRadius
    val metresToPixels = if (trueScale) pixelsPerMetre else (maxRadius - minRadius) / overlay.maxMetres
    val height = minRadius / 8f
    val points = overlay.ranges.indices.map { i ->
        val degrees = i * overlay.increment
        val rad = (degrees - mapBearing) * PI / 180.0
        val index = rangeIndex(degrees, overlay, heading)
        val metres = overlay.ranges[index]
        val pixels = gradientFrom + metres * metresToPixels
        OverlayPoint(
            outer = Offset((centre.x + pixels * sin(rad)).toFloat(), (centre.y - pixels * cos(rad)).toFloat()),
            inner = Offset((centre.x + (pixels - height) * sin(rad)).toFloat(), (centre.y - (pixels - height) * cos(rad)).toFloat()),
            range = metres,
            index = index,
        )
    }
    return MapOverlayShape(gradientFrom, gradientTo, points)
}

fun mapOverlayLabels(overlay: ObstacleOverlay, points: List<OverlayPoint>): List<OverlayPoint> =
    points.indices.filter { it % 3 == 0 }
        .map { i -> points[(i + 1) % points.size].takeIf { it.range < points[i].range } ?: points[i] }
        .fold(emptyList<OverlayPoint>() to -1.0) { (shown, previous), point ->
            if (point.range < overlay.maxMetres && abs(point.range - previous) > TEXT_STEP_METRES) (shown + point) to point.range else shown to previous
        }.first

data class VideoSegment(val from: Float, val to: Float, val radFrom: Double, val radTo: Double, val label: String?)

fun videoOverlaySegments(overlay: ObstacleOverlay, heightPx: Float, showText: Boolean): Pair<Pair<Float, Float>, List<VideoSegment>>? {
    val maxRadius = 0.9f * heightPx / 2f
    val segmentHeight = maxRadius * 0.2f / 8f
    val levels = overlay.maxMetres / LEVEL_METRES
    val gradientFrom = (maxRadius - segmentHeight * levels * 2).toFloat()
    if (maxRadius <= 0f || gradientFrom < 0f) return null
    val len = overlay.ranges.size
    val step = 360.0 / SEGMENTS
    val segments = (0 until SEGMENTS).flatMap { s ->
        val degrees = s * step
        val first = rangeIndex(degrees, overlay, 0.0)
        val next = rangeIndex(degrees + step, overlay, 0.0)
        val end = if (first < next) next else len + next
        val nearest = (first until end).map { it % len }.filter { overlay.ranges[it] < overlay.maxMetres }.minByOrNull { overlay.ranges[it] }
        val rangeMin = nearest?.let { overlay.ranges[it] } ?: overlay.maxMetres
        val radFrom = degrees * PI / 180.0
        val radTo = radFrom + step * PI / 180.0 - SEGMENT_GAP_RAD
        val drawn = (0 until ceil(levels).toInt()).map { ii ->
            val from = maxRadius - ii * segmentHeight * 2
            val rangeInLevel = overlay.maxMetres - (ii + 1) * LEVEL_METRES
            val reached = rangeMin > rangeInLevel || ii >= levels - 1
            val range = if (reached) rangeMin else overlay.maxMetres
            VideoSegment(from, from - segmentHeight, radFrom, radTo, nearest?.takeIf { showText && reached && range < overlay.maxMetres }?.let { overlay.texts[it] }) to (range == rangeMin)
        }
        val upTo = drawn.indexOfFirst { it.second }.let { if (it < 0) drawn.size else it + 1 }
        drawn.take(upTo).map { it.first }
    }
    val upToPositive = segments.indexOfFirst { it.to < 0f }.let { if (it < 0) segments.size else it }
    return (gradientFrom to maxRadius) to segments.take(upToPositive)
}

private fun DrawScope.outlinedText(measurer: TextMeasurer, text: String, at: Offset, textSize: Dp, fill: Color, strokeWidth: Float) {
    val style = TextStyle(fontSize = textSize.toSp(), fontWeight = FontWeight.Bold)
    val laid = measurer.measure(text, style)
    val topLeft = Offset(at.x, at.y - laid.firstBaseline)
    drawText(laid, color = Color(0f, 0f, 0f, 0.8f), topLeft = topLeft, drawStyle = Stroke(strokeWidth))
    drawText(laid, color = fill, topLeft = topLeft)
}

fun DrawScope.drawMapObstacleOverlay(measurer: TextMeasurer, overlay: ObstacleOverlay, shape: MapOverlayShape, centre: Offset, showText: Boolean) {
    val points = shape.points
    if (points.isEmpty() || shape.gradientTo <= 0f) return
    val brush = radial(centre, shape.gradientFrom, shape.gradientTo, MAP_STOPS)
    points.indices.filter { it % 3 == 0 }.forEach { i ->
        val (a, b, c, d) = listOf(i, (i + 1) % points.size, (i + 2) % points.size, (i + 3) % points.size).map { points[it] }
        val wedge = Path().apply {
            moveTo(a.inner.x, a.inner.y)
            lineTo(a.outer.x, a.outer.y)
            cubicTo(b.outer.x, b.outer.y, c.outer.x, c.outer.y, d.outer.x, d.outer.y)
            lineTo(d.inner.x, d.inner.y)
            cubicTo(d.inner.x, d.inner.y, c.inner.x, c.inner.y, b.inner.x, b.inner.y)
            close()
        }
        drawPath(wedge, brush)
    }
    if (showText) {
        mapOverlayLabels(overlay, points).forEach { outlinedText(measurer, overlay.texts[it.index], it.inner, MAP_TEXT, Color(1f, 1f, 1f, 0.9f), 2f) }
    }
}

fun DrawScope.drawVideoObstacleOverlay(measurer: TextMeasurer, overlay: ObstacleOverlay, showText: Boolean) {
    val (radii, segments) = videoOverlaySegments(overlay, size.height, showText) ?: return
    val centre = Offset(size.width / 2f, size.height / 2f)
    val brush = radial(centre, radii.first, radii.second, VIDEO_STOPS)
    segments.forEach { segment ->
        val radFrom = segment.radFrom - PI / 2
        val radTo = segment.radTo - PI / 2
        val top = Offset((centre.x + segment.from * cos(radFrom)).toFloat(), (centre.y + segment.from * sin(radFrom)).toFloat())
        val topEnd = Offset((centre.x + segment.from * cos(radTo)).toFloat(), (centre.y + segment.from * sin(radTo)).toFloat())
        val sweep = ((radTo - radFrom) * 180.0 / PI).toFloat()
        val ring = Path().apply {
            moveTo(top.x, top.y)
            arcTo(Rect(centre, segment.from), (radFrom * 180.0 / PI).toFloat(), sweep, false)
            arcTo(Rect(centre, segment.to), (radTo * 180.0 / PI).toFloat(), -sweep, false)
            lineTo(top.x, top.y)
            close()
        }
        withTransform({ scale(2f, 1f, pivot = Offset(centre.x, 0f)) }) { drawPath(ring, brush) }
        segment.label?.let { label ->
            val middle = Offset((top.x + topEnd.x) / 2f, (top.y + topEnd.y) / 2f)
            outlinedText(measurer, label, Offset(2f * middle.x - centre.x, middle.y), VIDEO_TEXT * 2, Color(1f, 1f, 1f, 0.8f), 4f)
        }
    }
}

@Composable
fun ObstacleVideoOverlay(modifier: Modifier, showText: Boolean) {
    val json by mapPath(OBSTACLE_VIEW)
    val measurer = rememberTextMeasurer()
    val overlay = remember(json) { obstacleOverlay(json) } ?: return
    Canvas(modifier) { drawVideoObstacleOverlay(measurer, overlay, showText) }
}
