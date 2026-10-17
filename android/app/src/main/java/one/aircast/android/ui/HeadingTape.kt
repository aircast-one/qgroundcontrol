package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.roundToInt

private const val TAPE_SPAN_DEGREES = 120f
private const val TICK_STEP = 10
private const val LABEL_STEP = 30
private const val READOUT_CLEARANCE_DEGREES = 15f
private const val EDGE_MARGIN_DEGREES = 5f
private const val EDGE_FADE = 0.6f
private const val EDGE_REACH = TAPE_SPAN_DEGREES / 2f - EDGE_MARGIN_DEGREES
private val TAPE_WIDTH = 320.dp
private val TAPE_HEIGHT = 40.dp
private val POINTER = 6.dp
private val POINTER_WING = 5.dp
private val LABEL_TICK = 10.dp
private val MINOR_TICK = 5.dp
private val GAP = 2.dp
private val HOME_RADIUS = 6.dp
private val TAPE_BACKDROP = Color.Black.copy(alpha = 0.35f)
private val CARDINALS = mapOf(0 to "N", 90 to "E", 180 to "S", 270 to "W")

internal fun relativeBearing(bearing: Float, heading: Float): Float = ((bearing - heading) % 360f + 540f) % 360f - 180f

internal fun tapeX(relative: Float, width: Float): Float = width / 2f + relative / TAPE_SPAN_DEGREES * width

internal fun tapeLabel(degrees: Int): String = Math.floorMod(degrees, 360).let { CARDINALS[it] ?: it.toString() }

internal fun tapeTicks(heading: Float): List<Int> {
    val half = TAPE_SPAN_DEGREES / 2f
    val first = (ceil((heading - half) / TICK_STEP) * TICK_STEP).toInt()
    val last = (floor((heading + half) / TICK_STEP) * TICK_STEP).toInt()
    return (first..last step TICK_STEP).toList()
}

internal fun tapeLabelShown(relative: Float): Boolean =
    abs(relative) > READOUT_CLEARANCE_DEGREES && abs(relative) < EDGE_REACH

internal fun headingReadout(heading: Float): String = "${Math.floorMod(heading.roundToInt(), 360)}°"

@Composable
internal fun HeadingTape(modifier: Modifier = Modifier) {
    val view by qgcPath(ATTITUDE_PATH)
    val reading = remember(view) { attitude(view) } ?: return
    val measurer = rememberTextMeasurer()
    val ink = Color.White
    val label = TextStyle(color = ink, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
    val readout = TextStyle(color = ink, fontSize = 15.sp, fontWeight = FontWeight.Bold, fontFeatureSettings = "tnum")
    val homeLetter = TextStyle(color = MaterialTheme.aircast.onSuccess, fontSize = 9.sp, fontWeight = FontWeight.Bold)
    val home = MaterialTheme.aircast.success
    val mission = MaterialTheme.aircast.mission
    val heading = headingReadout(reading.heading)
    Surface(
        modifier.size(TAPE_WIDTH, TAPE_HEIGHT).semantics { contentDescription = "Heading $heading" },
        shape = MaterialTheme.shapes.small,
        color = osdBackdrop(TAPE_BACKDROP),
    ) {
        Canvas(Modifier.fillMaxSize().osdShadow()) {
            val line = (POINTER + GAP).toPx()
            val labelTop = line + (LABEL_TICK + GAP).toPx()
            tapeTicks(reading.heading).map { degrees ->
                val relative = relativeBearing(degrees.toFloat(), reading.heading)
                val x = tapeX(relative, size.width)
                val shade = ink.copy(alpha = 1f - abs(relative) / (TAPE_SPAN_DEGREES / 2f) * EDGE_FADE)
                val labelled = degrees % LABEL_STEP == 0
                drawLine(shade, Offset(x, line), Offset(x, line + (if (labelled) LABEL_TICK else MINOR_TICK).toPx()), strokeWidth = 1.5.dp.toPx())
                if (labelled && tapeLabelShown(relative)) {
                    val text = measurer.measure(tapeLabel(degrees), label.copy(color = shade))
                    drawText(text, topLeft = Offset(x - text.size.width / 2f, labelTop))
                }
            }
            reading.headingToNextWaypoint?.let { drawPointer(clampedTapeX(it, reading.heading), mission) }
            (reading.homeBearing ?: reading.headingToHome)?.let { bearing ->
                val at = Offset(clampedTapeX(bearing, reading.heading), line)
                val letter = measurer.measure("H", homeLetter)
                drawCircle(home, radius = HOME_RADIUS.toPx(), center = at)
                drawText(letter, topLeft = at - Offset(letter.size.width / 2f, letter.size.height / 2f))
            }
            drawPointer(size.width / 2f, ink)
            val value = measurer.measure(heading, readout)
            drawText(value, topLeft = Offset(size.width / 2f - value.size.width / 2f, labelTop - GAP.toPx()))
        }
    }
}

private fun DrawScope.clampedTapeX(bearing: Float, heading: Float): Float =
    tapeX(relativeBearing(bearing, heading).coerceIn(-EDGE_REACH, EDGE_REACH), size.width)

private fun DrawScope.drawPointer(x: Float, colour: Color) {
    val wing = POINTER_WING.toPx()
    drawPath(Path().apply { moveTo(x - wing, 0f); lineTo(x + wing, 0f); lineTo(x, POINTER.toPx()); close() }, colour)
}
