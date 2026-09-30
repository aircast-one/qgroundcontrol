package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject
import kotlin.math.cos
import kotlin.math.sin

internal const val ATTITUDE_PATH = "view.attitude"
private val INSTRUMENT_SIZE = 132.dp
private const val HORIZON_FRACTION = 0.72f
private const val PITCH_SPAN_DEGREES = 45f
private const val LADDER_STEP_DEGREES = 5
private const val HOME_LETTER = "L"
private val CARDINALS = listOf(0 to "N", 90 to "E", 180 to "S", 270 to "W")

internal data class Attitude(
    val roll: Float,
    val pitch: Float,
    val heading: Float,
    val headingText: String,
    val courseOverGround: Float?,
    val headingToHome: Float?,
    val headingToNextWaypoint: Float?,
    val noseUp: Boolean,
)

private fun JSONObject.optAngle(key: String): Float? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }?.toFloat()

internal fun attitude(view: JSONObject?): Attitude? =
    view?.takeIf { it.optBoolean("available") }?.let {
        Attitude(
            roll = it.optAngle("roll") ?: 0f,
            pitch = it.optAngle("pitch") ?: 0f,
            heading = it.optAngle("heading") ?: 0f,
            headingText = it.optText("headingText"),
            courseOverGround = it.optAngle("courseOverGround"),
            headingToHome = it.optAngle("headingToHome"),
            headingToNextWaypoint = it.optAngle("headingToNextWaypoint"),
            noseUp = it.optBoolean("noseUp"),
        )
    }

internal fun pitchOffset(pitch: Float, horizonRadius: Float): Float =
    pitch * (horizonRadius * 2f) / PITCH_SPAN_DEGREES

internal fun ladderAngles(): List<Int> =
    (-90..90 step LADDER_STEP_DEGREES).filter { it != 0 }

private fun pointOnRing(center: Offset, radius: Float, degrees: Float): Offset {
    val radians = Math.toRadians(degrees.toDouble())
    return Offset(center.x + radius * sin(radians).toFloat(), center.y - radius * cos(radians).toFloat())
}

@Composable
fun AttitudeInstrument(modifier: Modifier = Modifier) {
    val view by qgcPath(ATTITUDE_PATH)
    val reading = remember(view) { attitude(view) } ?: return
    val measurer = rememberTextMeasurer()
    val sky = MaterialTheme.aircast.mapWater
    val ground = MaterialTheme.aircast.mapLand
    val ink = MaterialTheme.colorScheme.onSurface
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val accent = MaterialTheme.colorScheme.primary
    val mission = MaterialTheme.aircast.mission
    val home = MaterialTheme.aircast.success
    val label = TextStyle(color = ink, fontSize = 10.sp)
    val headingStyle = TextStyle(color = ink, fontSize = 13.sp)
    val homeStyle = TextStyle(color = MaterialTheme.aircast.onSuccess, fontSize = 10.sp, fontWeight = FontWeight.Bold)

    Surface(
        modifier = modifier
            .size(INSTRUMENT_SIZE)
            .semantics { contentDescription = "Attitude and heading ${reading.headingText}" },
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.85f),
    ) {
        Canvas(Modifier.size(INSTRUMENT_SIZE)) {
            val outer = size.minDimension / 2f
            val horizon = outer * HORIZON_FRACTION
            val dial = if (reading.noseUp) -reading.heading else 0f

            drawHorizon(reading, horizon, sky, ground, ink, label, measurer)

            rotate(dial) {
                (0 until 360 step 10).forEach { degrees ->
                    val long = degrees % 30 == 0
                    drawLine(
                        muted,
                        pointOnRing(center, outer - 2.dp.toPx(), degrees.toFloat()),
                        pointOnRing(center, outer - (if (long) 8.dp else 5.dp).toPx(), degrees.toFloat()),
                        strokeWidth = 1.dp.toPx(),
                    )
                }
                CARDINALS.forEach { (degrees, letter) ->
                    val text = measurer.measure(letter, label)
                    val at = pointOnRing(center, outer - 14.dp.toPx(), degrees.toFloat())
                    rotate(-dial, at) {
                        drawText(text, topLeft = at - Offset(text.size.width / 2f, text.size.height / 2f))
                    }
                }
                reading.courseOverGround?.let { drawBearing(it, outer, accent, dashed = false) }
                reading.headingToNextWaypoint?.let { drawBearing(it, outer, mission, dashed = true) }
                reading.headingToHome?.let {
                    val at = pointOnRing(center, outer - 7.dp.toPx(), it)
                    val letter = measurer.measure(HOME_LETTER, homeStyle)
                    drawCircle(home, radius = 7.dp.toPx(), center = at)
                    rotate(-dial, at) {
                        drawText(letter, topLeft = at - Offset(letter.size.width / 2f, letter.size.height / 2f))
                    }
                }
                rotate(reading.heading) {
                    val tip = Offset(center.x, center.y - outer + 1.dp.toPx())
                    drawPath(
                        Path().apply {
                            moveTo(tip.x, tip.y + 10.dp.toPx())
                            lineTo(tip.x - 6.dp.toPx(), tip.y)
                            lineTo(tip.x + 6.dp.toPx(), tip.y)
                            close()
                        },
                        accent,
                    )
                }
            }

            val headingText = measurer.measure(reading.headingText, headingStyle)
            drawText(headingText, topLeft = Offset(center.x - headingText.size.width / 2f, center.y + horizon * 0.45f))
        }
    }
}

