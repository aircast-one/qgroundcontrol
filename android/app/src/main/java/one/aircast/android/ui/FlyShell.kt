package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.toSize
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.Layout

import one.aircast.map.MapLayersSheet

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember

import androidx.compose.runtime.mutableStateOf

import androidx.compose.ui.draw.clipToBounds


import android.content.Context
import android.content.res.Configuration
import androidx.annotation.DrawableRes
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.ui.graphics.Color
import one.aircast.map.aircast
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.map.AircastSpace

private const val FLY_STORE = "fly"
private const val FLY_VIEW_KEY = "view"
private const val FLY_SCRIM_ALPHA = 0.55f
private val STATUS_ROW_HEIGHT = 32.dp
private const val MAP_PIP_KEY = "mapPip"
private const val VIDEO_PIP_KEY = "videoPip"
private val MINIMAP_WIDTH = 184.dp
private val MINIMAP_HEIGHT = 112.dp
private const val COMPASS_DIAL_KEY = "compassDial"
private val ACTION_RAIL_CLEARANCE = 72.dp
private val STOP_CLEARANCE = 56.dp
private val RAIL_CLEARANCE = STOP_CLEARANCE + AircastSpace.s3 * 2
private val TOP_SCRIM_HEIGHT = 96.dp
private const val TOP_SCRIM_ALPHA = 0.6f
private val PIP_TOGGLE_SIZE = 28.dp
private const val MINI_MAP_KEY = "LandscapeMiniMap"
private val MINIMAP_THUMB = 56.dp
private const val CAMERA_PIP_KEY = "cameraPip"
private val CAMERA_PIP_WIDTH = 112.dp
private val CAMERA_PIP_HEIGHT = 63.dp
private const val FULL_SCREEN_LAYER = 10f

internal enum class MiniMap { Thumb, Map, Compass }

internal fun miniMapNamed(name: String?): MiniMap = MiniMap.entries.firstOrNull { it.name == name } ?: MiniMap.Thumb

internal fun loadMiniMap(context: Context): MiniMap =
    miniMapNamed(context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).getString(MINI_MAP_KEY, null))

internal fun saveMiniMap(context: Context, mini: MiniMap) =
    context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).edit().putString(MINI_MAP_KEY, mini.name).apply()

internal fun miniWidth(mini: MiniMap): androidx.compose.ui.unit.Dp = when (mini) {
    MiniMap.Thumb -> MINIMAP_THUMB
    MiniMap.Map -> MINIMAP_WIDTH
    MiniMap.Compass -> MINIMAP_HEIGHT
}

private fun miniHeight(mini: MiniMap): Dp = if (mini == MiniMap.Thumb) MINIMAP_THUMB else MINIMAP_HEIGHT

internal fun videoPipShown(hasVideo: Boolean, expanded: Boolean): Boolean = hasVideo && expanded

@Composable
private fun PipToggle(expanded: Boolean, onToggle: () -> Unit, modifier: Modifier = Modifier) {
    Surface(
        onClick = onToggle,
        modifier = modifier.size(PIP_TOGGLE_SIZE),
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
    ) {
        Box(contentAlignment = Alignment.Center) {
            Icon(
                if (expanded) Icons.Default.KeyboardArrowDown else Icons.Default.KeyboardArrowUp,
                if (expanded) "Hide picture-in-picture" else "Show picture-in-picture",
                Modifier.size(18.dp),
            )
        }
    }
}

enum class FlyView(val label: String, @DrawableRes val icon: Int) {
    Video("Video", R.drawable.ic_videocam),
    Map("Map", R.drawable.ic_map),
    ThreeD("3D", R.drawable.ic_explore),
}

internal fun flyViewNamed(name: String?): FlyView = FlyView.entries.firstOrNull { it.name == name } ?: FlyView.Video

internal const val VIEWER3D_VIEW = "view.viewer3d"

