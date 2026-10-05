package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import one.aircast.mapspike.aircast
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.roundToInt

private const val TAPE_SPAN_DEGREES = 120f
private const val TICK_STEP = 5
private const val MAJOR_STEP = 15
private const val LABEL_STEP = 30
private const val POINTER_CLEARANCE_DEGREES = 8f
private val TAPE_WIDTH = 240.dp
private val TAPE_HEIGHT = 52.dp
private val TEXT_SHADOW = Shadow(Color.Black, Offset(0f, 1f), 3f)

internal fun relativeBearing(bearing: Float, heading: Float): Float = ((bearing - heading) % 360f + 540f) % 360f - 180f

internal fun tapeX(relative: Float, width: Float): Float = width / 2f + relative / TAPE_SPAN_DEGREES * width

internal fun tapeLabel(degrees: Int): String = when (val normal = ((degrees % 360) + 360) % 360) {
    0 -> "N"
    90 -> "E"
    180 -> "S"
    270 -> "W"
    else -> normal.toString()
}

internal fun tapeTicks(heading: Float): List<Int> {
    val half = TAPE_SPAN_DEGREES / 2f
    val first = (ceil((heading - half) / TICK_STEP) * TICK_STEP).toInt()
    val last = (floor((heading + half) / TICK_STEP) * TICK_STEP).toInt()
    return (first..last step TICK_STEP).toList()
}

internal fun headingReadout(heading: Float): String = "%03d".format(((heading.roundToInt() % 360) + 360) % 360)

@Composable
internal fun HeadingTape(reading: Attitude, modifier: Modifier = Modifier) {
    val measurer = rememberTextMeasurer()
    val ink = Color.White
    val muted = Color.White.copy(alpha = 0.7f)
    val label = TextStyle(color = ink, fontSize = 11.sp, fontWeight = FontWeight.SemiBold, shadow = TEXT_SHADOW)
    val readout = TextStyle(color = ink, fontSize = 15.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, shadow = TEXT_SHADOW)
    val marker = TextStyle(color = MaterialTheme.aircast.onSuccess, fontSize = 9.sp, fontWeight = FontWeight.Bold)
    val home = MaterialTheme.aircast.success
    val mission = MaterialTheme.aircast.mission
    Surface(
        modifier.width(TAPE_WIDTH).height(TAPE_HEIGHT).semantics { contentDescription = "Heading ${reading.headingText}" },
        shape = MaterialTheme.shapes.small,
        color = Color.Black.copy(alpha = 0.35f),
    ) {
        Canvas(Modifier.width(TAPE_WIDTH).height(TAPE_HEIGHT)) {
            val baseline = size.height - 6.dp.toPx()
            tapeTicks(reading.heading).forEach { degrees ->
                val x = tapeX(relativeBearing(degrees.toFloat(), reading.heading), size.width)
                val fade = 1f - abs(x - size.width / 2f) / (size.width / 2f) * 0.6f
                val tall = when {
                    degrees % LABEL_STEP == 0 -> 10.dp
                    degrees % MAJOR_STEP == 0 -> 7.dp
                    else -> 4.dp
                }.toPx()
                drawLine(muted.copy(alpha = muted.alpha * fade), Offset(x, baseline), Offset(x, baseline - tall), strokeWidth = 1.5.dp.toPx())
                if (degrees % LABEL_STEP == 0 && abs(relativeBearing(degrees.toFloat(), reading.heading)) > POINTER_CLEARANCE_DEGREES) {
                    val text = measurer.measure(tapeLabel(degrees), label.copy(color = ink.copy(alpha = fade)))
                    val left = x - text.size.width / 2f
                    if (left >= 0f && left + text.size.width <= size.width) drawText(text, topLeft = Offset(left, baseline - tall - text.size.height))
                }
            }
            reading.courseOverGround?.let { drawCaret(it, reading.heading, baseline, COURSE_COLOUR) }
            reading.headingToNextWaypoint?.let { drawCaret(it, reading.heading, baseline, mission) }
            reading.headingToHome?.let { drawHome(it, reading.heading, baseline, home, measurer, marker) }
            val value = measurer.measure(headingReadout(reading.heading), readout)
            val boxWidth = value.size.width + 10.dp.toPx()
            val boxLeft = size.width / 2f - boxWidth / 2f
            drawRoundRect(Color.Black.copy(alpha = 0.6f), topLeft = Offset(boxLeft, 2.dp.toPx()), size = Size(boxWidth, value.size.height.toFloat()), cornerRadius = CornerRadius(4.dp.toPx()))
            drawText(value, topLeft = Offset(size.width / 2f - value.size.width / 2f, 2.dp.toPx()))
            val pointer = 2.dp.toPx() + value.size.height
            drawPath(Path().apply { moveTo(size.width / 2f - 5.dp.toPx(), pointer); lineTo(size.width / 2f + 5.dp.toPx(), pointer); lineTo(size.width / 2f, pointer + 6.dp.toPx()); close() }, ink)
        }
    }
}

private fun DrawScope.clampedTapeX(bearing: Float, heading: Float): Float =
    tapeX(relativeBearing(bearing, heading).coerceIn(-TAPE_SPAN_DEGREES / 2f, TAPE_SPAN_DEGREES / 2f), size.width)

private fun DrawScope.drawCaret(bearing: Float, heading: Float, baseline: Float, colour: Color) {
    val x = clampedTapeX(bearing, heading)
    val wing = 4.dp.toPx()
    drawPath(Path().apply { moveTo(x, baseline - 1.dp.toPx()); lineTo(x - wing, baseline + 5.dp.toPx()); lineTo(x + wing, baseline + 5.dp.toPx()); close() }, colour)
}

private fun DrawScope.drawHome(bearing: Float, heading: Float, baseline: Float, colour: Color, measurer: TextMeasurer, style: TextStyle) {
    val at = Offset(clampedTapeX(bearing, heading), baseline - 3.dp.toPx())
    val letter = measurer.measure(HOME_LETTER, style)
    drawCircle(colour, radius = 6.dp.toPx(), center = at)
    drawText(letter, topLeft = at - Offset(letter.size.width / 2f, letter.size.height / 2f))
}
