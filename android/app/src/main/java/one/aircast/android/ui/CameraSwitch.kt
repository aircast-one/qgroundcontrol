package one.aircast.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.runtime.mutableStateOf
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.HorizontalDivider

private const val SWITCH_SCRIM_ALPHA = 0.55f
private const val SWITCH_BORDER_ALPHA = 0.4f
private val SWITCH_DOT = 8.dp
private val SWITCH_BORDER = 1.dp
private val CHIP_PAD_HORIZONTAL = 12.dp
private val CHIP_PAD_VERTICAL = 8.dp
private val CHIP_GAP = 6.dp
private val CHIP_LABEL_MAX = 96.dp
private val CHIP_GLYPH = 18.dp
private val CHIP_ARROW = 18.dp

internal data class CameraSwitchState(val shown: CameraEntry, val cameras: List<CameraEntry>)

internal fun cameraSwitchState(reading: CamerasReading?): CameraSwitchState? {
    val cameras = reading?.cameras.orEmpty().filter { it.active || it.problem == null }
    val shown = cameras.firstOrNull { it.active } ?: cameras.firstOrNull()
    return shown?.takeIf { cameras.size > 1 }?.let { CameraSwitchState(it, cameras) }
}

internal fun pipCamera(reading: CamerasReading?, streamOn: Boolean): CameraEntry? =
    reading?.pip?.takeIf { streamOn && it.enabled }?.slot?.let { slot -> reading.cameras.firstOrNull { it.slot == slot } }

internal fun splitToggleTarget(reading: CamerasReading?, room: Boolean, streamOn: Boolean): Boolean? =
    reading?.pip?.takeIf { streamOn && room && it.slot != null }?.let { !it.enabled }

@Composable
private fun streamOn(): Boolean = qgcDerived(VIDEO_VIEW) { videoReading(it)?.streamEnabled == true }.value

@Composable
internal fun cameraStatusTint(status: CameraStatus): Color = when (status) {
    CameraStatus.Live -> MaterialTheme.aircast.success
    CameraStatus.Connecting -> MaterialTheme.aircast.warning
    CameraStatus.NoSignal -> MaterialTheme.colorScheme.error
    CameraStatus.Idle -> MaterialTheme.colorScheme.outline
}

@Composable
internal fun CameraStatusDot(status: CameraStatus, modifier: Modifier = Modifier) {
    Box(modifier.size(SWITCH_DOT).background(cameraStatusTint(status), CircleShape).semantics { contentDescription = status.label })
}

internal fun showCamera(slot: Int, onRefused: () -> Unit = {}) = offMainInOrder { if (!VideoCommands.setActiveSource(slot)) onRefused() }

internal fun chipCamera(reading: CamerasReading?): CameraEntry? = reading?.cameras?.let { cameras -> cameras.firstOrNull { it.active } ?: cameras.firstOrNull() }

@Composable
internal fun CameraSwitch(splitRoom: Boolean, modifier: Modifier = Modifier) {
    val json by qgcPath(CAMERAS_VIEW)
    val reading = remember(json) { camerasReading(json) }
    val switch = cameraSwitchState(reading)
    val splitTo = splitToggleTarget(reading, splitRoom, streamOn())
    val shown = switch?.shown ?: chipCamera(reading)
    if ((switch == null && splitTo == null) || shown == null) return
    var open by remember { mutableStateOf(false) }
    val white = MaterialTheme.aircast.outdoorForeground
    Box(modifier) {
        Surface(
            onClick = { open = true },
            shape = CircleShape,
            color = osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
            contentColor = white,
            border = BorderStroke(SWITCH_BORDER, white.copy(alpha = SWITCH_BORDER_ALPHA)),
            modifier = Modifier.semantics { contentDescription = "Cameras, showing ${shown.title}" },
        ) {
            Row(
                Modifier.padding(horizontal = CHIP_PAD_HORIZONTAL, vertical = CHIP_PAD_VERTICAL),
                horizontalArrangement = Arrangement.spacedBy(CHIP_GAP),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                CameraStatusDot(shownCameraStatus(shown, showing = true, page = syntheticPage()))
                Text(shown.short, Modifier.widthIn(max = CHIP_LABEL_MAX), style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (splitTo == false) Icon(painterResource(R.drawable.ic_split_view), "Split view on", Modifier.size(CHIP_GLYPH))
                Icon(painterResource(R.drawable.ic_arrow_drop_down), null, Modifier.size(CHIP_ARROW))
            }
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            switch?.cameras?.map { camera ->
                val showing = camera.slot == shown.slot
                DropdownMenuItem(
                    text = { Text(camera.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    leadingIcon = { CameraStatusDot(shownCameraStatus(camera, showing, syntheticPage())) },
                    trailingIcon = if (showing) ({ Icon(Icons.Filled.Check, "Showing") }) else null,
                    onClick = {
                        open = false
                        if (!showing) showCamera(camera.slot)
                    },
                )
            }
            splitTo?.let { next ->
                if (switch != null) HorizontalDivider()
                DropdownMenuItem(
                    text = { Text("Split view") },
                    leadingIcon = { Icon(painterResource(R.drawable.ic_split_view), null, Modifier.size(CHIP_GLYPH)) },
                    trailingIcon = if (!next) ({ Icon(Icons.Filled.Check, "On") }) else null,
                    onClick = {
                        open = false
                        offMainInOrder { VideoCommands.setPictureInPicture(next) }
                    },
                    modifier = Modifier.semantics { role = Role.Switch },
                )
            }
        }
    }
}

@Composable
internal fun shownSecondCamera(): CameraEntry? {
    val streamOn = streamOn()
    return qgcDerived(CAMERAS_VIEW) { pipCamera(camerasReading(it), streamOn) }.value
}

internal fun neighbourCamera(state: CameraSwitchState?, step: Int): CameraEntry? =
    state?.cameras?.takeIf { it.size > 1 }?.let { cameras -> cameras[Math.floorMod(cameras.indexOf(state.shown) + step, cameras.size)] }

@Composable
internal fun rememberCameraSwiper(): (Int) -> Boolean {
    val json by qgcPath(CAMERAS_VIEW)
    val state by rememberUpdatedState(cameraSwitchState(camerasReading(json)))
    return remember { { step -> neighbourCamera(state, step)?.let { showCamera(it.slot); true } ?: false } }
}

@Composable
internal fun rememberGimbalDrags(): Boolean {
    val view by qgcPath(GIMBAL_INDICATOR_PATH)
    return remember(view) { onScreenGimbal(view) != null }
}
