package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.runningFold
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

internal const val VIRTUAL_JOYSTICK_PATH = "view.virtualJoystick"
internal const val VIRTUAL_JOYSTICK_VALUE = "vehicle.virtualTabletJoystickValue"
private val MAX_PAD_SIZE = 160.dp
private const val PAD_HEIGHT_FRACTION = 0.25f
private const val DEFAULT_PERIOD_MS = 40L

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
            Qgc.values
                .map { virtualJoystick(it[VIRTUAL_JOYSTICK_PATH]) }
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

@Composable
fun VirtualJoystick(modifier: Modifier = Modifier) {
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

    BoxWithConstraints(modifier.fillMaxWidth()) {
        val pad: Dp = min(maxHeight.takeIf { it != Dp.Infinity }?.times(PAD_HEIGHT_FRACTION) ?: MAX_PAD_SIZE, MAX_PAD_SIZE)
        Row(Modifier.fillMaxWidth().height(pad)) {
            ThumbPad(
                stick = left,
                reCenterY = state.autoCenterThrottle,
                description = "Left virtual stick",
                size = pad,
                onMove = { left = it },
            )
            Spacer(Modifier.weight(1f))
            ThumbPad(
                stick = right,
                reCenterY = true,
                description = "Right virtual stick",
                size = pad,
                onMove = { right = it },
            )
        }
    }
}

@Composable
private fun ThumbPad(stick: Offset, reCenterY: Boolean, description: String, size: Dp, onMove: (Offset) -> Unit) {
    val ring = MaterialTheme.colorScheme.onSurface
    val fill = MaterialTheme.colorScheme.surface.copy(alpha = 0.5f)
    val hat = MaterialTheme.colorScheme.primary
    val latestStick by rememberUpdatedState(stick)
    Canvas(
        Modifier
            .size(size)
            .semantics { contentDescription = description }
            .pointerInput(reCenterY) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    val origin = down.position
                    val start = latestStick
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id }
                        if (change == null || !change.pressed) break
                        change.consume()
                        val moved = change.position - origin
                        onMove(
                            Offset(
                                (start.x + moved.x / this.size.width).coerceIn(0f, 1f),
                                (start.y + moved.y / this.size.height).coerceIn(0f, 1f),
                            ),
                        )
                    }
                    onMove(released(latestStick, reCenterY))
                }
            },
    ) {
        val radius = this.size.minDimension / 2f
        drawCircle(fill, radius)
        drawCircle(ring, radius, style = Stroke(2.dp.toPx()))
        drawCircle(ring, radius / 2f, style = Stroke(2.dp.toPx()))
        drawCircle(hat, 12.dp.toPx(), center = Offset(stick.x * this.size.width, stick.y * this.size.height))
    }
}
