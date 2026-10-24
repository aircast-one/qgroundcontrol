package one.aircast.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.bridge.refusal
import one.aircast.map.LocalFlyMapEdits
import one.aircast.map.OrbitCircle
import one.aircast.map.optText
import org.json.JSONObject

internal const val GROUND_AT = "videoAr.groundAt"
internal const val VIDEO_AR_OUTLINE = "videoAr.outline"
internal const val PAUSE_VEHICLE_PATH = "vehicle.pauseVehicle"

internal const val PICTURE_HOLD_HINT = "Press and hold a spot to look at it or fly there"
internal const val BELOW_HORIZON = "Press and hold on a spot below the horizon"
internal const val NO_GIMBAL = "The vehicle reports no gimbal, so a spot in the picture can't be placed on the map"

private const val NOT_GROUND = "notGround"
private const val HOLD_RING_DELAY_MS = 150L
private const val HOLD_RING_MIN_MS = 100L
private const val OUTLINE_SEGMENTS = 36
private const val OUTLINE_POLL_MS = 200L
private const val CHIP_SCRIM_ALPHA = 0.6f
private val OUTLINE_COLOUR = Color(0xFF00E5FF)
private val EDGE_COLOUR = Color.Black.copy(alpha = 0.6f)
private val OUTLINE_WIDTH = 3.dp
private val EDGE_WIDTH = 2.dp
private val RING_SIZE = 48.dp
private val RING_STROKE = 4.dp
private val MARK_RADIUS = 13.dp
private val RETICLE_SIZE = 64.dp
private val RETICLE_STROKE = 2.dp
private val RETICLE_DOT = 3.dp
private val CHIP_GAP = 8.dp

internal data class PicturePress(val spot: Offset, val point: MapPoint?, val actions: List<String>, val warning: String?, val reason: String)

