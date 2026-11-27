package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeJoin
import one.aircast.map.AircastSpace
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

internal const val ATTITUDE_PATH = "view.attitude"
private const val HORIZON_FRACTION = 0.72f
private const val PITCH_SPAN_DEGREES = 45f

internal data class Attitude(
    val roll: Float,
    val pitch: Float,
    val heading: Float,
    val headingText: String,
    val courseOverGround: Float?,
    val headingToHome: Float?,
    val headingToNextWaypoint: Float?,
    val noseUp: Boolean,
    val homeBearing: Float? = null,
    val pilotBearing: Float? = null,
)

private fun JSONObject.optAngle(key: String): Float? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }?.toFloat()

internal val NO_VEHICLE_ATTITUDE = Attitude(0f, 0f, 0f, "", null, null, null, false)

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
            homeBearing = it.optAngle("homeBearing"),
            pilotBearing = it.optAngle("pilotBearing"),
        )
    }

internal fun pitchOffset(pitch: Float, horizonRadius: Float): Float =
    pitch * (horizonRadius * 2f) / PITCH_SPAN_DEGREES

private fun pointOnRing(center: Offset, radius: Float, degrees: Float): Offset {
    val radians = Math.toRadians(degrees.toDouble())
    return Offset(center.x + radius * sin(radians).toFloat(), center.y - radius * cos(radians).toFloat())
}

private const val OSD_DIAL_SCRIM = 0.4f
private const val OSD_DIAL_RING_ALPHA = 0.7f
private const val OSD_DIAL_HORIZON_ALPHA = 0.45f
private const val OSD_CHEVRON_TIP = 0.42f
private const val OSD_CHEVRON_WING = 0.3f
private const val OSD_CHEVRON_NOTCH = 0.14f
private val OSD_NORTH = Color(0xFFFF4D4D)
private val ROOMY_DIAL = 96.dp

@Composable
internal fun OsdCompassDial(size: androidx.compose.ui.unit.Dp, modifier: Modifier = Modifier) {
    val view by qgcPath(ATTITUDE_PATH)
    val reading = remember(view) { attitude(view) ?: NO_VEHICLE_ATTITUDE }
    val measurer = rememberTextMeasurer()
    val white = Color.White
    val home = MaterialTheme.aircast.success
    val north = TextStyle(color = OSD_NORTH, fontSize = 11.sp, fontWeight = FontWeight.Bold)
    val homeStyle = TextStyle(color = MaterialTheme.aircast.onSuccess, fontSize = 9.sp, fontWeight = FontWeight.Bold)
    val pilot = MaterialTheme.colorScheme.primary
    val roomy = size >= ROOMY_DIAL
    val headingStyle = TextStyle(color = white, fontSize = 11.sp, fontWeight = FontWeight.Bold)
    Canvas(modifier.size(size).semantics { contentDescription = "Heading ${reading.headingText}" }) {
        val radius = this.size.minDimension / 2f
        drawCircle(Color.Black.copy(alpha = OSD_DIAL_SCRIM), radius)
        clipPath(Path().apply { addOval(Rect(center, radius)) }) {
            rotate(-reading.roll) {
                translate(top = pitchOffset(reading.pitch, radius * HORIZON_FRACTION)) {
                    drawLine(white.copy(alpha = OSD_DIAL_HORIZON_ALPHA), Offset(center.x - radius, center.y), Offset(center.x + radius, center.y), strokeWidth = 1.5.dp.toPx())
                }
            }
        }
        drawCircle(white.copy(alpha = OSD_DIAL_RING_ALPHA), radius - 1.dp.toPx(), style = Stroke(1.5.dp.toPx()))
        (0 until 360 step 30).map { degrees ->
            drawLine(
                white.copy(alpha = OSD_DIAL_RING_ALPHA),
                pointOnRing(center, radius - 2.dp.toPx(), degrees - reading.heading),
                pointOnRing(center, radius - (if (degrees % 90 == 0) 8.dp else 5.dp).toPx(), degrees - reading.heading),
                strokeWidth = 1.5.dp.toPx(),
            )
        }
        reading.pilotBearing?.let { bearing ->
            val at = pointOnRing(center, radius - 8.dp.toPx(), bearing - reading.heading)
            drawCircle(white, radius = 5.dp.toPx(), center = at)
            drawCircle(pilot, radius = 3.5.dp.toPx(), center = at)
        }
        val n = measurer.measure("N", north)
        val northAt = pointOnRing(center, radius - 15.dp.toPx(), -reading.heading)
        drawText(n, topLeft = northAt - Offset(n.size.width / 2f, n.size.height / 2f))
        (reading.homeBearing ?: reading.headingToHome)?.let { bearing ->
            val at = pointOnRing(center, radius - 8.dp.toPx(), bearing - reading.heading)
            val letter = measurer.measure("H", homeStyle)
            drawCircle(home, radius = 7.dp.toPx(), center = at)
            drawText(letter, topLeft = at - Offset(letter.size.width / 2f, letter.size.height / 2f))
        }
        val chevron = Path().apply {
            moveTo(center.x, center.y - radius * OSD_CHEVRON_TIP)
            lineTo(center.x + radius * OSD_CHEVRON_WING, center.y + radius * OSD_CHEVRON_WING)
            lineTo(center.x, center.y + radius * OSD_CHEVRON_NOTCH)
            lineTo(center.x - radius * OSD_CHEVRON_WING, center.y + radius * OSD_CHEVRON_WING)
            close()
        }
        drawPath(chevron, white)
        drawPath(chevron, Color.Black.copy(alpha = OSD_DIAL_SCRIM), style = Stroke(1.dp.toPx()))
        if (roomy && reading.headingText.isNotBlank()) {
            val text = measurer.measure(reading.headingText, headingStyle)
            drawText(text, topLeft = Offset(center.x - text.size.width / 2f, center.y + radius * 0.5f))
        }
    }
}

