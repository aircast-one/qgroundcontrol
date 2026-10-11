package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.runningFold
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import org.json.JSONObject

internal const val VIRTUAL_JOYSTICK_PATH = "view.virtualJoystick"
internal const val VIRTUAL_JOYSTICK_VALUE = "vehicle.virtualTabletJoystickValue"
private const val DEFAULT_PERIOD_MS = 40L
private const val STICK_PAD_FRACTION = 0.3f
private val MIN_STICK_PAD = 96.dp
private val MAX_STICK_PAD = 140.dp
private const val KNOB_FRACTION = 0.16f
private const val PAD_FILL_ALPHA = 0.3f
private const val RING_ALPHA = 0.5f
private const val HELD_RING_ALPHA = 0.9f
private const val CHEVRON_ALPHA = 0.6f
private const val KNOB_SHADOW_ALPHA = 0.35f
private val CHEVRON_ARM = 5.dp
private val CHEVRON_INSET = 11.dp
private val CHEVRON_ANGLES = listOf(0f, 90f, 180f, 270f)

internal data class VirtualJoystickState(
    val show: Boolean,
    val sending: Boolean,
    val autoCenterThrottle: Boolean,
    val leftHandedMode: Boolean,
    val leftPositiveOnly: Boolean,
    val rightPositiveOnly: Boolean,
    val periodMs: Long,
)

internal fun virtualJoystick(view: JSONObject?): VirtualJoystickState? =
    view?.takeIf { it.optString("class") == "VirtualJoystick" }?.let {
        VirtualJoystickState(
            show = it.optBoolean("show"),
            sending = it.optBoolean("sending"),
            autoCenterThrottle = it.optBoolean("autoCenterThrottle"),
            leftHandedMode = it.optBoolean("leftHandedMode"),
            leftPositiveOnly = it.optBoolean("leftPositiveOnly"),
            rightPositiveOnly = it.optBoolean("rightPositiveOnly"),
            periodMs = it.optLong("periodMs", DEFAULT_PERIOD_MS),
        )
    }

internal data class StickAxes(val x: Double, val y: Double)

internal data class StickStates(val previous: VirtualJoystickState?, val current: VirtualJoystickState?)

internal fun sticksReset(previous: VirtualJoystickState?, current: VirtualJoystickState?): Boolean =
    current?.show != true || current.autoCenterThrottle != previous?.autoCenterThrottle

internal fun restingLeft(autoCenterThrottle: Boolean): Offset = Offset(0.5f, restingY(autoCenterThrottle))

internal val RESTING_RIGHT = Offset(0.5f, 0.5f)

internal fun released(stick: Offset, reCenterY: Boolean): Offset = Offset(0.5f, if (reCenterY) 0.5f else stick.y)

internal fun stickValues(state: VirtualJoystickState, left: Offset?, right: Offset?): List<Double> {
    val l = left ?: restingLeft(state.autoCenterThrottle)
    val r = right ?: RESTING_RIGHT
    return joystickValues(stickAxes(l.x, l.y, state.leftPositiveOnly), stickAxes(r.x, r.y, state.rightPositiveOnly), state.leftHandedMode)
}

object VirtualStickSender {
    @Volatile
    internal var left: Offset? = null

    @Volatile
    internal var right: Offset? = null

    fun start(scope: kotlinx.coroutines.CoroutineScope) {
        scope.launch(Dispatchers.IO) {
            Qgc.watch(listOf(VIRTUAL_JOYSTICK_PATH))
            try {
                Qgc.path(VIRTUAL_JOYSTICK_PATH)
                    .map { virtualJoystick(it) }
                    .distinctUntilChanged()
                    .runningFold(StickStates(null, null)) { states, next -> StickStates(states.current, next) }
                    .collectLatest { states ->
                        if (sticksReset(states.previous, states.current)) {
                            left = null
                            right = null
                        }
                        states.current?.takeIf { it.show && it.sending }?.let { sending ->
                            while (isActive) {
                                Qgc.invoke(VIRTUAL_JOYSTICK_VALUE, *stickValues(sending, left, right).toTypedArray())
                                delay(sending.periodMs)
                            }
                        }
                    }
            } finally {
                Qgc.unwatch(listOf(VIRTUAL_JOYSTICK_PATH))
            }
        }
    }
}

internal fun stickAxes(fractionX: Float, fractionY: Float, positiveOnly: Boolean): StickAxes {
    val pctUp = 1.0 - fractionY.coerceIn(0f, 1f)
    return StickAxes(
        x = fractionX.coerceIn(0f, 1f) * 2.0 - 1.0,
        y = if (positiveOnly) pctUp else pctUp * 2.0 - 1.0,
    )
}

internal fun restingY(reCenter: Boolean): Float = if (reCenter) 0.5f else 1f

