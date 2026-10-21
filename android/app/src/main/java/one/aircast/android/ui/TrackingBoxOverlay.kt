package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private val TRACK_COLOUR = Color(0xFF00E5FF)
private const val HINT_SCRIM_ALPHA = 0.55f
private val HINT_TOP = 56.dp

@Composable
fun TrackingBoxOverlay(modifier: Modifier = Modifier) {
    val cameraJson by qgcPath(CAMERA_VIEW)
    val videoJson by qgcPath(VIDEO_VIEW)
    val tracking = remember(cameraJson) { trackingReading(cameraJson) }
    val autodropJson by qgcPath(AUTODROP_STATUS)
    val running = autodropJson?.optString("state") == "waiting"
    val source = remember(videoJson) { videoReading(videoJson)?.sourceSize }
    val box = tracking?.box
    val density = LocalDensity.current

    if (tracking == null) {
        return
    }

    BoxWithConstraints(modifier) {
        val picture = paintedRect(maxWidth.value.toDouble(), maxHeight.value.toDouble(), source)
        val aim = if (!trackingCanAim(tracking)) {
            Modifier
        } else {
            Modifier.pointerInput(picture) {
                detectTapGestures { at ->
                    val (x, y) = with(density) { at.x.toDp().value.toDouble() to at.y.toDp().value.toDouble() }
                    sendTracking(trackingRequest(x, y, x, y, picture))
                }
            }.pointerInput(picture) {
                var pressed = Offset.Zero
                var latest = Offset.Zero
                detectDragGestures(
                    onDragStart = { at ->
                        pressed = at
                        latest = at
                    },
                    onDragEnd = {
                        val from = with(density) { pressed.x.toDp().value.toDouble() to pressed.y.toDp().value.toDouble() }
                        val to = with(density) { latest.x.toDp().value.toDouble() to latest.y.toDp().value.toDouble() }
                        sendTracking(trackingRequest(from.first, from.second, to.first, to.second, picture))
                    },
                ) { change, _ -> latest = change.position }
            }
        }
        Box(aim.matchParentSize())
        trackingHint(tracking)?.takeIf { !running }?.let { hint ->
            Surface(
                shape = CircleShape,
                color = Color.Black.copy(alpha = HINT_SCRIM_ALPHA),
                contentColor = Color.White,
                modifier = Modifier.align(Alignment.TopCenter).padding(top = HINT_TOP),
            ) { Text(hint, Modifier.padding(horizontal = 14.dp, vertical = 8.dp), style = MaterialTheme.typography.labelLarge) }
        }
        if (box == null) {
            return@BoxWithConstraints
        }
        Canvas(Modifier.matchParentSize()) {
            val scale = size.width / maxWidth.value
            drawRect(
                color = TRACK_COLOUR,
                topLeft = Offset(
                    ((picture.left + box.x * picture.width) * scale).toFloat(),
                    ((picture.top + box.y * picture.height) * scale).toFloat(),
                ),
                size = Size(
                    (box.width * picture.width * scale).toFloat(),
                    (box.height * picture.height * scale).toFloat(),
                ),
                style = Stroke(width = 4f),
            )
        }
    }
}

private fun sendTracking(request: TrackingRequest?) {
    when (request) {
        null -> Unit
        is TrackingRequest.Box -> offMainDetached {
            Qgc.invoke(CAMERA_START_TRACKING, trackingRectObject(request.rect))
        }
        is TrackingRequest.Point -> offMainDetached {
            Qgc.invoke(CAMERA_START_TRACKING, trackingPointObject(request), request.radius)
        }
    }
}
