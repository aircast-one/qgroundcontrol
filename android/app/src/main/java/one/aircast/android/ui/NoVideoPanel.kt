package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import kotlinx.coroutines.delay
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

private const val PROLONGED_SECONDS = 8
private val PANEL_MAX_WIDTH = 360.dp
private val VIDEO_SOURCES_PLACE = "Settings \u203a ${pageLook(VIDEO_SOURCES_PAGE).group.title} \u203a ${pageTitle(VIDEO_SOURCES_PAGE)}"

internal fun elapsedText(seconds: Int): String = when {
    seconds < 60 -> "$seconds s"
    seconds < 3600 -> "${seconds / 60} min"
    else -> "${seconds / 3600} h ${(seconds % 3600) / 60} min"
}

internal fun activeCameraStatus(video: VideoReading): String? =
    video.cameras.firstOrNull { it.slot == video.activeSource }?.status?.ifBlank { null }

internal fun noVideoDetail(video: VideoReading, seconds: Int): String = when {
    video.noVideoReason.isNotBlank() -> video.noVideoReason
    video.streaming -> "Receiving data \u2014 waiting for video for ${elapsedText(seconds)}"
    else -> "${video.noVideoText} for ${elapsedText(seconds)}"
}

internal enum class NoVideoAction { None, SetUp, Settings, TurnOn }

internal data class NoVideoState(val title: String, val detail: String, val action: NoVideoAction)

internal fun unavailableVideoState(video: VideoReading): NoVideoState? = when {
    video.available -> null
    !video.streamEnabled -> NoVideoState("Video off", "It turns back on when you arm.", NoVideoAction.TurnOn)
    !video.sourceChosen -> NoVideoState("No video source", "Add a camera in $VIDEO_SOURCES_PLACE.", NoVideoAction.SetUp)
    video.cameras.none { it.configured } -> NoVideoState("No stream address", "Enter the stream address in $VIDEO_SOURCES_PLACE.", NoVideoAction.Settings)
    else -> NoVideoState(video.summary, "", NoVideoAction.Settings)
}

internal fun whileArmed(state: NoVideoState): NoVideoState =
    state.copy(detail = "", action = state.action.takeIf { it == NoVideoAction.TurnOn } ?: NoVideoAction.None)

internal data class NoVideoButton(val label: String, val onClick: () -> Unit)

internal fun videoSourcesButton(navigation: AppNavigationState) = NoVideoButton("Video sources") { navigation.settingsPage = VIDEO_SOURCES_PAGE }

private val TURN_VIDEO_ON = NoVideoButton("Turn video on") { offMainDetached { VideoCommands.turnStreamOn() } }

internal enum class NoVideoSize { Full, Pill, Thumb }

internal val LocalNoVideoSize = staticCompositionLocalOf { NoVideoSize.Full }

@Composable
private fun NoVideoLayout(title: String, detail: String = "", primary: NoVideoButton? = null, secondary: NoVideoButton? = null, compactTitle: String = title) {
    if (LocalNoVideoSize.current == NoVideoSize.Thumb) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Icon(painterResource(R.drawable.ic_videocam_off), contentDescription = null, modifier = Modifier.size(24.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(compactTitle, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        return
    }
    if (LocalNoVideoSize.current == NoVideoSize.Pill) {
        Surface(shape = MaterialTheme.shapes.extraLarge, color = osdBackdrop(MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Row(Modifier.padding(start = 12.dp, end = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Icon(painterResource(R.drawable.ic_videocam_off), contentDescription = null, modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(syntheticPillTitle(compactTitle, LocalSyntheticBehind.current), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f, fill = false).padding(vertical = 12.dp))
                primary?.let { TextButton(onClick = it.onClick) { Text(it.label, maxLines = 1, softWrap = false) } }
            }
        }
        return
    }
    Column(
        Modifier.widthIn(max = PANEL_MAX_WIDTH).padding(horizontal = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Icon(painterResource(R.drawable.ic_videocam_off), contentDescription = null, modifier = Modifier.size(40.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface, textAlign = TextAlign.Center)
        if (detail.isNotBlank()) Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        if (primary != null || secondary != null) {
            Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                primary?.let { FilledTonalButton(onClick = it.onClick) { Text(it.label) } }
                secondary?.let { TextButton(onClick = it.onClick) { Text(it.label) } }
            }
        }
    }
}

@Composable
internal fun NoVideoPanel(video: VideoReading?) {
    val videoSources = videoSourcesButton(LocalAppNavigation.current)
    val flyJson by qgcPath(FLY_STATE)
    val armed = remember(flyJson) { flyState(flyJson)?.armed == true }
    val unavailable = video?.let { unavailableVideoState(it) }?.let { if (armed) whileArmed(it) else it }
    if (unavailable != null) {
        NoVideoLayout(
            title = unavailable.title,
            detail = unavailable.detail,
            primary = when (unavailable.action) {
                NoVideoAction.None -> null
                NoVideoAction.SetUp -> videoSources.copy(label = "Set up video")
                NoVideoAction.Settings -> videoSources
                NoVideoAction.TurnOn -> TURN_VIDEO_ON
            },
        )
        return
    }
    NoVideoStreamPanel(video, videoSources)
}

@Composable
private fun NoVideoStreamPanel(video: VideoReading?, videoSources: NoVideoButton) {
    var seconds by remember { mutableIntStateOf(0) }
    LaunchedEffect(video != null, video?.decoding) {
        seconds = 0
        while (video != null) {
            delay(1000)
            seconds++
        }
    }
    when {
        video == null -> NoVideoLayout("No video")
        seconds >= PROLONGED_SECONDS -> NoVideoLayout(
            title = "No video signal",
            detail = noVideoDetail(video, seconds),
            primary = NoVideoButton("Retry") {
                seconds = 0
                offMainDetached { VideoCommands.restart() }
            },
            secondary = videoSources,
            compactTitle = "No video",
        )
        else -> NoVideoLayout(activeCameraStatus(video) ?: video.summary)
    }
}
