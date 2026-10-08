package one.aircast.android.ui

import android.os.SystemClock
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
import androidx.compose.foundation.layout.widthIn
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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
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
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.aircast

private const val SWITCH_SCRIM_ALPHA = 0.55f
private const val SWITCH_BORDER_ALPHA = 0.4f
private const val SWITCH_DOUBLE_TAP_MS = 300L
private const val SWITCH_SETTLE_MS = 2000L
private val SWITCH_GLYPH = 18.dp
private val SWITCH_DOT = 8.dp
private val SWITCH_BORDER = 1.dp
private val SWITCH_PAD_HORIZONTAL = 10.dp
private val SWITCH_PAD_VERTICAL = 6.dp
private val SWITCH_GAP = 6.dp
private val SWITCH_LABEL_MAX = 72.dp
private val PIP_BUTTON_SIZE = 40.dp
private val PIP_GLYPH = 20.dp
private val PIP_BORDER = 2.dp
private val PIP_INSET = 4.dp
private val PIP_LABEL_PAD = 6.dp

internal data class CameraSwitchState(val shown: CameraEntry, val cameras: List<CameraEntry>, val toggleTo: Int?)

internal data class SwitchTap(val from: Int, val atMs: Long)

internal fun switchTapAllowed(last: SwitchTap?, shown: Int, nowMs: Long): Boolean =
    last == null || (nowMs - last.atMs).let { since -> since >= SWITCH_SETTLE_MS || (last.from != shown && since >= SWITCH_DOUBLE_TAP_MS) }

internal fun cameraSwitchState(reading: CamerasReading?): CameraSwitchState? {
    val cameras = reading?.cameras.orEmpty().filter { it.active || it.problem == null }
    val shown = cameras.firstOrNull { it.active } ?: cameras.firstOrNull()
    return shown?.takeIf { cameras.size > 1 }?.let { CameraSwitchState(it, cameras, cameras.singleOrNull { other -> other.slot != it.slot }?.slot) }
}

internal fun pipCamera(reading: CamerasReading?, streamOn: Boolean): CameraEntry? =
    reading?.pip?.takeIf { streamOn && it.enabled }?.slot?.let { slot -> reading.cameras.firstOrNull { it.slot == slot } }

internal fun pipToggleTarget(reading: CamerasReading?, thumbnailRoom: Boolean, streamOn: Boolean): Boolean? =
    reading?.pip?.takeIf { streamOn && thumbnailRoom && it.slot != null }?.let { !it.enabled }

@Composable
private fun streamOn(): Boolean {
    val videoJson by qgcPath(VIDEO_VIEW)
    return remember(videoJson) { videoReading(videoJson)?.streamEnabled == true }
}

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

private fun showCamera(slot: Int, onRefused: () -> Unit = {}) = offMainInOrder { if (!VideoCommands.setActiveSource(slot)) onRefused() }

@Composable
internal fun CameraSwitch(thumbnailRoom: Boolean, modifier: Modifier = Modifier) {
    val json by qgcPath(CAMERAS_VIEW)
    val reading = remember(json) { camerasReading(json) }
    val switch = cameraSwitchState(reading)
    val pipTo = pipToggleTarget(reading, thumbnailRoom, streamOn())
    if (switch == null && pipTo == null) return
    val portrait = flyIsPortrait()
    val buttons: @Composable () -> Unit = {
        switch?.let { CameraSwitchButton(it, if (portrait) MenuSide.Above else MenuSide.Start) }
        pipTo?.let { next -> PipButton(shown = !next) { offMainInOrder { VideoCommands.setPictureInPicture(next) } } }
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
    var lastTap by remember { mutableStateOf<SwitchTap?>(null) }
    val white = MaterialTheme.aircast.outdoorForeground
    val tapped: () -> Unit = {
        val now = SystemClock.uptimeMillis()
        state.toggleTo?.let { slot ->
            if (switchTapAllowed(lastTap, state.shown.slot, now)) {
                val tap = SwitchTap(state.shown.slot, now)
                lastTap = tap
                showCamera(slot) { if (lastTap == tap) lastTap = null }
            }
        } ?: run { open = true }
    }
    Box {
        Surface(
            onClick = tapped,
            shape = CircleShape,
            color = osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
            contentColor = white,
            border = BorderStroke(SWITCH_BORDER, white.copy(alpha = SWITCH_BORDER_ALPHA)),
            modifier = Modifier.semantics { contentDescription = "Switch camera, showing ${state.shown.title}" },
        ) {
            Row(
                Modifier.padding(horizontal = SWITCH_PAD_HORIZONTAL, vertical = SWITCH_PAD_VERTICAL),
                horizontalArrangement = Arrangement.spacedBy(SWITCH_GAP),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(painterResource(R.drawable.ic_videocam), null, Modifier.size(SWITCH_GLYPH))
                Text(state.shown.short, Modifier.widthIn(max = SWITCH_LABEL_MAX), style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
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
            Column(Modifier.width(IntrinsicSize.Max).verticalScroll(rememberScrollState()).padding(vertical = AircastSpace.s2)) {
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
        checked = shown,
        onCheckedChange = { onToggle() },
        modifier = Modifier.size(PIP_BUTTON_SIZE).semantics { role = Role.Switch },
        shape = CircleShape,
        color = if (shown) MaterialTheme.colorScheme.primary else osdBackdrop(Color.Black.copy(alpha = SWITCH_SCRIM_ALPHA)),
        contentColor = if (shown) MaterialTheme.colorScheme.onPrimary else white,
        border = if (shown) null else BorderStroke(SWITCH_BORDER, white.copy(alpha = SWITCH_BORDER_ALPHA)),
    ) {
        Box(contentAlignment = Alignment.Center) {
            Icon(painterResource(R.drawable.ic_picture_in_picture), "Second camera picture-in-picture", Modifier.size(PIP_GLYPH))
        }
    }
}

@Composable
internal fun shownPipCamera(): CameraEntry? {
    val json by qgcPath(CAMERAS_VIEW)
    val streamOn = streamOn()
    return remember(json, streamOn) { pipCamera(camerasReading(json), streamOn) }
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
                .clickable(onClickLabel = "Make ${camera.title} the main view") { showCamera(camera.slot) }
                .semantics { contentDescription = "Second camera, ${camera.title}" },
        )
        Row(
            Modifier.align(Alignment.TopStart).padding(PIP_LABEL_PAD).osdShadow(),
            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CameraStatusDot(camera.status)
            Text(camera.short, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.aircast.outdoorForeground, maxLines = 1)
        }
        editor()
    }
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
