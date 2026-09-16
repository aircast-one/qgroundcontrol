package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import one.aircast.android.bridge.qgcPath

private val TRACK_COLOUR = Color(0xFF00E5FF)

@Composable
fun TrackingBoxOverlay(modifier: Modifier = Modifier) {
    val cameraJson by qgcPath(CAMERA_VIEW)
    val videoJson by qgcPath(VIDEO_VIEW)
    val tracking = remember(cameraJson) { trackingReading(cameraJson) }
    val source = remember(videoJson) { videoReading(videoJson)?.sourceSize }
    val box = tracking?.box

    if (box == null) {
        return
    }

    BoxWithConstraints(modifier) {
        val picture = paintedRect(maxWidth.value.toDouble(), maxHeight.value.toDouble(), source)
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
