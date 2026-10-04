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

internal enum class NoVideoAction { None, SetUp, Settings }

internal data class NoVideoState(val title: String, val detail: String, val action: NoVideoAction)

internal fun unavailableVideoState(video: VideoReading): NoVideoState? = when {
    video.available -> null
    !video.sourceChosen -> NoVideoState("No video source", "Choose where the camera stream comes from.", NoVideoAction.SetUp)
    video.cameras.none { it.configured } -> NoVideoState("No stream address", "Enter the stream URL in Settings \u203a Video.", NoVideoAction.Settings)
    else -> NoVideoState(video.summary, "", NoVideoAction.Settings)
}

@Composable
internal fun NoVideoPanel(video: VideoReading?) {
    val flyJson by one.aircast.android.bridge.qgcPath(FLY_STATE)
    val armed = remember(flyJson) { flyState(flyJson)?.armed == true }
    val unavailable = video?.let(::unavailableVideoState)?.let { if (armed) it.copy(detail = "", action = NoVideoAction.None) else it }
    if (unavailable != null) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(unavailable.title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (unavailable.detail.isNotBlank()) Text(unavailable.detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (unavailable.action != NoVideoAction.None) {
                OutlinedButton(onClick = { AppNavigation.settingsPage = VIDEO_SETTINGS_PAGE }) {
                    Text(if (unavailable.action == NoVideoAction.SetUp) "Set up video" else "Video settings")
                }
            }
        }
        return
    }
    NoVideoStreamPanel(video)
}

@Composable
private fun NoVideoStreamPanel(video: VideoReading?) {
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
        if (video != null && !video.streamEnabled) {
            OutlinedButton(onClick = { AppNavigation.settingsPage = VIDEO_SETTINGS_PAGE }) { Text("Video settings") }
        }
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