internal fun flyViewsOffered(viewer3dEnabled: Boolean?, current: FlyView): List<FlyView> =
    FlyView.entries.filter { it != FlyView.ThreeD || viewer3dEnabled == true || (viewer3dEnabled == null && current == FlyView.ThreeD) }

internal fun flyViewAllowed(viewer3dEnabled: Boolean?, current: FlyView): FlyView =
    if (current == FlyView.ThreeD && viewer3dEnabled == false) FlyView.Map else current

internal fun viewer3dEnabled(view: org.json.JSONObject?): Boolean? = view?.takeIf { it.has("enabled") }?.optBoolean("enabled")

internal fun flyViewShown(chosen: FlyView, armed: Boolean, noVideoSource: Boolean): FlyView =
    if (chosen == FlyView.Video && armed && noVideoSource) FlyView.Map else chosen

internal fun flyViewSwapped(view: FlyView): FlyView = if (view == FlyView.Map) FlyView.Video else FlyView.Map

internal fun loadFlyView(context: Context): FlyView =
    flyViewNamed(context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).getString(FLY_VIEW_KEY, null))

internal fun saveFlyView(context: Context, view: FlyView) =
    context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).edit().putString(FLY_VIEW_KEY, view.name).apply()

@Composable
internal fun flyIsPortrait(): Boolean =
    LocalConfiguration.current.orientation == Configuration.ORIENTATION_PORTRAIT

