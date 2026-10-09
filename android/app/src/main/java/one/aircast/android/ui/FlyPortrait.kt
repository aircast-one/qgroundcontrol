package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.zIndex
import kotlinx.coroutines.launch
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.material3.Text
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.layout.layout
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpSize
import kotlin.math.roundToInt
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
private val PORTRAIT_DIAL_SIZE = 72.dp
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
    fullScreen: Boolean,
    onFullScreen: () -> Unit,
    onExitFullScreen: () -> Unit,
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
    var boxHeightPx by remember { mutableIntStateOf(0) }
    LaunchedEffect(split, barTopPx, deckHeightPx) { flyScreen.mapInsets = MapInsets(top = if (split) 0 else barTopPx, bottom = deckHeightPx) }
    val screenWidth = LocalConfiguration.current.screenWidthDp.dp
    val videoHeight = screenWidth / VIDEO_ASPECT
    val videoTop = barTop
    val mapTop = if (split) barTop + videoHeight else 0.dp
    val controlsTop = if (split) mapTop else barTop
    val thumbnail = portraitVideoThumbnail(split, reading)
    val pipRoom = pipRoom(thumbnail, flyScreen.videoTucked)
    val corner = flyScreen.pipCorner
    val roomAt: (PipCorner) -> Dp = { at -> if (corner == at) pipRoom else 0.dp }
    val buttonsTop by animateDpAsState(AircastSpace.s3 + roomAt(PipCorner.TopEnd), label = "buttonsTop")
    val overlaysTop by animateDpAsState(maxOf(AircastSpace.s3 + roomAt(PipCorner.TopEnd) + MAP_BUTTON_SIZE + AircastSpace.s2, AircastSpace.s3 + roomAt(PipCorner.TopStart)), label = "overlaysTop")
    val compassLift by animateDpAsState(roomAt(PipCorner.BottomStart), label = "compassLift")
    val railLift by animateDpAsState(roomAt(PipCorner.BottomEnd), label = "railLift")
    val deckHeight = with(LocalDensity.current) { deckHeightPx.toDp() }
    val density = LocalDensity.current
    val geometry = with(density) {
        PipGeometry(
            width = screenWidth.toPx(),
            pip = Size(PORTRAIT_PIP_WIDTH.toPx(), PORTRAIT_PIP_HEIGHT.toPx()),
            inset = AircastSpace.s3.toPx(),
            pipTop = (barTop + AircastSpace.s3).toPx(),
            bottomStartTop = boxHeightPx - deckHeightPx - (MAP_ATTRIBUTION_CLEARANCE + PORTRAIT_PIP_HEIGHT).toPx(),
            bottomEndTop = boxHeightPx - deckHeightPx - (MAP_SCALE_CLEARANCE + PORTRAIT_PIP_HEIGHT).toPx(),
            split = Rect(0f, videoTop.toPx(), screenWidth.toPx(), (videoTop + videoHeight).toPx()),
            full = Rect(0f, 0f, screenWidth.toPx(), boxHeightPx.toFloat().takeIf { it > 0f } ?: (videoTop + videoHeight).toPx()),
        )
    }
    var pipDrag by remember { mutableStateOf(Offset.Zero) }
    var holding by remember { mutableStateOf(false) }
    val target = when {
        fullScreen -> geometry.full
        split -> geometry.split
        else -> geometry.pip(flyScreen.pipCorner, pipDrag)
    }
    val frame = rememberVideoFrame(target, holding)
    val latestSplit by rememberUpdatedState(split)
    val latestTarget by rememberUpdatedState(target)
    val latestExitFullScreen by rememberUpdatedState(onExitFullScreen)
    val latestInFullScreen by rememberUpdatedState(fullScreen)
    val latestGeometry by rememberUpdatedState(geometry)
    val latestOnView by rememberUpdatedState(onView)
    val latestFullScreen by rememberUpdatedState(onFullScreen)
    val cameras = rememberCameraStepper()
    val gimbalDrags by rememberUpdatedState(rememberGimbalDrags())
    val swipeDistance = with(density) { VIDEO_SWIPE_DISTANCE.toPx() }
    val gestures = remember {
        Modifier.videoGestures(
            VideoGestureHandlers(
                owned = { !latestSplit },
                claimsSwipe = { moved -> sideways(moved) && !gimbalDrags },
                onTap = { latestOnView(FlyView.Video) },
                onDoubleTap = { if (latestInFullScreen) latestExitFullScreen() else latestFullScreen() },
                onSwipe = { moved ->
                    when (val swipe = videoSwipe(moved, swipeDistance)) {
                        VideoSwipe.Up, VideoSwipe.Down -> when {
                            latestSplit -> Unit
                            swipe == hidingSwipe(flyScreen.pipCorner) -> flyScreen.videoTucked = true
                            else -> latestOnView(FlyView.Video)
                        }
                        VideoSwipe.Left, VideoSwipe.Right -> cameraStep(swipe)?.let(cameras.step)
                        null -> Unit
                    }
                },
                onHold = { at ->
                    holding = true
                    if (latestSplit) {
                        pipDrag = latestGeometry.dragToCentre(flyScreen.pipCorner, latestTarget.topLeft + at)
                        flyScreen.videoTucked = false
                        latestExitFullScreen()
                        latestOnView(FlyView.Map)
                    }
                    true
                },
                onHoldDrag = { delta -> pipDrag += delta },
                onHoldEnd = {
                    flyScreen.pipCorner = latestGeometry.nearest(latestGeometry.pip(flyScreen.pipCorner, pipDrag).center)
                    pipDrag = Offset.Zero
                    holding = false
                },
            ),
        )
    }

    Column(Modifier.fillMaxSize().background(MaterialTheme.aircast.outdoorBackground)) {
        Box(Modifier.fillMaxWidth().weight(1f).clipToBounds().onSizeChanged { boxHeightPx = it.height }) {
            val underVideo = if (fullScreen) Modifier.clearAndSetSemantics {} else Modifier
            if (view == FlyView.ThreeD) Viewer3DPane(Modifier.fillMaxSize().padding(top = mapTop).then(underVideo)) else map(Modifier.fillMaxSize().padding(top = mapTop).then(underVideo))
            if (hasVideo) {
                androidx.compose.animation.AnimatedVisibility(
                    visible = split || !flyScreen.videoTucked,
                    modifier = Modifier.zIndex(if (fullScreen) FULL_SCREEN_LAYER else 0f),
                    enter = fadeIn() + slideInVertically { if (corner.bottom) it else -it },
                    exit = fadeOut() + slideOutVertically { if (corner.bottom) it else -it },
                ) {
                    val shape = if (split) RectangleShape else MaterialTheme.shapes.medium
                    val border = if (split) 0.dp else 1.dp
                    video(
                        Modifier
                            .videoFrame(target, { frame.value }, cameras.nudge)
                            .clip(shape)
                            .background(Color.Black)
                            .border(border, MaterialTheme.aircast.outdoorForeground.copy(alpha = PIP_BORDER_ALPHA), shape)
                            .semantics {
                                customActions = listOfNotNull(
                                    CustomAccessibilityAction("Show the video full screen") { latestFullScreen(); true },
                                    CustomAccessibilityAction("Hide the video") { flyScreen.videoTucked = true; true }.takeIf { !split },
                                    CustomAccessibilityAction("Make the video small") { onView(FlyView.Map); true }.takeIf { split },
                                )
                            }
                            .then(gestures),
                        split,
                    )
                }
            }
            if (!fullScreen) {
            androidx.compose.animation.AnimatedVisibility(split, enter = fadeIn(), exit = fadeOut()) {
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
                                .padding(start = AircastSpace.s3, top = overlaysTop, end = AircastSpace.s3, bottom = MAP_ATTRIBUTION_CLEARANCE + compassLift),
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
                        Box(Modifier.align(Alignment.BottomEnd).padding(end = AircastSpace.s3, bottom = MAP_SCALE_CLEARANCE + railLift)) { rail(split) }
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
            androidx.compose.animation.AnimatedVisibility(
                visible = thumbnail && flyScreen.videoTucked,
                modifier = Modifier
                    .align(cornerAlignment(corner))
                    .padding(
                        top = if (corner.bottom) 0.dp else barTop + AircastSpace.s3,
                        bottom = if (corner.bottom) deckHeight + if (corner.start) MAP_ATTRIBUTION_CLEARANCE else MAP_SCALE_CLEARANCE else 0.dp,
                        start = AircastSpace.s3,
                        end = AircastSpace.s3,
                    ),
                enter = fadeIn() + slideInVertically { if (corner.bottom) it else -it },
                exit = fadeOut() + slideOutVertically { if (corner.bottom) it else -it },
            ) {
                VideoTab(swipeDistance, growingSwipe(corner)) { flyScreen.videoTucked = false }
            }
            androidx.compose.animation.AnimatedVisibility(
                visible = split && !fullScreen,
                modifier = Modifier.align(Alignment.TopCenter).padding(top = videoTop + videoHeight),
                enter = fadeIn(),
                exit = fadeOut(),
            ) {
                SplitHandle(
                    swipeDistance,
                    onSmaller = { flyScreen.videoTucked = false; onView(FlyView.Map) },
                    onFullScreen = onFullScreen,
                )
            }
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
                BatteryReturnBar(Modifier.padding(horizontal = AircastSpace.s3))
                TrafficBanner(Modifier.padding(start = AircastSpace.s3, end = AircastSpace.s3, bottom = AircastSpace.s2))
            }
            }
        }
    }
}

