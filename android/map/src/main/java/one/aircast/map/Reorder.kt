package one.aircast.map

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import kotlinx.coroutines.withTimeoutOrNull

fun Modifier.holdToReorder(
    key: Any?,
    holdMillis: Long? = null,
    onLift: () -> Unit = {},
    onDrag: (Offset) -> Unit,
    onEnd: (completed: Boolean) -> Unit,
): Modifier = pointerInput(key) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        val released = withTimeoutOrNull(holdMillis ?: viewConfiguration.longPressTimeoutMillis) {
            waitForUpOrCancellation()
            true
        }
        if (released == null) {
            onLift()
            val completed = drag(down.id) { change ->
                onDrag(change.positionChange())
                change.consume()
            }
            onEnd(completed)
        }
    }
}
