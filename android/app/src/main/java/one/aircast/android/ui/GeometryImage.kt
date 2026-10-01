package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.sp
import org.json.JSONObject
import kotlin.math.hypot
import kotlin.math.min

private const val AXIS_INDICATOR_SIZE = 15f
private const val IMAGE_MARGIN = 5f
private const val FRAME_WIDTH = 6f
private const val COAX_DISTANCE = 0.03
private const val SPIN_ARC_DEGREES = 50f
private const val IMAGE_ASPECT = 2f
private val CLOCKWISE = Color(21, 158, 31, 200)
private val COUNTER_CLOCKWISE = Color(78, 195, 232, 200)
private val FRAME_ARROW = Color(255, 68, 43, 200)
private val FRAME = Color(150, 150, 150)

internal data class GeometryMotor(val index: Int, val label: Int, val x: Double, val y: Double, val counterClockwise: Boolean)

internal data class DrawnMotor(val motor: GeometryMotor, val center: Offset, val textCenter: Offset, val coax: Boolean)

internal data class GeometryLayout(
    val origin: Offset,
    val rotorDiameter: Float,
    val fontSize: Float,
    val extraYMargin: Float,
    val motors: List<DrawnMotor>,
)

internal fun geometryMotors(geometry: JSONObject?): List<GeometryMotor> {
    val motors = geometry?.optJSONArray("motors") ?: return emptyList()
    return (0 until motors.length()).mapNotNull { index ->
        motors.optJSONObject(index)?.let { GeometryMotor(it.optInt("index"), it.optInt("label"), it.optDouble("x"), it.optDouble("y"), it.optBoolean("counterClockwise")) }
    }
}

internal fun geometryLayout(motors: List<GeometryMotor>, width: Float, height: Float): GeometryLayout? {
    if (motors.size <= 1) return null
    val (minX, maxX) = motors.minOf { it.x } to motors.maxOf { it.x }
    val (minY, maxY) = motors.minOf { it.y } to motors.maxOf { it.y }
    if (maxX - minX < 0.0001 || maxY - minY < 0.0001) return null
    val coax = motors.mapIndexed { at, motor -> motors.take(at).any { before -> hypot(before.x - motor.x, before.y - motor.y) < COAX_DISTANCE } }
    val squeezed = width < height + AXIS_INDICATOR_SIZE * 4f
    val usableWidth = width - 2f * IMAGE_MARGIN - if (squeezed) AXIS_INDICATOR_SIZE else 0f
    val usableHeight = height - 2f * IMAGE_MARGIN - if (squeezed) AXIS_INDICATOR_SIZE else 0f
    val extraOffsetX = if (squeezed) AXIS_INDICATOR_SIZE else 0f
    val rotorDiameter = min(usableWidth, usableHeight) * if (motors.size <= 6) 0.31f else 0.25f
    val fontSize = rotorDiameter * 0.4f
    val extraYMargin = if (coax.any { it }) fontSize * 1.1f else 0f
    val scaleX = (usableWidth - rotorDiameter) / (maxY - minY).toFloat()
    val scaleY = (usableHeight - extraYMargin - rotorDiameter) / (maxX - minX).toFloat()
    val scale = min(scaleX, scaleY)
    val offsetX = IMAGE_MARGIN + extraOffsetX + usableWidth / 2f - ((maxY + minY) / 2.0).toFloat() * scale
    val offsetY = IMAGE_MARGIN + (usableHeight - extraYMargin) / 2f + ((maxX + minX) / 2.0).toFloat() * scale
    val drawn = motors.zip(coax) { motor, isCoax ->
        val base = Offset(offsetX + motor.y.toFloat() * scale, offsetY - motor.x.toFloat() * scale)
        val center = if (isCoax) base.copy(y = base.y + extraYMargin) else base
        val textCenter = if (isCoax) Offset(center.x, center.y + rotorDiameter / 2f - extraYMargin / 2f) else center
        DrawnMotor(motor, center, textCenter, isCoax)
    }
    return GeometryLayout(Offset(offsetX, offsetY), rotorDiameter, fontSize, extraYMargin, drawn)
}