@Composable
internal fun FlyScreen(
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
    val context = androidx.compose.ui.platform.LocalContext.current
    val videoJson by one.aircast.android.bridge.qgcPath(VIDEO_VIEW)
    val hasVideo = videoJson?.optBoolean("available") == true
    var mini by remember { mutableStateOf(loadMiniMap(context)) }
    val showMini: (MiniMap) -> Unit = { next ->
        mini = next
        saveMiniMap(context, next)
    }
    val pipExpanded = mini != MiniMap.Thumb
    val togglePip: () -> Unit = { showMini(if (pipExpanded) MiniMap.Thumb else MiniMap.Map) }
    val layout = LocalFlyScreenState.current.layout
    val flyJson by one.aircast.android.bridge.qgcPath(FLY_STATE)
    val armed = flyState(flyJson)?.armed == true
    LaunchedEffect(armed) { layout.lockWhileArmed(armed) }
    var boxSize by remember { mutableStateOf(IntSize.Zero) }
    Box(Modifier.fillMaxSize().onSizeChanged { boxSize = it }) {
        val mapIsPip = view == FlyView.Video
        val videoIsPip = view == FlyView.Map
        val mapShown = view == FlyView.Map || (mapIsPip && mini != MiniMap.Compass)
        val videoShown = view != FlyView.ThreeD && (view != FlyView.Map || videoPipShown(hasVideo, pipExpanded))
        if (view == FlyView.ThreeD) one.aircast.map.Viewer3DPane(Modifier.fillMaxSize())
        val pipAlign = Alignment.BottomStart
        val mapPipShape = MaterialTheme.shapes.medium
        val pipSize = Modifier.size(MINIMAP_WIDTH, MINIMAP_HEIGHT)
        val mapPipSize = if (mini == MiniMap.Thumb) Modifier.size(MINIMAP_THUMB) else pipSize

        if (mapShown) {
            map(
                (if (mapIsPip) {
                    Modifier
                        .zIndex(1f)
                        .align(pipAlign)
                        .windowInsetsPadding(WindowInsets.displayCutout)
                        .padding(AircastSpace.s3)
                        .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                        .then(mapPipSize)
                        .clip(mapPipShape)
                        .border(2.dp, MaterialTheme.colorScheme.onSurface, mapPipShape)
                } else {
                    Modifier.fillMaxSize()
                }).then(if (fullScreen) Modifier.clearAndSetSemantics {} else Modifier),
            )
        }
        val density = LocalDensity.current
        val cutout = WindowInsets.displayCutout
        val videoKey = orientedKey(VIDEO_PIP_KEY, landscape = true)
        val videoMoved = layout.offsets[videoKey] ?: (0f to 0f)
        val pipHome = with(density) {
            Offset(cutout.getLeft(this, LocalLayoutDirection.current) + AircastSpace.s3.toPx(), boxSize.height - cutout.getBottom(this) - AircastSpace.s3.toPx() - MINIMAP_HEIGHT.toPx())
        }
        val pipFrame = with(density) { Rect(pipHome + Offset(videoMoved.first.dp.toPx(), videoMoved.second.dp.toPx()), Size(MINIMAP_WIDTH.toPx(), MINIMAP_HEIGHT.toPx())) }
        val pipNow = videoIsPip && !fullScreen
        val videoTarget = if (pipNow) pipFrame else Rect(Offset.Zero, boxSize.toSize())
        var holding by remember { mutableStateOf(false) }
        val frame = rememberVideoFrame(videoTarget, holding)
        val cameras = rememberCameraStepper()
        val gimbalDrags by rememberUpdatedState(rememberGimbalDrags())
        val swipeDistance = with(density) { VIDEO_SWIPE_DISTANCE.toPx() }
        val latestPip by rememberUpdatedState(pipNow)
        val latestHome by rememberUpdatedState(pipHome)
        val latestTarget by rememberUpdatedState(videoTarget)
        val latestOnView by rememberUpdatedState(onView)
        val latestExitFullScreen by rememberUpdatedState(onExitFullScreen)
        val latestExpanded by rememberUpdatedState(pipExpanded)
        val latestShowMini by rememberUpdatedState(showMini)
        val videoGestures = remember {
            Modifier.videoGestures(
                VideoGestureHandlers(
                    owned = { false },
                    claimsSwipe = { moved -> sideways(moved) && !gimbalDrags },
                    onTap = {},
                    onDoubleTap = {},
                    onSwipe = { moved -> cameraStep(videoSwipe(moved, swipeDistance))?.let(cameras.step) },
                    onHold = { at ->
                        if (latestPip) {
                            false
                        } else {
                            holding = true
                            val pip = Size(MINIMAP_WIDTH.value * density.density, MINIMAP_HEIGHT.value * density.density)
                            val wanted = latestTarget.topLeft + at - Offset(pip.width / 2f, pip.height / 2f) - latestHome
                            val (x, y) = layout.offsets[videoKey] ?: (0f to 0f)
                            layout.nudge(videoKey, wanted.x / density.density - x, wanted.y / density.density - y)
                            if (!latestExpanded) latestShowMini(MiniMap.Map)
                            latestExitFullScreen()
                            latestOnView(FlyView.Map)
                            true
                        }
                    },
                    onHoldDrag = { delta -> layout.nudge(videoKey, delta.x / density.density, delta.y / density.density) },
                    onHoldEnd = {
                        layout.saveOffset(videoKey)
                        holding = false
                    },
                ),
            )
        }
        androidx.compose.animation.AnimatedVisibility(
            visible = videoShown,
            modifier = Modifier.zIndex(if (fullScreen) FULL_SCREEN_LAYER else if (videoIsPip) 1f else 0f),
            enter = fadeIn() + slideInVertically { it },
            exit = fadeOut() + slideOutVertically { it },
        ) {
            val shape = if (pipNow) MaterialTheme.shapes.medium else RectangleShape
            video(
                Modifier
                    .videoFrame(videoTarget, { frame.value }, cameras.nudge)
                    .clip(shape)
                    .background(Color.Black)
                    .then(videoGestures),
                !pipNow,
            )
        }
        if (!fullScreen) {
        Box(
            Modifier
                .align(Alignment.TopCenter)
                .fillMaxWidth()
                .height(TOP_SCRIM_HEIGHT)
                .background(androidx.compose.ui.graphics.Brush.verticalGradient(listOf(Color.Black.copy(alpha = TOP_SCRIM_ALPHA), Color.Transparent))),
        )
        if (mapIsPip) {
            if (mini == MiniMap.Thumb) {
                Box(
                    Modifier
                        .zIndex(2f)
                        .align(pipAlign)
                        .windowInsetsPadding(WindowInsets.displayCutout)
                        .padding(AircastSpace.s3)
                        .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                        .then(mapPipSize)
                        .clip(mapPipShape)
                        .clickable(onClickLabel = "Show the mini-map") { showMini(MiniMap.Map) }
                        .semantics { contentDescription = "Map" },
                )
            }
            if (mini == MiniMap.Compass) {
                Box(
                    Modifier
                        .zIndex(3f)
                        .align(Alignment.BottomStart)
                        .windowInsetsPadding(WindowInsets.displayCutout)
                        .padding(AircastSpace.s3)
                        .avoidedByVideoMessage(COMPASS_DIAL_KEY)
                        .clip(CircleShape)
                        .clickable(onClickLabel = "Show the map") { showMini(MiniMap.Map) }
                        .semantics { contentDescription = "Compass" },
                ) { OsdCompassDial(MINIMAP_HEIGHT) }
            }
            if (mini == MiniMap.Map) Box(
                Modifier
                    .zIndex(3f)
                    .align(pipAlign)
                        .windowInsetsPadding(WindowInsets.displayCutout)
                    .padding(AircastSpace.s3)
                    .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = true))
                    .then(pipSize)
                    .avoidedByVideoMessage(MAP_PIP_KEY)
                    .holdToEditLayout()
                    .clip(mapPipShape)
                    .clickable(onClickLabel = "Show the map full screen") { onView(FlyView.Map) }
                    .semantics { contentDescription = "Mini-map" },
            ) {
                PipToggle(true, togglePip, Modifier.align(Alignment.TopStart))
                Surface(
                    onClick = { showMini(MiniMap.Compass) },
                    modifier = Modifier.align(Alignment.TopEnd).size(PIP_TOGGLE_SIZE),
                    shape = CircleShape,
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                ) {
                    Box(contentAlignment = Alignment.Center) { Icon(painterResource(R.drawable.ic_explore), "Show the compass", Modifier.size(18.dp)) }
                }
                LayoutPipEditor(MAP_PIP_KEY, mapPipShape, Modifier.matchParentSize())
            }
        }
        val latestToggle by rememberUpdatedState(togglePip)
        val latestFullScreen by rememberUpdatedState(onFullScreen)
        val pipGestures = remember {
            Modifier.videoGestures(
                VideoGestureHandlers(
                    owned = { !layout.editing },
                    claimsSwipe = { false },
                    onTap = { latestOnView(FlyView.Video) },
                    onDoubleTap = { latestFullScreen() },
                    onSwipe = { moved ->
                        when (val swipe = videoSwipe(moved, swipeDistance)) {
                            VideoSwipe.Down -> latestToggle()
                            VideoSwipe.Up -> latestOnView(FlyView.Video)
                            else -> cameraStep(swipe)?.let(cameras.step)
                        }
                    },
                    onHold = { if (layout.editing) false else { holding = true; true } },
                    onHoldDrag = { delta -> layout.nudge(videoKey, delta.x / density.density, delta.y / density.density) },
                    onHoldEnd = {
                        layout.saveOffset(videoKey)
                        holding = false
                    },
                ),
                pass = PointerEventPass.Main,
            )
        }
        if (videoIsPip && hasVideo) {
            if (pipExpanded) {
                Box(
                    Modifier
                        .zIndex(3f)
                        .align(pipAlign)
                        .windowInsetsPadding(WindowInsets.displayCutout)
                        .padding(AircastSpace.s3)
                        .then(layoutPlacement(VIDEO_PIP_KEY, keepOnScreen = true))
                        .then(pipSize)
                        .clip(MaterialTheme.shapes.medium)
                        .semantics {
                            contentDescription = "Video picture-in-picture"
                            onClick("Show the video large") { onView(FlyView.Video); true }
                            customActions = listOf(
                                CustomAccessibilityAction("Show the video full screen") { onFullScreen(); true },
                                CustomAccessibilityAction("Hide the video") { togglePip(); true },
                            )
                        }
                        .then(pipGestures),
                ) {
                    PipToggle(true, togglePip, Modifier.align(Alignment.TopStart))
                    LayoutPipEditor(VIDEO_PIP_KEY, MaterialTheme.shapes.medium, Modifier.matchParentSize())
                }
            } else {
                PipToggle(false, togglePip, Modifier.zIndex(3f).align(pipAlign).windowInsetsPadding(WindowInsets.displayCutout).padding(AircastSpace.s3).verticalSwipe(swipeDistance) { if (it == VideoSwipe.Up) togglePip() })
            }
        }
        if (mapIsPip) shownPipCamera()?.let { camera ->
            CameraPipThumbnail(
                camera,
                Modifier
                    .zIndex(3f)
                    .align(pipAlign)
                    .windowInsetsPadding(WindowInsets.displayCutout)
                    .padding(start = ACTION_RAIL_CLEARANCE, bottom = miniHeight(mini) + AircastSpace.s3 * 2)
                    .then(layoutPlacement(CAMERA_PIP_KEY, keepOnScreen = true))
                    .size(CAMERA_PIP_WIDTH, CAMERA_PIP_HEIGHT)
                    .avoidedByVideoMessage(CAMERA_PIP_KEY)
                    .holdToEditLayout(),
            ) { LayoutPipEditor(CAMERA_PIP_KEY, MaterialTheme.shapes.medium, Modifier.fillMaxSize()) }
        }
        val settingsOver = LocalAppNavigation.current.settingsOpen
        Box(Modifier.zIndex(1f).fillMaxSize().graphicsLayer { alpha = if (settingsOver) 0f else 1f }.windowInsetsPadding(WindowInsets.displayCutout)) {
        if (view == FlyView.Map) {
            var layers by remember { mutableStateOf(false) }
            Box(Modifier.align(Alignment.BottomStart).padding(start = AircastSpace.s3, bottom = (if (hasVideo && pipExpanded) MINIMAP_HEIGHT else PIP_TOGGLE_SIZE) + AircastSpace.s3 * 2)) {
                LayoutWidget("mapLayers", hideable = false) {
                    Surface(
                        onClick = { layers = true },
                        modifier = Modifier.size(48.dp),
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.surfaceContainerHigh,
                    ) {
                        Box(contentAlignment = Alignment.Center) { Icon(painterResource(R.drawable.ic_layers), "Map layers") }
                    }
                }
            }
            if (layers) MapLayersSheet { layers = false }
        }

        val flyScreen = LocalFlyScreenState.current
        val videoMessage = videoShown && !videoIsPip && videoReading(videoJson)?.decoding != true
        CompositionLocalProvider(LocalFlyOsd provides (view == FlyView.Video)) {
            FlyChromeLayout(
                modifier = Modifier.fillMaxSize(),
                top = {
                    Column(
                        Modifier
                            .windowInsetsPadding(WindowInsets.statusBars)
                            .padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s2),
                        verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    ) {
                        CompositionLocalProvider(LocalCompactStatus provides true) {
                            Row(
                                Modifier.fillMaxWidth().heightIn(min = STATUS_ROW_HEIGHT),
                                horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                                verticalAlignment = Alignment.CenterVertically,
                                content = status,
                            )
                        }
                        TrafficBanner()
                        Row(
                            Modifier.horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                            verticalAlignment = Alignment.CenterVertically,
                        ) { keyRow() }
                    }
                },
                overlays = {
                    Column(
                        Modifier.fillMaxWidth().clipToBounds().padding(start = ACTION_RAIL_CLEARANCE, end = AircastSpace.s3),
                        verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    ) {
                        CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) {
                            overlays()
                        }
                    }
                },
                bottom = {

                },
                bottomAlignment = Alignment.End,
                overlaysAboveBottom = true,
                message = if (videoMessage) ({ Box(Modifier.osdShadow()) { FlyNoVideoMessage() } }) else null,
                messageAboveBottom = false,
                obstacles = { flyScreen.obstacles.values },
                onInsets = { insets -> if (insets != flyScreen.mapInsets) flyScreen.mapInsets = insets },
            )
            Box(
                Modifier
                    .zIndex(1f)
                    .fillMaxHeight()
                    .align(Alignment.CenterEnd)
                    .windowInsetsPadding(WindowInsets.statusBars)
                    .padding(vertical = RAIL_CLEARANCE, horizontal = 0.dp)
                    .padding(end = AircastSpace.s3),
                contentAlignment = Alignment.Center,
            ) {
                CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) { rail(mapIsPip) }
            }
            Box(
                Modifier
                    .zIndex(2f)
                    .fillMaxSize()
                    .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, bottom = 0.dp),
            ) { actions(FlyDeckLayout.Rail) }
            if (hasVehicle()) {
            val besideMini = if (mapIsPip) miniWidth(mini) else if (hasVideo && pipExpanded) MINIMAP_WIDTH else PIP_TOGGLE_SIZE
            Box(
                Modifier
                    .zIndex(2f)
                    .align(Alignment.BottomStart)
                    .padding(start = besideMini + AircastSpace.s3 * 2, bottom = AircastSpace.s3)
                    .osdShadow(),
            ) {
                CompositionLocalProvider(LocalFlyOsd provides true) { TelemetryRow(valuesShown = true, chooser = false, compact = true, stacked = true) }
            }
            }
        }
        }
        }
    }
}

