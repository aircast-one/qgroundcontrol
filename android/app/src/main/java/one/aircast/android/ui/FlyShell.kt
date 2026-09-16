package one.aircast.android.ui

import android.content.res.Configuration
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.unit.dp

private val KEY_ROW_END_GAP = 16.dp

internal const val VIDEO_BAND_RATIO = 16f / 9f

@Composable
internal fun flyIsPortrait(): Boolean =
    LocalConfiguration.current.orientation == Configuration.ORIENTATION_PORTRAIT

@Composable
internal fun FlyPortrait(
    videoExpanded: Boolean,
    onSwap: () -> Unit,
    controlsExpanded: Boolean,
    onToggleControls: () -> Unit,
    controllable: Boolean,
    video: @Composable (Modifier, Boolean) -> Unit,
    map: @Composable (Modifier) -> Unit,
    keyRow: @Composable () -> Unit,
    keyRowEnd: @Composable () -> Unit,
    videoAiming: Boolean,
    overlays: @Composable () -> Unit,
    actions: @Composable () -> Unit,
) {
    Column(Modifier.fillMaxSize()) {
        Box(
            Modifier
                .fillMaxWidth()
                .aspectRatio(VIDEO_BAND_RATIO),
        ) {
            if (videoExpanded) {
                video(Modifier.fillMaxSize(), true)
            } else {
                map(Modifier.fillMaxSize())
                Box(Modifier.fillMaxSize().clickable { onSwap() })
            }
        }

        Surface(color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f)) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    Modifier.weight(1f).horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) { keyRow() }
                Box(Modifier.padding(start = KEY_ROW_END_GAP)) { keyRowEnd() }
            }
        }

        Box(
            Modifier
                .fillMaxWidth()
                .weight(1f),
        ) {
            if (videoExpanded) {
                map(Modifier.fillMaxSize())
                Box(Modifier.fillMaxSize().clickable { onSwap() })
            } else {
                video(Modifier.fillMaxSize(), false)
                if (!videoAiming) {
                    Box(Modifier.fillMaxSize().clickable { onSwap() })
                }
            }
            Column(
                Modifier
                    .align(Alignment.TopStart)
                    .padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) { overlays() }
        }

        Column(Modifier.fillMaxWidth()) {
            if (controllable) {
                Surface(
                    Modifier.align(Alignment.CenterHorizontally),
                    color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                    shape = MaterialTheme.shapes.small,
                ) {
                    IconButton(onClick = onToggleControls) {
                        Icon(
                            if (controlsExpanded) Icons.Default.KeyboardArrowDown else Icons.Default.KeyboardArrowUp,
                            if (controlsExpanded) "Hide flight controls" else "Show flight controls",
                        )
                    }
                }
            }
            AnimatedVisibility(visible = controlsExpanded || !controllable) {
                Surface(
                    Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                ) { actions() }
            }
        }
    }
}
