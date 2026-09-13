package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

private val BOX_COLOUR = Color(0xFF4DD0E1)
private val TARGET_COLOUR = Color(0xFFFFD54F)

@Composable
fun DetectionOverlay(modifier: Modifier = Modifier) {
    val json by qgcPath(DETECTIONS)
    val videoJson by qgcPath(VIDEO_VIEW)
    val reading = remember(json) { detections(json) }
    val boxes = remember(reading) { visibleBoxes(reading) }
    val trouble = remember(reading) { detectionTrouble(reading) }
    val source = remember(videoJson) { videoReading(videoJson)?.sourceSize }

    BoxWithConstraints(modifier) {
        val picture = paintedRect(maxWidth.value.toDouble(), maxHeight.value.toDouble(), source)

        Canvas(Modifier.matchParentSize()) {
            val scale = size.width / maxWidth.value
            boxes.forEach { box ->
                drawRect(
                    color = if (box.target) TARGET_COLOUR else BOX_COLOUR,
                    topLeft = Offset(
                        ((picture.left + box.x * picture.width) * scale).toFloat(),
                        ((picture.top + box.y * picture.height) * scale).toFloat(),
                    ),
                    size = Size(
                        (box.w * picture.width * scale).toFloat(),
                        (box.h * picture.height * scale).toFloat(),
                    ),
                    style = Stroke(width = if (box.target) 4f else 2f),
                )
            }
        }

        boxes.forEach { box ->
            val caption = boxCaption(box)
            if (caption.isNotBlank()) {
                Text(
                    text = caption,
                    style = MaterialTheme.typography.labelSmall,
                    color = Color.Black,
                    modifier = Modifier
                        .offset(
                            x = (picture.left + box.x * picture.width).dp,
                            y = (picture.top + box.y * picture.height).dp,
                        )
                        .background(if (box.target) TARGET_COLOUR else BOX_COLOUR)
                        .padding(horizontal = 3.dp),
                )
            }
        }

        trouble?.let { reason ->
            Text(
                text = "Detections: $reason",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onErrorContainer,
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .background(MaterialTheme.colorScheme.errorContainer)
                    .padding(horizontal = 4.dp, vertical = 2.dp),
            )
        }
    }
}