@Composable
internal fun GeometryImage(motors: List<GeometryMotor>, modifier: Modifier = Modifier) {
    val measurer = rememberTextMeasurer()
    val ink = MaterialTheme.colorScheme.onSurface
    Canvas(modifier.fillMaxWidth().aspectRatio(IMAGE_ASPECT)) {
        val layout = geometryLayout(motors, size.width, size.height) ?: return@Canvas
        layout.motors.filter { !it.coax }.forEach { drawLine(FRAME, layout.origin, it.center, strokeWidth = FRAME_WIDTH) }
        val centerSize = layout.rotorDiameter * 0.8f
        drawRoundRect(FRAME, Offset(layout.origin.x - centerSize / 2f, layout.origin.y - centerSize / 2f), Size(centerSize, centerSize), CornerRadius(FRAME_WIDTH))
        val arrowWidth = layout.rotorDiameter / 4f
        val arrowHeight = layout.rotorDiameter / 2f
        drawPath(
            Path().apply {
                moveTo(layout.origin.x - arrowWidth / 2f, layout.origin.y + arrowHeight / 2f)
                lineTo(layout.origin.x, layout.origin.y - arrowHeight / 2f)
                lineTo(layout.origin.x + arrowWidth / 2f, layout.origin.y + arrowHeight / 2f)
                close()
            },
            FRAME_ARROW,
        )
        drawAxisIndicator(Offset(AXIS_INDICATOR_SIZE / 2f, size.height - AXIS_INDICATOR_SIZE / 2f), ink, measurer)
        (layout.motors.filter { it.coax } + layout.motors.filter { !it.coax }).forEach { drawMotor(it, layout, ink, measurer) }
    }
}

private fun DrawScope.drawMotor(drawn: DrawnMotor, layout: GeometryLayout, ink: Color, measurer: TextMeasurer) {
    val fill = if (drawn.motor.counterClockwise) COUNTER_CLOCKWISE else CLOCKWISE
    val arrowColor = fill.copy(alpha = 1f)
    val radius = layout.rotorDiameter / 2f
    drawCircle(fill, radius, drawn.center)
    val label = measurer.measure((drawn.motor.label).toString(), TextStyle(color = ink, fontSize = (layout.fontSize / density).sp))
    drawText(label, topLeft = drawn.textCenter - Offset(label.size.width / 2f, label.size.height / 2f))
    val offsets = if (drawn.coax) listOf(30f, 150f) else listOf(0f, 180f)
    offsets.forEach { offset ->
        val turn = if (drawn.motor.counterClockwise) -SPIN_ARC_DEGREES / 2f + offset else SPIN_ARC_DEGREES / 2f + offset
        val ySign = if (drawn.motor.counterClockwise) 1f else -1f
        rotate(turn, drawn.center) {
            drawArc(arrowColor, 0f, ySign * SPIN_ARC_DEGREES, false, Offset(drawn.center.x - radius, drawn.center.y - radius), Size(radius * 2f, radius * 2f), style = Stroke(2.5f))
            val head = FRAME_WIDTH * 1.25f
            drawPath(
                Path().apply {
                    moveTo(drawn.center.x + radius - FRAME_WIDTH / 2f, drawn.center.y + ySign * head / 2f)
                    lineTo(drawn.center.x + radius, drawn.center.y - ySign * head / 2f)
                    lineTo(drawn.center.x + radius + FRAME_WIDTH / 2f, drawn.center.y + ySign * head / 2f)
                    close()
                },
                arrowColor,
            )
        }
    }
}

private fun DrawScope.drawAxisIndicator(origin: Offset, ink: Color, measurer: TextMeasurer) {
    val length = AXIS_INDICATOR_SIZE * 2f
    val up = Offset(origin.x, origin.y - length)
    val right = Offset(origin.x + length, origin.y)
    drawLine(ink, origin, up, strokeWidth = 1.5f)
    drawLine(ink, origin, right, strokeWidth = 1.5f)
    val style = TextStyle(color = ink, fontSize = (AXIS_INDICATOR_SIZE / density).sp)
    val x = measurer.measure("x", style)
    drawText(x, topLeft = Offset(up.x - x.size.width / 2f, up.y - x.size.height))
    val y = measurer.measure(" y", style)
    drawText(y, topLeft = Offset(right.x, right.y - y.size.height / 2f))
}

internal fun motorAt(layout: GeometryLayout, point: Offset, highlighted: Set<Int>): Int? {
    val hits = layout.motors.filter { it.motor.index in highlighted }.filter { drawn ->
        val radius = if (drawn.coax) layout.fontSize / 2f else layout.rotorDiameter / 2f
        (drawn.textCenter - point).getDistance() < radius
    }
    return hits.singleOrNull()?.motor?.index
}
