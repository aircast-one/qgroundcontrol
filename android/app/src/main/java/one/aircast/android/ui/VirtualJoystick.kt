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
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
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

private const val STATE_REFRESH_MS = 500L

internal fun restingValues(state: VirtualJoystickState): List<Double> =
    joystickValues(stickAxes(0.5f, restingY(state.autoCenterThrottle), state.leftPositiveOnly), stickAxes(0.5f, 0.5f, state.rightPositiveOnly), state.leftHandedMode)

object VirtualStickSender {
    @Volatile
    internal var latest: List<Double>? = null

    fun start(scope: kotlinx.coroutines.CoroutineScope) {
        scope.launch(Dispatchers.IO) {
            var state: VirtualJoystickState? = null
            var readAt = 0L
            while (isActive) {
                val now = System.currentTimeMillis()
                if (now - readAt >= STATE_REFRESH_MS) {
                    state = virtualJoystick(Qgc.get(VIRTUAL_JOYSTICK_PATH))
                    readAt = now
                    if (state?.show != true) latest = null
                }
                val current = state
                if (current != null && current.show && current.sending) {
                    Qgc.invoke(VIRTUAL_JOYSTICK_VALUE, *(latest ?: restingValues(current)).toTypedArray())
                }
                delay(current?.periodMs ?: DEFAULT_PERIOD_MS)
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

    var left by remember { mutableStateOf(Offset(0.5f, restingY(state.autoCenterThrottle))) }
    var right by remember { mutableStateOf(Offset(0.5f, 0.5f)) }
    LaunchedEffect(state.autoCenterThrottle) {
        left = Offset(0.5f, restingY(state.autoCenterThrottle))
    }
    val latestLeft by rememberUpdatedState(stickAxes(left.x, left.y, state.leftPositiveOnly))
    val latestRight by rememberUpdatedState(stickAxes(right.x, right.y, state.rightPositiveOnly))
    val latestState by rememberUpdatedState(state)

    LaunchedEffect(latestLeft, latestRight, latestState.leftHandedMode) {
        VirtualStickSender.latest = joystickValues(latestLeft, latestRight, latestState.leftHandedMode)
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
                    onMove(Offset(0.5f, if (reCenterY) 0.5f else latestStick.y))
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
