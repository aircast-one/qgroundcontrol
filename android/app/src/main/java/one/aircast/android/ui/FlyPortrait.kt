package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.MapLayersSheet
import one.aircast.map.Viewer3DPane
import one.aircast.map.aircast

private const val VIDEO_ASPECT = 16f / 9f
private const val MAP_BUTTON_SCRIM_ALPHA = 0.55f
private val PORTRAIT_PIP_WIDTH = 160.dp
private val PORTRAIT_PIP_HEIGHT = 90.dp
private val PORTRAIT_DIAL_SIZE = 120.dp
private val PORTRAIT_STATUS_HEIGHT = 32.dp
private val MAP_BUTTON_SIZE = 40.dp
private val MAP_ATTRIBUTION_CLEARANCE = 28.dp

internal fun portraitSplit(view: FlyView, hasVideo: Boolean): Boolean = view == FlyView.Video && hasVideo

@Composable
internal fun FlyPortrait(
    view: FlyView,
    onView: (FlyView) -> Unit,
    status: @Composable RowScope.() -> Unit,
    video: @Composable (Modifier, Boolean) -> Unit,
    map: @Composable (Modifier) -> Unit,
    keyRow: @Composable () -> Unit,
    rail: @Composable () -> Unit,
    overlays: @Composable () -> Unit,
    actions: @Composable (FlyDeckLayout) -> Unit,
) {
    val videoJson by qgcPath(VIDEO_VIEW)
    val reading = remember(videoJson) { videoReading(videoJson) }
    val hasVideo = reading?.available == true
    val split = portraitSplit(view, hasVideo)
    val flyScreen = LocalFlyScreenState.current
    LaunchedEffect(Unit) { flyScreen.mapInsets = MapInsets(top = 0, bottom = 0) }
    val buttonsTop = AircastSpace.s3 + if (!split && hasVideo) PORTRAIT_PIP_HEIGHT + AircastSpace.s3 else 0.dp
    val videoHeight = LocalConfiguration.current.screenWidthDp.dp / VIDEO_ASPECT
    val mapTop = if (split) videoHeight else 0.dp

    Column(Modifier.fillMaxSize().background(MaterialTheme.aircast.outdoorBackground)) {
        Row(
            Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s2)
                .heightIn(min = PORTRAIT_STATUS_HEIGHT),
            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
            verticalAlignment = Alignment.CenterVertically,
            content = status,
        )
        Box(Modifier.fillMaxWidth().weight(1f).clipToBounds()) {
            if (view == FlyView.ThreeD) Viewer3DPane(Modifier.fillMaxSize().padding(top = mapTop)) else map(Modifier.fillMaxSize().padding(top = mapTop))
            if (hasVideo) {
                video(
                    if (split) {
                        Modifier.fillMaxWidth().height(videoHeight)
                    } else {
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(AircastSpace.s3)
                            .size(PORTRAIT_PIP_WIDTH, PORTRAIT_PIP_HEIGHT)
                            .clip(MaterialTheme.shapes.medium)
                            .border(2.dp, MaterialTheme.aircast.outdoorForeground, MaterialTheme.shapes.medium)
                    },
                    split,
                )
            }
            if (split) {
                Column(Modifier.fillMaxWidth().height(videoHeight)) {
                    Box(Modifier.fillMaxWidth().weight(1f)) {
                        if (reading?.decoding != true) Box(Modifier.matchParentSize().osdShadow()) { FlyNoVideoMessage() }
                    }
                    CompositionLocalProvider(LocalFlyOsd provides true) {
                        Row(
                            Modifier.padding(AircastSpace.s2).horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                            verticalAlignment = Alignment.CenterVertically,
                        ) { keyRow() }
                    }
                }
            }
            Box(Modifier.fillMaxSize().padding(top = mapTop)) {
                Column(
                    Modifier
                        .align(Alignment.TopStart)
                        .padding(start = AircastSpace.s3, top = buttonsTop + MAP_BUTTON_SIZE + AircastSpace.s2, end = AircastSpace.s3),
                    verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                ) {
                    overlays()
                }
                MapButtons(
                    view = view,
                    split = split,
                    hasVideo = hasVideo,
                    onView = onView,
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(end = AircastSpace.s3, top = buttonsTop),
                )
                Box(Modifier.align(Alignment.BottomEnd).padding(end = AircastSpace.s3, bottom = MAP_ATTRIBUTION_CLEARANCE)) { rail() }
                OsdCompassDial(PORTRAIT_DIAL_SIZE, Modifier.align(Alignment.BottomStart).padding(start = AircastSpace.s3, bottom = MAP_ATTRIBUTION_CLEARANCE))
            }
        }
        Surface(
            Modifier.fillMaxWidth(),
            color = MaterialTheme.aircast.outdoorBackground,
        ) {
            Box(Modifier.windowInsetsPadding(WindowInsets.navigationBars)) { actions(FlyDeckLayout.Bottom) }
        }
    }
}

@Composable
private fun MapButtons(view: FlyView, split: Boolean, hasVideo: Boolean, onView: (FlyView) -> Unit, modifier: Modifier) {
    val viewer3dJson by qgcPath(VIEWER3D_VIEW)
    val enabled3d = viewer3dEnabled(viewer3dJson)
    LaunchedEffect(enabled3d, view) { flyViewAllowed(enabled3d, view).takeIf { it != view }?.let(onView) }
    var layers by remember { mutableStateOf(false) }
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2)) {
        if (split) MapButton(R.drawable.ic_fullscreen, "Enlarge the map") { onView(FlyView.Map) }
        if (!split && hasVideo && view == FlyView.ThreeD) MapButton(R.drawable.ic_videocam, "Show the camera above the map") { onView(FlyView.Video) }
        MapButton(R.drawable.ic_layers, "Map layers") { layers = true }
        if (FlyView.ThreeD in flyViewsOffered(enabled3d, view)) {
            MapButton(if (view == FlyView.ThreeD) R.drawable.ic_map else R.drawable.ic_explore, if (view == FlyView.ThreeD) "Show the map" else "Show the 3D view") {
                onView(if (view == FlyView.ThreeD) FlyView.Map else FlyView.ThreeD)
            }
        }
    }
    if (layers) MapLayersSheet { layers = false }
}

@Composable
private fun MapButton(@DrawableRes icon: Int, label: String, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        modifier = Modifier.size(MAP_BUTTON_SIZE),
        shape = CircleShape,
        color = Color.Black.copy(alpha = MAP_BUTTON_SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
    ) {
        Box(contentAlignment = Alignment.Center) { Icon(painterResource(icon), label, Modifier.size(20.dp)) }
    }
}
