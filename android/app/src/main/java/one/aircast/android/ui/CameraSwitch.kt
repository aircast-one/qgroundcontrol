package one.aircast.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.window.PopupProperties
import one.aircast.android.R
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.aircast

private const val SWITCH_SCRIM_ALPHA = 0.55f
private const val SWITCH_BORDER_ALPHA = 0.4f
private val SWITCH_GLYPH = 18.dp
private val SWITCH_DOT = 8.dp
private val PIP_BUTTON_SIZE = 40.dp
private val PIP_BORDER = 2.dp
private val PIP_INSET = 4.dp

internal data class CameraSwitchState(val shown: CameraEntry, val cameras: List<CameraEntry>, val toggleTo: Int?)

internal fun cameraSwitchState(reading: CamerasReading?): CameraSwitchState? {
    val cameras = reading?.cameras.orEmpty().filter { it.active || it.problem == null }
    val shown = cameras.firstOrNull { it.active } ?: cameras.firstOrNull()
    return shown?.takeIf { cameras.size > 1 }?.let { CameraSwitchState(it, cameras, cameras.singleOrNull { other -> other.slot != it.slot }?.slot) }
}

internal fun pipCamera(reading: CamerasReading?): CameraEntry? =
    reading?.pip?.takeIf { it.enabled }?.slot?.let { slot -> reading.cameras.firstOrNull { it.slot == slot } }

internal fun pipToggleTarget(reading: CamerasReading?, thumbnailRoom: Boolean): Boolean? =
    reading?.pip?.takeIf { thumbnailRoom && it.slot != null }?.let { !it.enabled }

internal enum class MenuSide { Start, Above }

internal fun menuBeside(anchor: IntRect, window: IntSize, menu: IntSize, side: MenuSide, gap: Int, rtl: Boolean): IntOffset {
    val x = when (side) {
        MenuSide.Start -> if (rtl) anchor.right + gap else anchor.left - gap - menu.width
        MenuSide.Above -> if (rtl) anchor.left else anchor.right - menu.width
    }
    val y = when (side) {
        MenuSide.Start -> anchor.center.y - menu.height / 2
        MenuSide.Above -> anchor.top - gap - menu.height
    }
    return IntOffset(x.coerceAtMost(window.width - menu.width).coerceAtLeast(0), y.coerceAtMost(window.height - menu.height).coerceAtLeast(0))
}

private class BesideAnchor(private val side: MenuSide, private val gap: Int) : PopupPositionProvider {
    override fun calculatePosition(anchorBounds: IntRect, windowSize: IntSize, layoutDirection: LayoutDirection, popupContentSize: IntSize): IntOffset =
        menuBeside(anchorBounds, windowSize, popupContentSize, side, gap, layoutDirection == LayoutDirection.Rtl)
}

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

private fun showCamera(slot: Int) = offMainDetached { VideoCommands.setActiveSource(slot) }

@Composable
internal fun CameraSwitch(thumbnailRoom: Boolean, modifier: Modifier = Modifier) {
    val json by qgcPath(CAMERAS_VIEW)
    val reading = remember(json) { camerasReading(json) }
    val switch = cameraSwitchState(reading)
    val pipTo = pipToggleTarget(reading, thumbnailRoom)
    if (switch == null && pipTo == null) return
    val portrait = flyIsPortrait()
    val buttons: @Composable () -> Unit = {
        switch?.let { CameraSwitchButton(it, if (portrait) MenuSide.Above else MenuSide.Start) }
        pipTo?.let { next -> PipButton(shown = !next) { offMainDetached { VideoCommands.setPictureInPicture(next) } } }
    }
    if (portrait) {
        Row(modifier, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2), verticalAlignment = Alignment.CenterVertically) { buttons() }
    } else {
        Column(modifier, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(AircastSpace.s2)) { buttons() }
    }
}