private val SKY = Color(0xFF3A78C4)
private val GROUND = Color(0xFF4F8A3C)
private val WINGS = Color(0xFFFF6B2C)
private val LADDER = listOf(-20, -10, 10, 20)
private val ROLL_TICKS = listOf(-60, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60)
private val ATTITUDE_GAUGE_SIZE = 96.dp

internal fun attitudeLabel(reading: Attitude): String = "Attitude, pitch ${reading.pitch.roundToInt()}°, roll ${reading.roll.roundToInt()}°"

@Composable
internal fun OsdAttitudeDial(size: androidx.compose.ui.unit.Dp, modifier: Modifier = Modifier) {
    val view by qgcPath(ATTITUDE_PATH)
    val reading = remember(view) { attitude(view) ?: NO_VEHICLE_ATTITUDE }
    val measurer = rememberTextMeasurer()
    val white = Color.White
    val rung = TextStyle(color = white, fontSize = 8.sp)
    Canvas(modifier.size(size).semantics { contentDescription = attitudeLabel(reading) }) {
        val radius = this.size.minDimension / 2f
        val horizon = radius * HORIZON_FRACTION
        val step = pitchOffset(10f, horizon)
        clipPath(Path().apply { addOval(Rect(center, radius)) }) {
            rotate(-reading.roll) {
                translate(top = pitchOffset(reading.pitch, horizon)) {
                    drawRect(SKY, Offset(center.x - radius * 3f, center.y - radius * 4f), Size(radius * 6f, radius * 4f))
                    drawRect(GROUND, Offset(center.x - radius * 3f, center.y), Size(radius * 6f, radius * 4f))
                    drawLine(white, Offset(center.x - radius * 3f, center.y), Offset(center.x + radius * 3f, center.y), strokeWidth = 1.5.dp.toPx())
                    LADDER.map { degrees ->
                        val y = center.y - degrees / 10f * step
                        val label = measurer.measure(abs(degrees).toString(), rung)
                        drawLine(white, Offset(center.x - radius * 0.18f, y), Offset(center.x + radius * 0.18f, y), strokeWidth = 1.dp.toPx())
                        drawText(label, topLeft = Offset(center.x + radius * 0.24f, y - label.size.height / 2f))
                    }
                }
            }
        }
        ROLL_TICKS.map { degrees ->
            drawLine(
                white,
                pointOnRing(center, radius - 1.dp.toPx(), degrees.toFloat()),
                pointOnRing(center, radius - (if (degrees % 30 == 0) 7.dp else 4.dp).toPx(), degrees.toFloat()),
                strokeWidth = 1.dp.toPx(),
            )
        }
        drawCircle(white.copy(alpha = OSD_DIAL_RING_ALPHA), radius - 0.75.dp.toPx(), style = Stroke(1.5.dp.toPx()))
        drawCircle(WINGS, radius = 2.5.dp.toPx(), center = pointOnRing(center, radius - 3.dp.toPx(), -reading.roll))
        val wing = radius * 0.42f
        val wings = Path().apply {
            moveTo(center.x - wing, center.y)
            lineTo(center.x - wing * 0.35f, center.y)
            lineTo(center.x, center.y + wing * 0.25f)
            lineTo(center.x + wing * 0.35f, center.y)
            lineTo(center.x + wing, center.y)
        }
        drawPath(wings, WINGS, style = Stroke(3.dp.toPx(), join = StrokeJoin.Round))
    }
}

@Composable
internal fun AttitudeGauges() {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2)) {
        OsdAttitudeDial(ATTITUDE_GAUGE_SIZE)
        OsdCompassDial(ATTITUDE_GAUGE_SIZE)
    }
}

@Composable
internal fun AttitudeChoice() {
    val layout = LocalFlyScreenState.current.layout
    val shown = ATTITUDE_KEY !in layout.hidden
    SectionHeader("Instruments")
    ListItem(
        headlineContent = { Text(ATTITUDE_NAME) },
        leadingContent = { Checkbox(checked = shown, onCheckedChange = null) },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = Modifier.clickable { layout.setHidden(ATTITUDE_KEY, shown) },
    )
}