@Composable
internal fun FleetCard() {
    val flyScreen = LocalFlyScreenState.current
    if (androidx.compose.ui.platform.LocalConfiguration.current.screenWidthDp < FLEET_CARD_MIN_SCREEN_DP) return
    if (flyScreen.guidedPanelOpen) return
    Surface(
        shape = MaterialTheme.shapes.large,
        color = osdBackdrop(MaterialTheme.colorScheme.surfaceContainer),
        modifier = Modifier.widthIn(max = FLEET_CARD_MAX_WIDTH).heightIn(max = FLEET_CARD_MAX_HEIGHT),
    ) {
        FleetPanel(Modifier.verticalScroll(rememberScrollState()).padding(vertical = AircastSpace.s2))
    }
}

internal val MAP_LAYERS_CLEARANCE = 48.dp + AircastSpace.s3 + AircastSpace.s2
private val FLEET_CARD_MAX_WIDTH = 440.dp
private val FLEET_CARD_MAX_HEIGHT = 320.dp
private const val FLEET_CARD_MIN_SCREEN_DP = 600

internal data class MapInsets(val top: Int, val bottom: Int)

internal val LocalAvoidedByVideoMessage = staticCompositionLocalOf { false }

@Composable
private fun FlyChromeLayout(
    modifier: Modifier,
    top: @Composable () -> Unit,
    overlays: @Composable () -> Unit,
    bottom: @Composable () -> Unit,
    bottomAlignment: Alignment.Horizontal,
    overlaysAboveBottom: Boolean,
    message: (@Composable () -> Unit)?,
    messageAboveBottom: Boolean,
    obstacles: () -> Collection<Rect>,
    onInsets: (MapInsets) -> Unit,
) {
    val placed = remember { PlacedCoordinates() }
    Layout(
        contents = listOf(top, overlays, bottom, message ?: {}),
        modifier = modifier.onPlaced { placed.coordinates = it },
    ) { (topSlot, overlaySlot, bottomSlot, messageSlot), constraints ->
        val loose = constraints.copy(minWidth = 0, minHeight = 0)
        val width = constraints.maxWidth
        val height = constraints.maxHeight
        val tops = topSlot.map { it.measure(loose) }
        val topHeight = tops.maxOfOrNull { it.height } ?: 0
        val bottoms = bottomSlot.map { it.measure(loose) }
        val bottomHeight = bottoms.maxOfOrNull { it.height } ?: 0
        val overlayRoom = (height - topHeight - if (overlaysAboveBottom) bottomHeight else 0).coerceAtLeast(0)
        val overlayPlaceables = overlaySlot.map { it.measure(loose.copy(maxHeight = overlayRoom)) }
        val overlayHeight = overlayPlaceables.maxOfOrNull { it.height } ?: 0
        val origin = placed.coordinates?.takeIf { it.isAttached }?.positionInRoot() ?: Offset.Zero
        val free = Rect(
            left = AircastSpace.s3.toPx(),
            top = topHeight.toFloat(),
            right = width - AircastSpace.s3.toPx(),
            bottom = height - AircastSpace.s3.toPx() - if (messageAboveBottom) bottomHeight else 0,
        )
        val region = messageRegion(free, obstacles().map { it.translate(-origin) }, AircastSpace.s3.toPx(), NO_VIDEO_PILL_WIDTH.toPx(), NO_VIDEO_PILL_HEIGHT.toPx())
            ?.let { room -> centredRegion(room, width / 2f, NO_VIDEO_PILL_WIDTH.toPx()) }
        val messages = region?.let { room -> messageSlot.map { it.measure(Constraints.fixed(room.width.toInt(), room.height.toInt())) } }.orEmpty()
        layout(width, height) {
            region?.let { room -> messages.forEach { it.place(room.left.toInt(), room.top.toInt()) } }
            tops.forEach { it.place(0, 0) }
            overlayPlaceables.forEach { it.place(0, topHeight) }
            bottoms.forEach { it.place(bottomAlignment.align(it.width, width, layoutDirection), height - it.height) }
            onInsets(MapInsets(top = topHeight + overlayHeight, bottom = bottomHeight))
        }
    }
}