@Composable
private fun CameraSwitchButton(state: CameraSwitchState, side: MenuSide) {
    var open by remember { mutableStateOf(false) }
    val white = MaterialTheme.aircast.outdoorForeground
    Box {
        Surface(
            onClick = { state.toggleTo?.let(::showCamera) ?: run { open = true } },
            shape = CircleShape,
            color = osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
            contentColor = white,
            border = BorderStroke(1.dp, white.copy(alpha = SWITCH_BORDER_ALPHA)),
            modifier = Modifier.semantics { contentDescription = "Switch camera, showing ${state.shown.title}" },
        ) {
            Row(
                Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(painterResource(R.drawable.ic_videocam), null, Modifier.size(SWITCH_GLYPH))
                Text(state.shown.short, style = MaterialTheme.typography.labelLarge, maxLines = 1)
                CameraStatusDot(state.shown.status)
            }
        }
        if (open) CameraMenu(state.cameras, side) { open = false }
    }
}

@Composable
private fun CameraMenu(cameras: List<CameraEntry>, side: MenuSide, onDismiss: () -> Unit) {
    val gap = with(LocalDensity.current) { AircastSpace.s2.roundToPx() }
    Popup(popupPositionProvider = remember(side, gap) { BesideAnchor(side, gap) }, onDismissRequest = onDismiss, properties = PopupProperties(focusable = true)) {
        Surface(shape = MenuDefaults.shape, color = MenuDefaults.containerColor, tonalElevation = MenuDefaults.TonalElevation, shadowElevation = MenuDefaults.ShadowElevation) {
            Column(Modifier.width(IntrinsicSize.Max).verticalScroll(rememberScrollState()).padding(vertical = 8.dp)) {
                cameras.map { camera ->
                    DropdownMenuItem(
                        text = { Text(camera.title, maxLines = 1) },
                        leadingIcon = { CameraStatusDot(camera.status) },
                        trailingIcon = if (camera.active) ({ Icon(Icons.Default.Check, "On screen") }) else null,
                        onClick = {
                            onDismiss()
                            if (!camera.active) showCamera(camera.slot)
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun PipButton(shown: Boolean, onToggle: () -> Unit) {
    val white = MaterialTheme.aircast.outdoorForeground
    Surface(
        onClick = onToggle,
        modifier = Modifier.size(PIP_BUTTON_SIZE),
        shape = CircleShape,
        color = if (shown) MaterialTheme.colorScheme.primary else osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
        contentColor = if (shown) MaterialTheme.colorScheme.onPrimary else white,
        border = if (shown) null else BorderStroke(1.dp, white.copy(alpha = SWITCH_BORDER_ALPHA)),
    ) {
        Box(contentAlignment = Alignment.Center) {
            Icon(painterResource(R.drawable.ic_picture_in_picture), if (shown) "Hide the second camera" else "Show a second camera", Modifier.size(20.dp))
        }
    }
}

@Composable
internal fun shownPipCamera(): CameraEntry? {
    val json by qgcPath(CAMERAS_VIEW)
    return remember(json) { pipCamera(camerasReading(json)) }
}

@Composable
internal fun CameraPipThumbnail(camera: CameraEntry, modifier: Modifier = Modifier, editor: @Composable () -> Unit = {}) {
    val shape = MaterialTheme.shapes.medium
    Box(modifier.clip(shape).background(MaterialTheme.aircast.outdoorBackground).border(PIP_BORDER, MaterialTheme.colorScheme.onSurface, shape)) {
        VideoChannelSurface(PIP_VIDEO_CHANNEL, Modifier.fillMaxSize().padding(PIP_INSET))
        Box(
            Modifier
                .matchParentSize()
                .then(if (camera.status == CameraStatus.Live) Modifier else Modifier.background(MaterialTheme.aircast.outdoorBackground))
                .clickable(onClickLabel = "Show ${camera.title} full screen") { showCamera(camera.slot) }
                .semantics { contentDescription = "Second camera, ${camera.title}" },
        )
        Row(
            Modifier.align(Alignment.TopStart).padding(6.dp).osdShadow(),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CameraStatusDot(camera.status)
            Text(camera.short, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.aircast.outdoorForeground, maxLines = 1)
        }
        editor()
    }
}
