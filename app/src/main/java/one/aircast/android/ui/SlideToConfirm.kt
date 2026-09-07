package one.aircast.android.ui

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt

internal const val SLIDE_CONFIRM_FRACTION = 0.9f

private val TRACK_HEIGHT = 64.dp
private val THUMB_SIZE = 56.dp

internal fun slideFraction(offsetPx: Float, trackPx: Int, thumbPx: Float): Float {
    val travel = trackPx - thumbPx
    if (travel <= 0f) return 0f
    return (offsetPx / travel).coerceIn(0f, 1f)
}

internal fun slideConfirms(fraction: Float): Boolean = fraction >= SLIDE_CONFIRM_FRACTION

@Composable
fun SlideToConfirm(
    label: String,
    modifier: Modifier = Modifier,
    destructive: Boolean = false,
    onConfirm: () -> Unit,
) {
    var offsetPx by remember(label) { mutableFloatStateOf(0f) }
    var trackPx by remember(label) { mutableIntStateOf(0) }
    var settled by remember(label) { mutableStateOf(false) }
    val thumbPx = with(LocalDensity.current) { THUMB_SIZE.toPx() }
    val fraction = slideFraction(offsetPx, trackPx, thumbPx)
    val shown by animateFloatAsState(if (settled) 0f else offsetPx, label = "slide")

    val accent = if (destructive) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary

    Box(
        modifier
            .fillMaxWidth()
            .height(TRACK_HEIGHT)
            .clip(CircleShape)
            .background(accent.copy(alpha = 0.15f + 0.35f * fraction))
            .onSizeChanged { trackPx = it.width },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 1f - fraction),
        )
        Box(
            Modifier
                .align(Alignment.CenterStart)
                .offset { IntOffset(shown.roundToInt(), 0) }
                .padding(4.dp)
                .size(THUMB_SIZE)
                .clip(CircleShape)
                .background(accent)
                .draggable(
                    orientation = Orientation.Horizontal,
                    state = rememberDraggableState { delta ->
                        settled = false
                        offsetPx = (offsetPx + delta).coerceIn(0f, (trackPx - thumbPx).coerceAtLeast(0f))
                    },
                    onDragStopped = {
                        if (slideConfirms(slideFraction(offsetPx, trackPx, thumbPx))) {
                            onConfirm()
                        }
                        offsetPx = 0f
                        settled = true
                    },
                ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.KeyboardArrowRight,
                contentDescription = null,
                tint = Color.White,
            )
        }
    }
}