internal val VIDEO_TAB_HEIGHT = 36.dp
private const val FULL_SCREEN_LAYER = 1f
private val HANDLE_TOUCH = DpSize(96.dp, 28.dp)
private val HANDLE_BAR = DpSize(40.dp, 4.dp)
private const val HANDLE_ALPHA = 0.8f

internal fun cornerAlignment(corner: PipCorner): Alignment = when (corner) {
    PipCorner.TopStart -> Alignment.TopStart
    PipCorner.TopEnd -> Alignment.TopEnd
    PipCorner.BottomStart -> Alignment.BottomStart
    PipCorner.BottomEnd -> Alignment.BottomEnd
}

internal fun pipRoom(thumbnail: Boolean, tucked: Boolean): Dp = when {
    !thumbnail -> 0.dp
    tucked -> VIDEO_TAB_HEIGHT + AircastSpace.s3
    else -> PORTRAIT_PIP_HEIGHT + AircastSpace.s3
}

internal data class PipGeometry(
    val width: Float,
    val pip: Size,
    val inset: Float,
    val pipTop: Float,
    val bottomStartTop: Float,
    val bottomEndTop: Float,
    val split: Rect,
    val full: Rect,
) {
    private fun top(corner: PipCorner): Float = when {
        !corner.bottom -> pipTop
        corner.start -> bottomStartTop
        else -> bottomEndTop
    }

    fun anchor(corner: PipCorner): Offset = Offset(if (corner.start) inset else width - inset - pip.width, top(corner))

    fun pip(corner: PipCorner, drag: Offset): Rect = Rect(anchor(corner) + drag, pip)

    fun dragToCentre(corner: PipCorner, finger: Offset): Offset = finger - anchor(corner) - Offset(pip.width / 2f, pip.height / 2f)

    fun nearest(centre: Offset): PipCorner {
        val start = centre.x < width / 2f
        val bottomTop = top(if (start) PipCorner.BottomStart else PipCorner.BottomEnd)
        val bottom = centre.y > (pipTop + bottomTop + pip.height) / 2f
        return PipCorner.entries.first { it.start == start && it.bottom == bottom }
    }
}