internal fun joystickValues(left: StickAxes, right: StickAxes, leftHanded: Boolean): List<Double> =
    if (leftHanded) {
        listOf(left.x, left.y, right.x, right.y)
    } else {
        listOf(right.x, right.y, left.x, left.y)
    }

internal fun stickPad(shortSide: Dp): Dp = (shortSide * STICK_PAD_FRACTION).coerceIn(MIN_STICK_PAD, MAX_STICK_PAD)

internal fun knobTravel(pad: Float): Float = pad / 2f - pad * KNOB_FRACTION

internal fun knobOffset(stick: Offset, travel: Float): Offset {
    val wanted = Offset(stick.x - 0.5f, stick.y - 0.5f) * (2f * travel)
    val distance = wanted.getDistance()
    return if (distance > travel) wanted * (travel / distance) else wanted
}

internal fun dragged(start: Offset, moved: Offset, travel: Float): Offset =
    Offset((start.x + moved.x / (2f * travel)).coerceIn(0f, 1f), (start.y + moved.y / (2f * travel)).coerceIn(0f, 1f))

@Composable
internal fun shownStickPad(): Dp? {
    val shown by qgcDerived(VIRTUAL_JOYSTICK_PATH) { virtualJoystick(it)?.show == true }
    val configuration = LocalConfiguration.current
    return stickPad(minOf(configuration.screenWidthDp, configuration.screenHeightDp).dp).takeIf { shown }
}

@Composable
fun VirtualJoystick(pad: Dp, modifier: Modifier = Modifier) {
    val view by qgcPath(VIRTUAL_JOYSTICK_PATH)
    val state = virtualJoystick(view)?.takeIf { it.show } ?: return

    var left by remember(state.autoCenterThrottle) { mutableStateOf(VirtualStickSender.left ?: restingLeft(state.autoCenterThrottle)) }
    var right by remember { mutableStateOf(VirtualStickSender.right ?: RESTING_RIGHT) }
    val autoCenter by rememberUpdatedState(state.autoCenterThrottle)
    LaunchedEffect(left, right) {
        VirtualStickSender.left = left
        VirtualStickSender.right = right
    }
    DisposableEffect(Unit) {
        onDispose {
            VirtualStickSender.left = VirtualStickSender.left?.let { released(it, autoCenter) }
            VirtualStickSender.right = VirtualStickSender.right?.let { released(it, true) }
        }
    }

    Row(modifier.fillMaxWidth()) {
        ThumbPad(stick = left, reCenterY = state.autoCenterThrottle, description = "Left virtual stick", size = pad, onMove = { left = it })
        Spacer(Modifier.weight(1f))
        ThumbPad(stick = right, reCenterY = true, description = "Right virtual stick", size = pad, onMove = { right = it })
    }
}

@Composable
private fun ThumbPad(stick: Offset, reCenterY: Boolean, description: String, size: Dp, onMove: (Offset) -> Unit) {
    val ink = MaterialTheme.aircast.outdoorForeground
    val latestStick by rememberUpdatedState(stick)
    var held by remember { mutableStateOf(false) }
    Canvas(
        Modifier
            .size(size)
            .semantics { contentDescription = description }
            .pointerInput(reCenterY) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    held = true
                    val origin = down.position
                    val start = latestStick
                    val travel = knobTravel(this.size.width.toFloat())
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id }
                        if (change == null || !change.pressed) break
                        change.consume()
                        onMove(dragged(start, change.position - origin, travel))
                    }
                    held = false
                    onMove(released(latestStick, reCenterY))
                }
            },
    ) {
        val radius = this.size.minDimension / 2f
        drawCircle(Color.Black.copy(alpha = PAD_FILL_ALPHA), radius)
        drawCircle(ink.copy(alpha = if (held) HELD_RING_ALPHA else RING_ALPHA), radius - 0.75.dp.toPx(), style = Stroke(1.5.dp.toPx()))
        CHEVRON_ANGLES.map { angle -> rotate(angle) { chevron(ink.copy(alpha = CHEVRON_ALPHA), radius) } }
        val knob = this.size.minDimension * KNOB_FRACTION
        val at = center + knobOffset(stick, knobTravel(this.size.minDimension))
        drawCircle(Color.Black.copy(alpha = KNOB_SHADOW_ALPHA), knob + 2.dp.toPx(), at)
        drawCircle(ink, if (held) knob * 1.1f else knob, at)
    }
}

private fun DrawScope.chevron(color: Color, radius: Float) {
    val tip = Offset(center.x, center.y - radius + CHEVRON_INSET.toPx())
    val arm = CHEVRON_ARM.toPx()
    val stroke = 2.dp.toPx()
    drawLine(color, tip, tip + Offset(-arm, arm), stroke, StrokeCap.Round)
    drawLine(color, tip, tip + Offset(arm, arm), stroke, StrokeCap.Round)
}