internal fun picturePress(spot: Offset, answer: JSONObject?, hasGimbal: Boolean): PicturePress {
    val placed = answer?.takeIf { hasGimbal && it.optBoolean("ok") }
        ?.let { MapPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
        ?.takeIf { it.latitude.isFinite() && it.longitude.isFinite() }
    val listed = answer?.optJSONArray("actions")
    return PicturePress(
        spot = spot,
        point = placed,
        actions = (0 until (listed?.length() ?: 0)).mapNotNull { listed?.optString(it)?.ifBlank { null } },
        warning = answer?.optText("warning")?.ifBlank { null },
        reason = when {
            !hasGimbal -> NO_GIMBAL
            answer?.optText("refusal") == NOT_GROUND -> BELOW_HORIZON
            else -> refusal(answer).orEmpty()
        },
    )
}

internal fun groundAt(at: Offset, width: Int, height: Int, hasGimbal: Boolean): PicturePress {
    val spot = Offset(at.x / width, at.y / height)
    if (!hasGimbal) return picturePress(spot, null, hasGimbal = false)
    val (pan, tilt) = screenFraction(at.x, at.y, width, height)
    return picturePress(spot, Qgc.call(GROUND_AT, pan.toDouble(), tilt.toDouble(), width.toDouble() / height, false), hasGimbal = true)
}

@Composable
internal fun HoldRing(at: Offset?) {
    val pressed = at ?: return
    val timeout = LocalViewConfiguration.current.longPressTimeoutMillis
    val progress = remember(pressed) { Animatable(0f) }
    LaunchedEffect(pressed) {
        delay(HOLD_RING_DELAY_MS)
        progress.animateTo(1f, tween((timeout - HOLD_RING_DELAY_MS).coerceAtLeast(HOLD_RING_MIN_MS).toInt(), easing = LinearEasing))
    }
    Canvas(Modifier.fillMaxSize()) {
        if (progress.value == 0f) return@Canvas
        val stroke = RING_STROKE.toPx()
        val diameter = RING_SIZE.toPx() - stroke * 2
        drawArc(
            Color.White,
            startAngle = -90f,
            sweepAngle = 360f * progress.value,
            useCenter = false,
            topLeft = pressed - Offset(diameter / 2, diameter / 2),
            size = Size(diameter, diameter),
            style = Stroke(stroke, cap = StrokeCap.Round),
        )
    }
}

@Composable
internal fun PictureMark(spot: Offset) {
    Canvas(Modifier.fillMaxSize()) {
        val at = Offset(spot.x * size.width, spot.y * size.height)
        val radius = MARK_RADIUS.toPx()
        drawCircle(EDGE_COLOUR, radius, at, style = Stroke(EDGE_WIDTH.toPx() * 2))
        drawCircle(Color.White, radius - 1f, at, style = Stroke(EDGE_WIDTH.toPx()))
        drawCircle(Color.White, EDGE_WIDTH.toPx(), at)
    }
}

private tailrec suspend fun followOutline(circle: OrbitCircle, shown: (List<ArSpot?>) -> Unit) {
    shown(withContext(Dispatchers.Default) { outlineSpots(Qgc.call(VIDEO_AR_OUTLINE, circle.centre.latitude, circle.centre.longitude, circle.radiusMetres, OUTLINE_SEGMENTS)) })
    delay(OUTLINE_POLL_MS)
    followOutline(circle, shown)
}

@Composable
internal fun OrbitOutline(modifier: Modifier = Modifier) {
    val pending = LocalFlyMapEdits.current.orbit
    val running by qgcDerived(VIDEO_AR_VIEW, ::orbitOutline)
    var polled by remember { mutableStateOf<List<ArSpot?>>(emptyList()) }
    LaunchedEffect(pending) {
        pending?.let { circle -> followOutline(circle) { polled = it } }
    }
    val legs = outlineLegs(if (pending != null) polled else running)
    if (legs.isEmpty()) return
    Canvas(modifier) {
        val placed = legs.map { (from, to) -> arOffset(from, size.width, size.height) to arOffset(to, size.width, size.height) }
        placed.map { (start, end) -> drawLine(EDGE_COLOUR, start, end, OUTLINE_WIDTH.toPx() + EDGE_WIDTH.toPx() * 2, StrokeCap.Round) }
        placed.map { (start, end) -> drawLine(OUTLINE_COLOUR, start, end, OUTLINE_WIDTH.toPx(), StrokeCap.Round) }
    }
}

internal enum class Pointing(val text: String, val stopPath: String) {
    Looking("Looking at this point", STOP_ROI_PATH),
    Orbiting("Orbiting this point", PAUSE_VEHICLE_PATH),
}

internal fun pointing(looking: Boolean, orbiting: Boolean): Pointing? = when {
    orbiting -> Pointing.Orbiting
    looking -> Pointing.Looking
    else -> null
}

@Composable
internal fun LookAtReticle(modifier: Modifier = Modifier) {
    val looking by qgcDerived(GUIDED_ACTIONS) { it?.optBoolean("roiActive", false) == true }
    val orbiting by qgcDerived(ORBIT_VIEW) { orbitReading(it)?.orbiting == true }
    val shown = pointing(looking, orbiting) ?: return
    val flyScreen = LocalFlyScreenState.current
    val scope = rememberCoroutineScope()
    Canvas(modifier.size(RETICLE_SIZE)) {
        drawCircle(EDGE_COLOUR, size.minDimension / 2 - RETICLE_STROKE.toPx(), style = Stroke(RETICLE_STROKE.toPx() * 2))
        drawCircle(Color.White, size.minDimension / 2 - RETICLE_STROKE.toPx(), style = Stroke(RETICLE_STROKE.toPx()))
        drawCircle(Color.White, RETICLE_DOT.toPx())
    }
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = Color.Black.copy(alpha = CHIP_SCRIM_ALPHA),
        contentColor = Color.White,
        modifier = modifier.layout { measurable, constraints ->
            val chip = measurable.measure(constraints)
            layout(chip.width, chip.height) { chip.place(0, (RETICLE_SIZE / 2 + CHIP_GAP).roundToPx() + chip.height / 2) }
        }.videoControl(),
    ) {
        Row(Modifier.padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(shown.text, style = MaterialTheme.typography.labelLarge)
            TextButton(onClick = {
                scope.launch {
                    withContext(Dispatchers.Default) { Qgc.refusalOf(shown.stopPath) }?.let { flyScreen.refusal = it }
                }
            }) { Text("Stop", color = Color.White) }
        }
    }
}