@Composable
private fun VideoTab(swipeDistance: Float, showSwipe: VideoSwipe, onShow: () -> Unit) {
    Surface(
        onClick = onShow,
        shape = CircleShape,
        color = Color.Black.copy(alpha = SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
        modifier = Modifier
            .height(VIDEO_TAB_HEIGHT)
            .verticalSwipe(swipeDistance) { if (it == showSwipe) onShow() }
            .semantics { contentDescription = "Show the video" },
    ) {
        Row(Modifier.padding(horizontal = AircastSpace.s3), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1)) {
            Icon(painterResource(R.drawable.ic_videocam), null, Modifier.size(18.dp))
            Text("Video", style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
private fun SplitHandle(swipeDistance: Float, onSmaller: () -> Unit, onFullScreen: () -> Unit) {
    Box(
        Modifier
            .size(HANDLE_TOUCH)
            .verticalSwipe(swipeDistance) { swipe -> if (swipe == VideoSwipe.Up) onSmaller() else onFullScreen() }
            .semantics {
                contentDescription = "Video size"
                customActions = listOf(
                    CustomAccessibilityAction("Make the video small") { onSmaller(); true },
                    CustomAccessibilityAction("Show the video full screen") { onFullScreen(); true },
                )
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(HANDLE_BAR).background(MaterialTheme.aircast.outdoorForeground.copy(alpha = HANDLE_ALPHA), CircleShape).osdShadow())
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
