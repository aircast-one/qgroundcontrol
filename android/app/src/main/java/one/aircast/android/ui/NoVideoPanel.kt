package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached

private const val PROLONGED_SECONDS = 8
private const val VIDEO_RESTART = "video.restart"
private const val VIDEO_SETTINGS_PAGE = "Video"

internal fun elapsedText(seconds: Int): String = when {
    seconds < 60 -> "$seconds s"
    seconds < 3600 -> "${seconds / 60} min"
    else -> "${seconds / 3600} h ${(seconds % 3600) / 60} min"
}

internal fun activeCameraStatus(video: VideoReading): String? =
    video.cameras.firstOrNull { it.slot == video.activeSource }?.status?.ifBlank { null }

internal fun noVideoDetail(video: VideoReading, seconds: Int): String =
    "${if (video.streaming) "Receiving data \u2014 waiting for video" else video.noVideoText} for ${elapsedText(seconds)}"

@Composable
internal fun NoVideoPanel(video: VideoReading?) {
    var seconds by remember { mutableIntStateOf(0) }
    LaunchedEffect(video?.streamEnabled, video?.decoding) {
        seconds = 0
        while (video?.streamEnabled == true) {
            delay(1000)
            seconds++
        }
    }
    val prolonged = video != null && video.streamEnabled && seconds >= PROLONGED_SECONDS
    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(
            text = when {
                video == null -> "No video"
                !video.streamEnabled -> "Video off"
                prolonged -> "No video signal"
                else -> activeCameraStatus(video) ?: video.summary
            },
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (prolonged && video != null) {
            Text(noVideoDetail(video, seconds), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = {
                    seconds = 0
                    offMainDetached { Qgc.invoke(VIDEO_RESTART) }
                }) { Text("Retry") }
                OutlinedButton(onClick = { AppNavigation.settingsPage = VIDEO_SETTINGS_PAGE }) { Text("Video settings") }
            }
        }
    }
}