private class PlacedCoordinates {
    var coordinates: LayoutCoordinates? = null
}

@Composable
internal fun Modifier.avoidedByVideoMessage(key: String): Modifier {
    val flyScreen = LocalFlyScreenState.current
    DisposableEffect(key) { onDispose { flyScreen.obstacles.remove(key) } }
    return onGloballyPositioned { flyScreen.obstacles[key] = Rect(it.positionInRoot(), it.size.toSize()) }
}


internal data class MenuDestination(val label: String, @androidx.annotation.DrawableRes val icon: Int, val current: Boolean, val onSelect: () -> Unit)

@Composable
internal fun FlyTabMenu(destinations: List<MenuDestination>) {
    var open by remember { mutableStateOf(false) }
    Box {
        Surface(
            onClick = { open = true },
            modifier = Modifier.size(STATUS_ROW_HEIGHT + AircastSpace.s2),
            shape = CircleShape,
            color = osdBackdrop(Color.Black.copy(alpha = FLY_SCRIM_ALPHA)),
            contentColor = MaterialTheme.aircast.outdoorForeground,
        ) {
            Box(contentAlignment = Alignment.Center) { Icon(Icons.Default.Menu, "Menu", Modifier.size(22.dp)) }
        }
        androidx.compose.material3.DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            destinations.map { destination ->
                val tint = if (destination.current) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
                androidx.compose.material3.DropdownMenuItem(
                    text = { Text(destination.label, color = tint) },
                    leadingIcon = { Icon(painterResource(destination.icon), null, tint = tint) },
                    onClick = {
                        open = false
                        if (!destination.current) destination.onSelect()
                    },
                )
            }
        }
    }
}
