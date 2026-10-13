package one.aircast.android.ui

import android.os.SystemClock
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.runningFold
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicLong

internal const val VIRTUAL_JOYSTICK_PATH = "view.virtualJoystick"
internal const val VIRTUAL_JOYSTICK_VALUE = "vehicle.virtualTabletJoystickValue"
private const val DEFAULT_PERIOD_MS = 40L
internal const val QUICKEST_RESEND_MS = 20L
private const val STICK_PAD_FRACTION = 0.42f
private val MIN_STICK_PAD = 96.dp
private val MAX_STICK_PAD = 200.dp
private const val KNOB_FRACTION = 0.22f
private const val PAD_FILL_ALPHA = 0.12f
private const val RING_ALPHA = 0.45f
private const val HELD_RING_ALPHA = 0.8f
private const val KNOB_ALPHA = 0.28f
private const val HELD_KNOB_ALPHA = 0.45f
private const val LEFT_STICK_KEY = "leftStick"
private const val RIGHT_STICK_KEY = "rightStick"

internal data class VirtualJoystickState(
    val enabled: Boolean,
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
            enabled = it.optBoolean("enabled"),
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

internal data class Sticks(val left: Offset, val right: Offset)

internal val CENTRED = Offset(0.5f, 0.5f)

private val GROUND_STATES = setOf("disarmed", "armed")

internal fun sticksReset(previous: VirtualJoystickState?, current: VirtualJoystickState?): Boolean =
    current?.enabled != true || current.autoCenterThrottle != previous?.autoCenterThrottle || current.leftHandedMode != previous.leftHandedMode

internal fun onTheGround(fly: FlyState?): Boolean = fly?.connected == true && fly.state in GROUND_STATES

internal fun throttleOnLeft(state: VirtualJoystickState): Boolean = !state.leftHandedMode

internal fun throttleParked(state: VirtualJoystickState, grounded: Boolean): Boolean =
    grounded && if (throttleOnLeft(state)) state.leftPositiveOnly else state.rightPositiveOnly

internal fun released(stick: Offset, throttle: Boolean, autoCenter: Boolean, parked: Boolean): Offset =
    Offset(0.5f, if (!throttle || autoCenter || parked) 0.5f else stick.y)

internal fun shownSticks(state: VirtualJoystickState, left: Offset?, right: Offset?, grounded: Boolean, leftHeld: Boolean, rightHeld: Boolean): Sticks {
    val throttleLeft = throttleOnLeft(state)
    val parked = throttleParked(state, grounded)
    val park = { stick: Offset, held: Boolean -> if (parked && !held) Offset(stick.x, 1f) else stick }
    val l = left ?: CENTRED
    val r = right ?: CENTRED
    return if (throttleLeft) Sticks(park(l, leftHeld), r) else Sticks(l, park(r, rightHeld))
}

internal fun stickValues(state: VirtualJoystickState, sticks: Sticks): List<Double> =
    joystickValues(stickAxes(sticks.left.x, sticks.left.y, state.leftPositiveOnly), stickAxes(sticks.right.x, sticks.right.y, state.rightPositiveOnly), state.leftHandedMode)

object VirtualStickSender {
    @Volatile
    internal var left: Offset? = null

    @Volatile
    internal var right: Offset? = null

    @Volatile
    internal var layout: Pair<Boolean, Boolean>? = null

    @Volatile
    internal var leftHeld: Boolean = false

    @Volatile
    internal var rightHeld: Boolean = false

    private val moved = Channel<Unit>(Channel.CONFLATED)
    private val touchedAt = AtomicLong(0L)

    internal fun moveLeft(stick: Offset, at: Long) {
        left = stick
        touchedAt.compareAndSet(0L, at)
        moved.trySend(Unit)
    }

    internal fun moveRight(stick: Offset, at: Long) {
        right = stick
        touchedAt.compareAndSet(0L, at)
        moved.trySend(Unit)
    }

    fun start(scope: kotlinx.coroutines.CoroutineScope) {
        scope.launch(Dispatchers.IO) {
            Qgc.watch(listOf(VIRTUAL_JOYSTICK_PATH, FLY_STATE))
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
                        states.current?.takeIf { it.enabled && it.sending }?.let { sending ->
                            val fly = Qgc.path(FLY_STATE)
                            while (isActive) {
                                val sticks = shownSticks(sending, left, right, onTheGround(flyState(fly.value)), leftHeld, rightHeld)
                                val touchAge = touchedAt.getAndSet(0L).takeIf { it > 0L }?.let { SystemClock.uptimeMillis() - it }
                                Qgc.invoke(VIRTUAL_JOYSTICK_VALUE, *(stickValues(sending, sticks) + listOfNotNull(touchAge)).toTypedArray())
                                delay(QUICKEST_RESEND_MS)
                                withTimeoutOrNull(sending.periodMs - QUICKEST_RESEND_MS) { moved.receive() }
                            }
                        }
                    }
            } finally {
                Qgc.unwatch(listOf(VIRTUAL_JOYSTICK_PATH, FLY_STATE))
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

internal fun joystickValues(left: StickAxes, right: StickAxes, leftHanded: Boolean): List<Double> =
    if (leftHanded) {
        listOf(left.x, left.y, right.x, right.y)
    } else {
        listOf(right.x, right.y, left.x, left.y)
    }

internal fun stickPad(shortSide: Dp, room: Dp = Dp.Infinity): Dp = minOf(shortSide * STICK_PAD_FRACTION, room).coerceIn(MIN_STICK_PAD, MAX_STICK_PAD)

internal fun knobTravel(pad: Float): Float = pad / 2f - pad * KNOB_FRACTION

internal fun knobOffset(stick: Offset, travel: Float): Offset {
    val wanted = Offset(stick.x - 0.5f, stick.y - 0.5f) * (2f * travel)
    val distance = wanted.getDistance()
    return if (distance > travel) wanted * (travel / distance) else wanted
}

internal fun dragged(start: Offset, moved: Offset, travel: Float): Offset =
    Offset((start.x + moved.x / (2f * travel)).coerceIn(0f, 1f), (start.y + moved.y / (2f * travel)).coerceIn(0f, 1f))

@Composable
internal fun shownStickPad(shortSide: Dp, room: Dp = Dp.Infinity): Dp? {
    val shown by qgcDerived(VIRTUAL_JOYSTICK_PATH) { virtualJoystick(it)?.show == true }
    return stickPad(shortSide, room).takeIf { shown }
}

@Composable
fun VirtualJoystick(pad: Dp, modifier: Modifier = Modifier, trailing: @Composable () -> Unit = {}) {
    val view by qgcPath(VIRTUAL_JOYSTICK_PATH)
    val state = virtualJoystick(view)?.takeIf { it.show } ?: return
    val grounded by qgcDerived(FLY_STATE) { onTheGround(flyState(it)) }

    val layout = state.autoCenterThrottle to state.leftHandedMode
    val kept = VirtualStickSender.layout == layout
    var left by remember(layout) { mutableStateOf(VirtualStickSender.left?.takeIf { kept } ?: CENTRED) }
    var right by remember(layout) { mutableStateOf(VirtualStickSender.right?.takeIf { kept } ?: CENTRED) }
    var leftHeld by remember { mutableStateOf(false) }
    var rightHeld by remember { mutableStateOf(false) }
    val latestState by rememberUpdatedState(state)
    val parked = throttleParked(state, grounded)
    val latestParked by rememberUpdatedState(parked)
    LaunchedEffect(layout, left, right, leftHeld, rightHeld) {
        VirtualStickSender.layout = layout
        VirtualStickSender.left = left
        VirtualStickSender.right = right
        VirtualStickSender.leftHeld = leftHeld
        VirtualStickSender.rightHeld = rightHeld
    }
    DisposableEffect(Unit) {
        onDispose {
            val throttleLeft = throttleOnLeft(latestState)
            VirtualStickSender.left = VirtualStickSender.left?.let { released(it, throttleLeft, latestState.autoCenterThrottle, latestParked) }
            VirtualStickSender.right = VirtualStickSender.right?.let { released(it, !throttleLeft, latestState.autoCenterThrottle, latestParked) }
            VirtualStickSender.leftHeld = false
            VirtualStickSender.rightHeld = false
        }
    }

    val shown = shownSticks(state, left, right, grounded, leftHeld, rightHeld)
    val release = { stick: Offset, throttle: Boolean -> released(stick, throttle, latestState.autoCenterThrottle, latestParked) }
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
        ThumbPad(
            stick = shown.left,
            description = "Left virtual stick",
            size = pad,
            modifier = Modifier.avoidedByVideoMessage(LEFT_STICK_KEY),
            onHeld = { leftHeld = it },
            onMove = { stick, at ->
                left = stick
                VirtualStickSender.moveLeft(stick, at)
            },
            onRelease = {
                left = release(it, throttleOnLeft(latestState))
                VirtualStickSender.moveLeft(left, SystemClock.uptimeMillis())
            },
        )
        Spacer(Modifier.weight(1f))
        ThumbPad(
            stick = shown.right,
            description = "Right virtual stick",
            size = pad,
            modifier = Modifier.avoidedByVideoMessage(RIGHT_STICK_KEY),
            onHeld = { rightHeld = it },
            onMove = { stick, at ->
                right = stick
                VirtualStickSender.moveRight(stick, at)
            },
            onRelease = {
                right = release(it, !throttleOnLeft(latestState))
                VirtualStickSender.moveRight(right, SystemClock.uptimeMillis())
            },
        )
        trailing()
    }
}

@Composable
private fun ThumbPad(stick: Offset, description: String, size: Dp, modifier: Modifier, onHeld: (Boolean) -> Unit, onMove: (Offset, Long) -> Unit, onRelease: (Offset) -> Unit) {
    val ink = MaterialTheme.aircast.outdoorForeground
    val latestStick by rememberUpdatedState(stick)
    val latestHeld by rememberUpdatedState(onHeld)
    val latestMove by rememberUpdatedState(onMove)
    val latestRelease by rememberUpdatedState(onRelease)
    var held by remember { mutableStateOf(false) }
    Canvas(
        modifier
            .size(size)
            .semantics { contentDescription = description }
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    held = true
                    latestHeld(true)
                    val origin = down.position
                    val start = latestStick
                    val travel = knobTravel(this.size.width.toFloat())
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id }
                        if (change == null || !change.pressed) break
                        change.consume()
                        latestMove(dragged(start, change.position - origin, travel), change.uptimeMillis)
                    }
                    held = false
                    latestHeld(false)
                    latestRelease(latestStick)
                }
            },
    ) {
        val radius = this.size.minDimension / 2f
        drawCircle(Color.Black.copy(alpha = PAD_FILL_ALPHA), radius)
        drawCircle(ink.copy(alpha = if (held) HELD_RING_ALPHA else RING_ALPHA), radius - 0.75.dp.toPx(), style = Stroke(1.5.dp.toPx()))
        val at = center + knobOffset(stick, knobTravel(this.size.minDimension))
        drawCircle(ink.copy(alpha = if (held) HELD_KNOB_ALPHA else KNOB_ALPHA), this.size.minDimension * KNOB_FRACTION, at)
    }
}
