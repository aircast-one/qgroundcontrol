package one.aircast.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector4D
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import kotlin.math.roundToInt
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitTouchSlopOrCancellation
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.pointerInput
import kotlin.math.abs

internal enum class VideoSwipe { Up, Down, Left, Right }

internal fun videoSwipe(moved: Offset, threshold: Float): VideoSwipe? = when {
    maxOf(abs(moved.x), abs(moved.y)) < threshold -> null
    abs(moved.y) > abs(moved.x) -> if (moved.y < 0f) VideoSwipe.Up else VideoSwipe.Down
    else -> if (moved.x < 0f) VideoSwipe.Left else VideoSwipe.Right
}

internal fun sideways(moved: Offset): Boolean = abs(moved.x) > abs(moved.y)

internal fun pipOnStart(centreX: Float, width: Float): Boolean = centreX < width / 2f

internal class VideoGestureHandlers(
    val owned: () -> Boolean,
    val claimsSwipe: (Offset) -> Boolean,
    val onTap: () -> Unit,
    val onDoubleTap: () -> Unit,
    val onSwipe: (Offset) -> Unit,
    val onHold: (Offset) -> Boolean,
    val onHoldDrag: (Offset) -> Unit,
    val onHoldEnd: () -> Unit,
)

private sealed interface Press {
    data object Released : Press
    data object Cancelled : Press
    data class Moved(val by: Offset) : Press
}

private suspend fun AwaitPointerEventScope.pressOutcome(pointer: PointerId, from: Offset, owned: Boolean, pass: PointerEventPass): Press {
    while (true) {
        val event = awaitPointerEvent(pass)
        if (event.changes.count { it.pressed } > 1) return Press.Cancelled
        val change = event.changes.firstOrNull { it.id == pointer } ?: return Press.Released
        if (change.isConsumed) return Press.Cancelled
        if (owned) change.consume()
        if (!change.pressed) return Press.Released
        val moved = change.position - from
        if (moved.getDistance() > viewConfiguration.touchSlop) return Press.Moved(moved)
    }
}

private tailrec suspend fun AwaitPointerEventScope.follow(pointer: PointerId, last: Offset, pass: PointerEventPass = PointerEventPass.Initial, onMove: (Offset) -> Unit): Offset {
    val change = awaitPointerEvent(pass).changes.firstOrNull { it.id == pointer } ?: return last
    change.consume()
    if (!change.pressed) return change.position
    onMove(change.position - change.previousPosition)
    return follow(pointer, change.position, pass, onMove)
}

internal fun Modifier.videoGestures(handlers: VideoGestureHandlers, pass: PointerEventPass = PointerEventPass.Initial): Modifier = pointerInput(Unit) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false, pass = pass)
        if (down.isConsumed) return@awaitEachGesture
        val owned = handlers.owned()
        if (owned) down.consume()
        val press = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) { pressOutcome(down.id, down.position, owned, pass) }
        when {
            press is Press.Moved && !owned && handlers.claimsSwipe(press.by) ->
                handlers.onSwipe(follow(down.id, down.position + press.by, pass) {} - down.position)
            press == null -> if (handlers.onHold(down.position)) {
                follow(down.id, down.position, pass, handlers.onHoldDrag)
                handlers.onHoldEnd()
            }
            press == Press.Cancelled -> Unit
            press == Press.Released && !owned -> withTimeoutOrNull(viewConfiguration.doubleTapTimeoutMillis) {
                awaitFirstDown(requireUnconsumed = false, pass = pass)
            }?.let { again ->
                again.consume()
                follow(again.id, again.position, pass) {}
                handlers.onDoubleTap()
            }
            !owned -> Unit
            press == Press.Released -> {
                val again = withTimeoutOrNull(viewConfiguration.doubleTapTimeoutMillis) {
                    awaitFirstDown(requireUnconsumed = false, pass = pass).also { it.consume() }
                }
                if (again == null) {
                    handlers.onTap()
                } else {
                    follow(again.id, again.position, pass) {}
                    handlers.onDoubleTap()
                }
            }
            press is Press.Moved -> handlers.onSwipe(follow(down.id, down.position + press.by, pass) {} - down.position)
        }
    }
}

internal fun Modifier.verticalSwipe(threshold: Float, onSwipe: (VideoSwipe) -> Unit): Modifier = pointerInput(Unit) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        awaitTouchSlopOrCancellation(down.id) { change, _ -> change.consume() }
            ?.let { moving -> videoSwipe(follow(moving.id, moving.position) {} - down.position, threshold)?.let(onSwipe) }
    }
}

internal val CAMERA_NUDGE = 48.dp
internal val VIDEO_SWIPE_DISTANCE = 32.dp

@Composable
internal fun rememberVideoFrame(target: Rect, holding: Boolean): Animatable<Rect, AnimationVector4D> {
    val frame = remember { Animatable(target, Rect.VectorConverter) }
    LaunchedEffect(target, holding) {
        if (holding || frame.value.isEmpty) frame.snapTo(target) else frame.animateTo(target, spring(stiffness = Spring.StiffnessMediumLow))
    }
    return frame
}

internal fun Modifier.videoFrame(target: Rect, shown: () -> Rect, nudge: () -> Float): Modifier = this
    .layout { measurable, _ ->
        val placeable = measurable.measure(Constraints.fixed(target.width.roundToInt().coerceAtLeast(0), target.height.roundToInt().coerceAtLeast(0)))
        layout(target.right.roundToInt().coerceAtLeast(0), target.bottom.roundToInt().coerceAtLeast(0)) { placeable.place(target.left.roundToInt(), target.top.roundToInt()) }
    }
    .graphicsLayer {
        val now = shown()
        transformOrigin = TransformOrigin(0f, 0f)
        scaleX = if (target.width > 0f) now.width / target.width else 1f
        scaleY = if (target.height > 0f) now.height / target.height else 1f
        translationX = now.left - target.left + nudge()
        translationY = now.top - target.top
    }

internal class CameraStepper(val step: (Int) -> Unit, val nudge: () -> Float)

@Composable
internal fun rememberCameraStepper(): CameraStepper {
    val swipeCamera = rememberCameraSwiper()
    val nudge = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val distance = with(LocalDensity.current) { CAMERA_NUDGE.toPx() }
    return remember {
        CameraStepper(
            step = { step ->
                if (swipeCamera(step)) scope.launch {
                    nudge.snapTo(distance * step)
                    nudge.animateTo(0f, spring(stiffness = Spring.StiffnessMediumLow))
                }
            },
            nudge = { nudge.value },
        )
    }
}

internal fun cameraStep(swipe: VideoSwipe?): Int? = when (swipe) {
    VideoSwipe.Left -> 1
    VideoSwipe.Right -> -1
    else -> null
}
