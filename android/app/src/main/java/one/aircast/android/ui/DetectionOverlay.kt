package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast

private val BOX_COLOUR = Color(0xFF4DD0E1).copy(alpha = 0.55f)
private val TARGET_COLOUR = Color(0xFFFFD54F)
private const val REFUSAL_MS = 4000L
private const val SCRIM_ALPHA = 0.55f

@Composable
private fun ChaseOsd(pursuit: PursuitReading, modifier: Modifier = Modifier) {
    var confirming by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(refusal) {
        if (refusal != null) {
            delay(REFUSAL_MS)
            refusal = null
        }
    }
    Column(modifier, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        if (pursuit.engaged) {
            Button(
                onClick = { offMainDetached { Qgc.invoke(CAMERA_STOP_PURSUIT) } },
                modifier = Modifier.widthIn(min = 160.dp).heightIn(min = 48.dp).semantics { contentDescription = "Stop chasing the target" },
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                    contentColor = MaterialTheme.colorScheme.onError,
                ),
            ) { Text("Chasing — Stop", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, maxLines = 1) }
        } else {
            Button(
                onClick = { confirming = true },
                enabled = pursuit.ready,
                modifier = Modifier.widthIn(min = 160.dp).heightIn(min = 48.dp).semantics { contentDescription = "Chase the locked target" },
            ) { Text("Chase", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, maxLines = 1) }
            (refusal ?: pursuit.reason.takeIf { !pursuit.ready && it.isNotBlank() })?.let { why ->
                Text(
                    why,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.aircast.outdoorForeground,
                    modifier = Modifier.osdShadow().background(osdBackdrop(Color.Black.copy(alpha = SCRIM_ALPHA)), RoundedCornerShape(50)).padding(horizontal = 10.dp, vertical = 2.dp),
                )
            }
        }
    }
    if (confirming) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text("Chase the target?") },
            text = { Text("The aircraft will fly itself toward the locked target. Keep the area clear and be ready to take manual control.") },
            confirmButton = {
                TextButton(onClick = {
                    confirming = false
                    offMainDetached { refusal = Qgc.refusalOf(CAMERA_START_PURSUIT) }
                }) { Text("Chase", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { confirming = false }) { Text("Cancel") }
            },
        )
    }
}

@Composable
fun DetectionOverlay(modifier: Modifier = Modifier) {
    val json by qgcPath(DETECTIONS)
    val videoJson by qgcPath(VIDEO_VIEW)
    val reading = remember(json) { detections(json) }
    val boxes = remember(reading) { visibleBoxes(reading) }
    val trouble = remember(reading) { detectionTrouble(reading) }
    val pursuit = remember(json) { pursuitReading(json) }
    val locked = remember(boxes) { boxes.any { it.target } }
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
                    style = Stroke(width = if (box.target) 4f else 1.5f),
                )
            }
        }

        boxes.forEach { box ->
            val caption = box.caption
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

        if (pursuit != null && locked) {
            ChaseOsd(pursuit, Modifier.align(Alignment.TopCenter).padding(top = 12.dp))
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
