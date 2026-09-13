package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

@Composable
fun VideoSourceLayer(modifier: Modifier = Modifier) {
    val videoJson by qgcPath(VIDEO_VIEW)
    val video = remember(videoJson) { videoReading(videoJson) }
    val sources = remember(video) { switchableSources(video) }

    if (sources.isEmpty()) {
        return
    }

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
    ) {
        Row(
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            sources.forEach { source ->
                FilterChip(
                    selected = source.slot == video?.activeSource,
                    onClick = {
                        offMainDetached { Qgc.invoke("video.setActiveVideoSource", source.slot) }
                    },
                    label = { Text(if (source.connecting) "${source.title} · connecting" else source.title) },
                )
            }
        }
    }
}
