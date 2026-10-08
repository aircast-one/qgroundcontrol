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
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.Constraints
import one.aircast.map.MAP_SCALE_CLEARANCE
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
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
private val PORTRAIT_BAR_HEIGHT = 64.dp
private const val SCRIM_ALPHA = 0.7f
private const val PIP_BORDER_ALPHA = 0.5f
private const val DECK_SCRIM_START = 0.4f
private val MAP_BUTTON_SIZE = 40.dp
private val MAP_ATTRIBUTION_CLEARANCE = 28.dp
private val PORTRAIT_CAMERA_PIP_WIDTH = 96.dp
private val PORTRAIT_CAMERA_PIP_HEIGHT = 54.dp

internal fun portraitSplit(view: FlyView, hasVideo: Boolean): Boolean = view == FlyView.Video && hasVideo

internal fun portraitShowsCamera(reading: VideoReading?): Boolean =
    reading != null && (reading.available || (!reading.streamEnabled && reading.sourceChosen))

internal fun portraitVideoThumbnail(split: Boolean, reading: VideoReading?): Boolean = !split && reading?.available == true

@Composable
internal fun FlyPortrait(
    view: FlyView,
    onView: (FlyView) -> Unit,
    status: @Composable RowScope.() -> Unit,
    video: @Composable (Modifier, Boolean) -> Unit,
    map: @Composable (Modifier) -> Unit,
    keyRow: @Composable () -> Unit,
    rail: @Composable (Boolean) -> Unit,
    overlays: @Composable () -> Unit,
    actions: @Composable (FlyDeckLayout) -> Unit,
) {
    val videoJson by qgcPath(VIDEO_VIEW)
    val reading = remember(videoJson) { videoReading(videoJson) }
    val hasVideo = portraitShowsCamera(reading)
    val split = portraitSplit(view, hasVideo)
    val flyScreen = LocalFlyScreenState.current
    var chromeHeightPx by remember { mutableIntStateOf(0) }
    val barTop = maxOf(
        WindowInsets.statusBars.asPaddingValues().calculateTopPadding() + PORTRAIT_BAR_HEIGHT,
        with(LocalDensity.current) { chromeHeightPx.toDp() },
    )
    val barTopPx = with(LocalDensity.current) { barTop.roundToPx() }
    var deckHeightPx by remember { mutableIntStateOf(0) }
    LaunchedEffect(split, barTopPx, deckHeightPx) { flyScreen.mapInsets = MapInsets(top = if (split) 0 else barTopPx, bottom = deckHeightPx) }
    val videoHeight = LocalConfiguration.current.screenWidthDp.dp / VIDEO_ASPECT
    val videoTop = barTop
    val mapTop = if (split) barTop + videoHeight else 0.dp
    val controlsTop = if (split) mapTop else barTop
    val buttonsTop = AircastSpace.s3 + if (portraitVideoThumbnail(split, reading)) PORTRAIT_PIP_HEIGHT + AircastSpace.s3 else 0.dp

    Column(Modifier.fillMaxSize().background(MaterialTheme.aircast.outdoorBackground)) {
        Box(Modifier.fillMaxWidth().weight(1f).clipToBounds()) {
            if (view == FlyView.ThreeD) Viewer3DPane(Modifier.fillMaxSize().padding(top = mapTop)) else map(Modifier.fillMaxSize().padding(top = mapTop))
            if (hasVideo) {
                video(
                    if (split) {
                        Modifier.padding(top = videoTop).fillMaxWidth().height(videoHeight)
                    } else {
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(top = barTop)
                            .padding(AircastSpace.s3)
                            .size(PORTRAIT_PIP_WIDTH, PORTRAIT_PIP_HEIGHT)
                            .clip(MaterialTheme.shapes.medium)
                            .border(1.dp, MaterialTheme.aircast.outdoorForeground.copy(alpha = PIP_BORDER_ALPHA), MaterialTheme.shapes.medium)
                    },
                    split,
                )
            }
            if (split) {
                Column(Modifier.padding(top = videoTop).fillMaxWidth().height(videoHeight)) {
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
            if (split) shownPipCamera()?.let { camera -> CameraPipThumbnail(camera, Modifier.padding(top = videoTop).padding(AircastSpace.s2).size(PORTRAIT_CAMERA_PIP_WIDTH, PORTRAIT_CAMERA_PIP_HEIGHT)) }
            ControlsAboveDeck(
                controls = {
                    Box(Modifier.fillMaxSize().padding(top = controlsTop)) {
                        Column(
                            Modifier
                                .fillMaxHeight()
                                .padding(start = AircastSpace.s3, top = buttonsTop + MAP_BUTTON_SIZE + AircastSpace.s2, end = AircastSpace.s3, bottom = MAP_ATTRIBUTION_CLEARANCE),
                            verticalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Column(Modifier.weight(1f, fill = false).clipToBounds(), verticalArrangement = Arrangement.spacedBy(AircastSpace.s2)) { overlays() }
                            OsdCompassDial(PORTRAIT_DIAL_SIZE, Modifier.padding(top = AircastSpace.s2))
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
                        Box(Modifier.align(Alignment.BottomEnd).padding(end = AircastSpace.s3, bottom = MAP_SCALE_CLEARANCE)) { rail(split) }
                    }
                },
                deck = {
                    Box(
                        Modifier
                            .fillMaxWidth()
                            .onSizeChanged { deckHeightPx = it.height }
                            .background(Brush.verticalGradient(0f to Color.Transparent, DECK_SCRIM_START to Color.Black.copy(alpha = SCRIM_ALPHA)))
                            .windowInsetsPadding(WindowInsets.navigationBars),
                    ) { actions(FlyDeckLayout.Bottom) }
                },
            )
            Column(Modifier.fillMaxWidth().onSizeChanged { chromeHeightPx = it.height }) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .background(Brush.verticalGradient(listOf(Color.Black.copy(alpha = SCRIM_ALPHA), Color.Transparent)))
                        .windowInsetsPadding(WindowInsets.statusBars)
                        .height(PORTRAIT_BAR_HEIGHT)
                        .padding(horizontal = AircastSpace.s3),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    verticalAlignment = Alignment.CenterVertically,
                    content = status,
                )
                TrafficBanner(Modifier.padding(start = AircastSpace.s3, end = AircastSpace.s3, bottom = AircastSpace.s2))
            }
        }
    }
}

@Composable
private fun ControlsAboveDeck(controls: @Composable () -> Unit, deck: @Composable () -> Unit) {
    Layout(contents = listOf(controls, deck), modifier = Modifier.fillMaxSize()) { (controlsMeasurables, deckMeasurables), constraints ->
        val deckPlaceable = deckMeasurables.single().measure(constraints.copy(minWidth = constraints.maxWidth, minHeight = 0))
        val controlsHeight = (constraints.maxHeight - deckPlaceable.height).coerceAtLeast(0)
        val controlsPlaceable = controlsMeasurables.single().measure(Constraints.fixed(constraints.maxWidth, controlsHeight))
        layout(constraints.maxWidth, constraints.maxHeight) {
            controlsPlaceable.place(0, 0)
            deckPlaceable.place(0, constraints.maxHeight - deckPlaceable.height)
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
