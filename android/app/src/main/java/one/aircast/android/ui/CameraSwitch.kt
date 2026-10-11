package one.aircast.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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
import one.aircast.map.AircastSpace
import one.aircast.map.aircast

private const val SWITCH_SCRIM_ALPHA = 0.55f
private const val SWITCH_BORDER_ALPHA = 0.4f
private val SWITCH_DOT = 8.dp
private val SWITCH_BORDER = 1.dp
private val SEGMENT_PAD_HORIZONTAL = 10.dp
private val SEGMENT_PAD_VERTICAL = 6.dp
private val SEGMENT_GAP = 6.dp
private val SEGMENT_LABEL_MAX = 72.dp
private val SPLIT_BUTTON_SIZE = 40.dp
private val SPLIT_GLYPH = 20.dp
private val SEGMENT_INSET = 6.dp
private val MENU_ARROW = 18.dp
internal const val MAX_SEGMENTS = 3

internal data class CameraSwitchState(val shown: CameraEntry, val cameras: List<CameraEntry>)

internal fun cameraSwitchState(reading: CamerasReading?): CameraSwitchState? {
    val cameras = reading?.cameras.orEmpty().filter { it.active || it.problem == null }
    val shown = cameras.firstOrNull { it.active } ?: cameras.firstOrNull()
    return shown?.takeIf { cameras.size > 1 }?.let { CameraSwitchState(it, cameras) }
}

internal fun pipCamera(reading: CamerasReading?, streamOn: Boolean): CameraEntry? =
    reading?.pip?.takeIf { streamOn && it.enabled }?.slot?.let { slot -> reading.cameras.firstOrNull { it.slot == slot } }

internal fun pickedFromMenu(state: CameraSwitchState): Boolean = state.cameras.size > MAX_SEGMENTS

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

@Composable
internal fun CameraSwitch(splitRoom: Boolean, modifier: Modifier = Modifier) {
    val portrait = flyIsPortrait()
    val json by qgcPath(CAMERAS_VIEW)
    val reading = remember(json) { camerasReading(json) }
    val switch = cameraSwitchState(reading)
    val splitTo = splitToggleTarget(reading, splitRoom, streamOn())
    if (switch == null && splitTo == null) return
    val parts: @Composable () -> Unit = {
        switch?.let { CameraSegments(it, portrait) }
        splitTo?.let { next -> SplitButton(shown = !next) { offMainInOrder { VideoCommands.setPictureInPicture(next) } } }
    }
    if (portrait) {
        Row(modifier, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2), verticalAlignment = Alignment.CenterVertically) { parts() }
    } else {
        Column(modifier, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(AircastSpace.s2)) { parts() }
    }
}

@Composable
private fun CameraSegments(state: CameraSwitchState, across: Boolean) {
    val white = MaterialTheme.aircast.outdoorForeground
    val segments: @Composable () -> Unit = {
        state.cameras.map { camera -> CameraSegment(camera, selected = camera.slot == state.shown.slot) }
    }
    Surface(
        shape = MaterialTheme.shapes.large,
        color = osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
        contentColor = white,
        border = BorderStroke(SWITCH_BORDER, white.copy(alpha = SWITCH_BORDER_ALPHA)),
        modifier = Modifier.semantics { contentDescription = "Cameras, showing ${state.shown.title}" },
    ) {
        when {
            pickedFromMenu(state) -> CameraMenu(state)
            across -> Row(Modifier.padding(horizontal = SEGMENT_INSET)) { segments() }
            else -> Column(Modifier.verticalScroll(rememberScrollState()).padding(vertical = SEGMENT_INSET)) { segments() }
        }
    }
}

@Composable
private fun CameraMenu(state: CameraSwitchState) {
    var open by remember { mutableStateOf(false) }
    Box(Modifier.padding(horizontal = SEGMENT_INSET)) {
        CameraSegment(state.shown, selected = true, menu = true)
        Box(Modifier.matchParentSize().clickable(onClickLabel = "Choose a camera") { open = true })
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            state.cameras.map { camera ->
                DropdownMenuItem(
                    text = { Text(camera.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    leadingIcon = { CameraStatusDot(camera.status) },
                    onClick = {
                        open = false
                        if (camera.slot != state.shown.slot) showCamera(camera.slot)
                    },
                )
            }
        }
    }
}

@Composable
private fun CameraSegment(camera: CameraEntry, selected: Boolean, menu: Boolean = false) {
    val white = MaterialTheme.aircast.outdoorForeground
    Box(
        Modifier
            .then(if (menu) Modifier else Modifier.selectable(selected = selected, role = Role.Tab, onClick = { if (!selected) showCamera(camera.slot) }))
            .minimumInteractiveComponentSize(),
        contentAlignment = Alignment.Center,
    ) {
        Row(
            Modifier
                .clip(MaterialTheme.shapes.large)
                .background(if (selected) white else Color.Transparent)
                .padding(horizontal = SEGMENT_PAD_HORIZONTAL, vertical = SEGMENT_PAD_VERTICAL),
            horizontalArrangement = Arrangement.spacedBy(SEGMENT_GAP),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CameraStatusDot(camera.status)
            Text(
                camera.short,
                Modifier.widthIn(max = SEGMENT_LABEL_MAX),
                style = MaterialTheme.typography.labelLarge,
                color = if (selected) Color.Black else white,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (menu) Icon(painterResource(R.drawable.ic_arrow_drop_down), contentDescription = null, tint = Color.Black, modifier = Modifier.size(MENU_ARROW))
        }
    }
}

@Composable
private fun SplitButton(shown: Boolean, onToggle: () -> Unit) {
    val white = MaterialTheme.aircast.outdoorForeground
    Surface(
        checked = shown,
        onCheckedChange = { onToggle() },
        modifier = Modifier.size(SPLIT_BUTTON_SIZE).semantics { role = Role.Switch },
        shape = CircleShape,
        color = if (shown) MaterialTheme.colorScheme.primary else osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
        contentColor = if (shown) MaterialTheme.colorScheme.onPrimary else white,
        border = if (shown) null else BorderStroke(SWITCH_BORDER, white.copy(alpha = SWITCH_BORDER_ALPHA)),
    ) {
        Box(contentAlignment = Alignment.Center) {
            Icon(painterResource(R.drawable.ic_split_view), "Show two cameras side by side", Modifier.size(SPLIT_GLYPH))
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