private fun DrawScope.drawBearing(bearing: Float, outer: Float, colour: Color, dashed: Boolean) {
    drawLine(
        colour,
        pointOnRing(center, outer * 0.78f, bearing),
        pointOnRing(center, outer - 1.dp.toPx(), bearing),
        strokeWidth = 2.dp.toPx(),
        pathEffect = if (dashed) PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 3.dp.toPx())) else null,
    )
}

private fun DrawScope.drawHorizon(
    reading: Attitude,
    radius: Float,
    sky: Color,
    ground: Color,
    ink: Color,
    label: TextStyle,
    measurer: TextMeasurer,
) {
    val disc = Path().apply { addOval(Rect(center, radius)) }
    clipPath(disc) {
        rotate(-reading.roll) {
            translate(top = pitchOffset(reading.pitch, radius)) {
                drawRect(sky, topLeft = Offset(center.x - radius * 3, center.y - radius * 6), size = Size(radius * 6, radius * 6))
                drawRect(ground, topLeft = Offset(center.x - radius * 3, center.y), size = Size(radius * 6, radius * 6))
                drawLine(ink, Offset(center.x - radius * 3, center.y), Offset(center.x + radius * 3, center.y), strokeWidth = 1.dp.toPx())
                ladderAngles().forEach { degrees ->
                    val y = center.y - pitchOffset(degrees.toFloat(), radius)
                    val half = radius * (if (degrees % 10 == 0) 0.35f else 0.2f)
                    drawLine(ink.copy(alpha = 0.8f), Offset(center.x - half, y), Offset(center.x + half, y), strokeWidth = 1.dp.toPx())
                    if (degrees % 10 == 0) {
                        val text = measurer.measure(degrees.toString(), label)
                        drawText(text, topLeft = Offset(center.x + half + 2.dp.toPx(), y - text.size.height / 2f))
                    }
                }
            }
        }
    }
    drawCircle(ink.copy(alpha = 0.5f), radius = radius, center = center, style = Stroke(1.dp.toPx()))
    drawLine(ink, Offset(center.x - radius * 0.45f, center.y), Offset(center.x - radius * 0.15f, center.y), strokeWidth = 3.dp.toPx())
    drawLine(ink, Offset(center.x + radius * 0.15f, center.y), Offset(center.x + radius * 0.45f, center.y), strokeWidth = 3.dp.toPx())
    drawCircle(ink, radius = 2.dp.toPx(), center = center)
}
