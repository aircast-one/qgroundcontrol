package one.aircast.android.ui

import androidx.compose.foundation.gestures.awaitEachGesture
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
    data object Pinched : Press
    data class Moved(val by: Offset) : Press
}

private suspend fun AwaitPointerEventScope.pressOutcome(pointer: PointerId, from: Offset, owned: Boolean): Press {
    while (true) {
        val event = awaitPointerEvent(PointerEventPass.Initial)
        if (event.changes.count { it.pressed } > 1) return Press.Pinched
        val change = event.changes.firstOrNull { it.id == pointer } ?: return Press.Released
        if (owned) change.consume()
        if (!change.pressed) return Press.Released
        val moved = change.position - from
        if (moved.getDistance() > viewConfiguration.touchSlop) return Press.Moved(moved)
    }
}

private tailrec suspend fun AwaitPointerEventScope.follow(pointer: PointerId, last: Offset, onMove: (Offset) -> Unit): Offset {
    val change = awaitPointerEvent(PointerEventPass.Initial).changes.firstOrNull { it.id == pointer } ?: return last
    change.consume()
    if (!change.pressed) return change.position
    onMove(change.position - change.previousPosition)
    return follow(pointer, change.position, onMove)
}

internal fun Modifier.videoGestures(handlers: VideoGestureHandlers): Modifier = pointerInput(Unit) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
        val owned = handlers.owned()
        if (owned) down.consume()
        val press = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) { pressOutcome(down.id, down.position, owned) }
        when {
            press is Press.Moved && !owned && handlers.claimsSwipe(press.by) ->
                handlers.onSwipe(follow(down.id, down.position + press.by) {} - down.position)
            press == null -> if (handlers.onHold(down.position)) {
                follow(down.id, down.position, handlers.onHoldDrag)
                handlers.onHoldEnd()
            }
            !owned || press == Press.Pinched -> Unit
            press == Press.Released -> {
                val again = withTimeoutOrNull(viewConfiguration.doubleTapTimeoutMillis) {
                    awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial).also { it.consume() }
                }
                if (again == null) {
                    handlers.onTap()
                } else {
                    follow(again.id, again.position) {}
                    handlers.onDoubleTap()
                }
            }
            press is Press.Moved -> handlers.onSwipe(follow(down.id, down.position + press.by) {} - down.position)
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

internal fun cameraSwipes(gimbalDrags: () -> Boolean, step: (Int) -> Unit, threshold: Float) = VideoGestureHandlers(
    owned = { false },
    claimsSwipe = { moved -> sideways(moved) && !gimbalDrags() },
    onTap = {},
    onDoubleTap = {},
    onSwipe = { moved ->
        when (videoSwipe(moved, threshold)) {
            VideoSwipe.Left -> step(1)
            VideoSwipe.Right -> step(-1)
            else -> Unit
        }
    },
    onHold = { false },
    onHoldDrag = {},
    onHoldEnd = {},
)
