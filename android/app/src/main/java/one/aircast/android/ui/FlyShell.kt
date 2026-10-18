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
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.toSize
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionOnScreen
import androidx.compose.ui.platform.LocalView
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
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
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
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
private val ACTION_RAIL_CLEARANCE = 72.dp
private val STOP_CLEARANCE = 56.dp
private val RAIL_CLEARANCE = STOP_CLEARANCE + AircastSpace.s3 * 2
private val TOP_SCRIM_HEIGHT = 96.dp
private const val TOP_SCRIM_ALPHA = 0.6f
private val MINIMAP_THUMB = 56.dp
private const val FULL_SCREEN_LAYER = 10f
private const val DECISION_LAYER = 4f
private val MAP_SCALE_CLEARANCE = 88.dp
private val STICK_FLOOR = MINIMAP_THUMB + AircastSpace.s3 * 2
private val RAIL_SHUTTER_ROOM = 56.dp

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
    cameraSwitch: @Composable (Boolean, Modifier) -> Unit,
    cameraControls: @Composable () -> Unit,
    overlays: @Composable () -> Unit,
    actions: @Composable (FlyDeckLayout) -> Unit,
    fullScreen: Boolean,
    onFullScreen: () -> Unit,
    onExitFullScreen: () -> Unit,
) {
    val videoJson by one.aircast.android.bridge.qgcPath(VIDEO_VIEW)
    val hasVideo = videoJson?.optBoolean("available") == true
    var boxSize by remember { mutableStateOf(IntSize.Zero) }
    val boxHeight = with(LocalDensity.current) { boxSize.height.toDp() }.takeIf { it > 0.dp } ?: LocalConfiguration.current.screenHeightDp.dp
    val stickBand = boxHeight - TOP_SCRIM_HEIGHT - STICK_FLOOR
    val stickPad = shownStickPad(boxHeight, stickBand)
    val layout = LocalFlyScreenState.current.layout
    val bottomLead = ACTION_RAIL_CLEARANCE - AircastSpace.s3
    val cutoutPadding = WindowInsets.displayCutout.asPaddingValues()
    val direction = LocalLayoutDirection.current
    val cutoutStart = cutoutPadding.calculateStartPadding(direction)
    val cutoutEnd = cutoutPadding.calculateEndPadding(direction)
    var controlsWidth by remember { mutableStateOf(0.dp) }
    var letterbox by remember { mutableStateOf(Letterbox(0.dp, 0.dp)) }
    val insets = stickInsets(stickPad, fullScreen, letterbox, cutoutStart, cutoutEnd, controlsWidth)
    val hostView = LocalView.current
    val rootDensity = LocalDensity.current
    val ornamentEnd = insets.ornamentEnd
    val shellState = LocalFlyScreenState.current
    SideEffect { shellState.mapOrnamentEnd = ornamentEnd }
    DisposableEffect(Unit) { onDispose { shellState.mapOrnamentEnd = null } }
    val cornerRoom = if (stickPad != null) MAP_LAYERS_CLEARANCE else 0.dp
    val flyJson by one.aircast.android.bridge.qgcPath(FLY_STATE)
    val armed = flyState(flyJson)?.armed == true
    LaunchedEffect(armed) { layout.lockWhileArmed(armed) }
    Box(Modifier.fillMaxSize().onSizeChanged { boxSize = it }.onGloballyPositioned { letterbox = letterboxOf(hostView, it, rootDensity) }) {
        val mapIsPip = view == FlyView.Video
        val videoIsPip = view == FlyView.Map
        val videoShown = view != FlyView.ThreeD && (view != FlyView.Map || hasVideo)
        val thumbShown = mapIsPip || (videoIsPip && hasVideo)
        if (view == FlyView.ThreeD) one.aircast.map.Viewer3DPane(Modifier.fillMaxSize())
        val thumbShape = MaterialTheme.shapes.medium
        val thumbPlace = Modifier
            .align(Alignment.BottomStart)
            .windowInsetsPadding(WindowInsets.displayCutout)
            .padding(start = bottomLead)
            .padding(AircastSpace.s3)
            .size(MINIMAP_THUMB)
            .clip(thumbShape)

        if (view != FlyView.ThreeD) {
            map(
                (if (mapIsPip) Modifier.zIndex(1f).then(thumbPlace).border(2.dp, MaterialTheme.colorScheme.onSurface, thumbShape) else Modifier.fillMaxSize())
                    .then(if (fullScreen) Modifier.clearAndSetSemantics {} else Modifier),
            )
        }
        val density = LocalDensity.current
        val cutout = WindowInsets.displayCutout
        val thumbFrame = with(density) {
            Rect(
                Offset(cutout.getLeft(this, LocalLayoutDirection.current) + (AircastSpace.s3 + bottomLead).toPx(), boxSize.height - cutout.getBottom(this) - (AircastSpace.s3 + MINIMAP_THUMB).toPx()),
                Size(MINIMAP_THUMB.toPx(), MINIMAP_THUMB.toPx()),
            )
        }
        val pipNow = videoIsPip && !fullScreen
        val videoTarget = if (pipNow) thumbFrame else Rect(Offset.Zero, boxSize.toSize())
        val frame = rememberVideoFrame(videoTarget, holding = false)
        val cameras = rememberCameraStepper()
        val dragAims by rememberUpdatedState(rememberDragAims())
        val syntheticAims by rememberUpdatedState(rememberSyntheticAims())
        val swipeDistance = with(density) { VIDEO_SWIPE_DISTANCE.toPx() }
        val latestPip by rememberUpdatedState(pipNow)
        val latestExitFullScreen by rememberUpdatedState(onExitFullScreen)
        val latestEnterFullScreen by rememberUpdatedState(onFullScreen)
        val latestInFullScreen by rememberUpdatedState(fullScreen)
        val videoGestures = remember {
            Modifier.videoGestures(
                VideoGestureHandlers(
                    owned = { false },
                    claimsSwipe = { moved -> sideways(moved) && !dragAims && !(syntheticAims && !latestPip) },
                    onTap = {},
                    onDoubleTap = { if (latestInFullScreen) latestExitFullScreen() else latestEnterFullScreen() },
                    onSwipe = { moved -> cameraStep(videoSwipe(moved, swipeDistance))?.let(cameras.step) },
                    onHold = { false },
                    onHoldDrag = {},
                    onHoldEnd = {},
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
        if (mapIsPip) Box(
            Modifier
                .zIndex(2f)
                .then(thumbPlace)
                .clickable(onClickLabel = "Show the map") { onView(FlyView.Map) }
                .semantics { contentDescription = "Map" },
        )
        if (videoIsPip && hasVideo) Box(
            Modifier
                .zIndex(3f)
                .then(thumbPlace)
                .border(2.dp, MaterialTheme.colorScheme.onSurface, thumbShape)
                .clickable(onClickLabel = "Show the video") { onView(FlyView.Video) }
                .semantics { contentDescription = "Video" },
        )
        val settingsOver = LocalAppNavigation.current.settingsOpen
        Box(Modifier.zIndex(1f).fillMaxSize().graphicsLayer { alpha = if (settingsOver) 0f else 1f }.windowInsetsPadding(WindowInsets.displayCutout)) {
        if (view == FlyView.Map) {
            var layers by remember { mutableStateOf(false) }
            Box(Modifier.align(Alignment.BottomEnd).padding(end = AircastSpace.s3, bottom = if (stickPad != null) AircastSpace.s3 else MAP_SCALE_CLEARANCE)) {
                LayoutWidget("mapLayers", hideable = false) {
                    Surface(
                        onClick = { layers = true },
                        modifier = Modifier.size(MAP_LAYERS_SIZE),
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
                    Box {
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
                            HeadingTape(Modifier.align(Alignment.CenterHorizontally))
                            TrafficBanner()
                            Row(
                                Modifier.horizontalScroll(rememberScrollState()),
                                horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                                verticalAlignment = Alignment.CenterVertically,
                            ) { keyRow() }
                        }
                        BatteryReturnBar(Modifier.align(Alignment.TopCenter).windowInsetsPadding(WindowInsets.statusBars))
                    }
                },
                overlays = {
                    Column(
                        Modifier.fillMaxWidth().clipToBounds().padding(start = insets.overlaysStart, end = insets.overlaysEnd),
                        verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    ) {
                        CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) {
                            overlays()
                        }
                    }
                },
                bottom = { Spacer(Modifier.height(cornerRoom)) },
                bottomAlignment = Alignment.End,
                overlaysAboveBottom = true,
                message = if (videoMessage) ({ Box(Modifier.osdShadow()) { FlyNoVideoMessage() } }) else null,
                messageAboveBottom = false,
                obstacles = { flyScreen.obstacles.values },
                onInsets = { insets -> if (insets != flyScreen.mapInsets) flyScreen.mapInsets = insets },
            )
            if (stickPad == null) Box(
                Modifier
                    .zIndex(1f)
                    .fillMaxHeight()
                    .align(Alignment.CenterEnd)
                    .windowInsetsPadding(WindowInsets.statusBars)
                    .padding(top = RAIL_CLEARANCE + AircastSpace.s3, bottom = RAIL_CLEARANCE)
                    .padding(end = AircastSpace.s3),
                contentAlignment = Alignment.Center,
            ) {
                CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) {
                    Row(verticalAlignment = Alignment.Top) {
                        cameraSwitch(mapIsPip, Modifier.padding(end = AircastSpace.s2))
                        Box(Modifier.widthIn(min = RAIL_SHUTTER_ROOM), contentAlignment = Alignment.Center) { cameraControls() }
                    }
                }
            } else Box(
                Modifier
                    .zIndex(1f)
                    .align(Alignment.BottomEnd)
                    .padding(end = if (view == FlyView.Map) MAP_LAYERS_CLEARANCE else AircastSpace.s3, bottom = AircastSpace.s3)
                    .heightIn(min = MAP_LAYERS_SIZE),
                contentAlignment = Alignment.CenterEnd,
            ) {
                CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) { cameraSwitch(mapIsPip, Modifier) }
            }
            if (hasVehicle() && !flyScreen.guidedPanelOpen) {
            val besideThumb = if (thumbShown) MINIMAP_THUMB else 0.dp
            Box(
                Modifier
                    .zIndex(2f)
                    .align(Alignment.BottomStart)
                    .padding(start = bottomLead + besideThumb + AircastSpace.s3 * 2, end = AircastSpace.s3, bottom = AircastSpace.s3)
                    .osdShadow(),
            ) {
                CompositionLocalProvider(LocalFlyOsd provides true) { TelemetryRow(valuesShown = true, chooser = false, compact = true, stacked = true) }
            }
            }
        }
        }
        val deciding = LocalFlyScreenState.current.guidedPanelOpen
        CompositionLocalProvider(LocalFlyOsd provides (view == FlyView.Video)) {
            Box(
                Modifier
                    .zIndex(if (deciding) DECISION_LAYER else 2f)
                    .fillMaxSize()
                    .graphicsLayer { alpha = if (settingsOver) 0f else 1f }
                    .windowInsetsPadding(WindowInsets.displayCutout)
                    .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, bottom = 0.dp),
            ) { actions(FlyDeckLayout.Rail) }
        }
        }
        stickPad?.let { pad ->
            val settingsOver = LocalAppNavigation.current.settingsOpen
            val underDecision = LocalFlyScreenState.current.guidedPanelOpen
            CompositionLocalProvider(LocalFlyOsd provides (view == FlyView.Video), LocalAvoidedByVideoMessage provides true) {
                VirtualJoystick(
                    pad,
                    Modifier
                        .zIndex(if (underDecision) DECISION_LAYER - 0.5f else FULL_SCREEN_LAYER + 1f)
                        .align(Alignment.BottomCenter)
                        .graphicsLayer { alpha = if (settingsOver) 0f else 1f }
                        .padding(start = insets.rowStart, end = insets.rowEnd, bottom = STICK_FLOOR)
                        .height(maxOf(stickBand, pad)),
                    trailing = {
                        if (!fullScreen) Row(Modifier.fillMaxHeight()) {
                            Spacer(Modifier.width(insets.trailingGap))
                            Box(
                                Modifier.fillMaxHeight().widthIn(min = RAIL_SHUTTER_ROOM).onSizeChanged { controlsWidth = with(density) { it.width.toDp() } },
                                contentAlignment = Alignment.Center,
                            ) { cameraControls() }
                        }
                    },
                )
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

private val MAP_LAYERS_SIZE = 48.dp
internal val MAP_LAYERS_CLEARANCE = MAP_LAYERS_SIZE + AircastSpace.s3 + AircastSpace.s2
private val FLEET_CARD_MAX_WIDTH = 440.dp
private val FLEET_CARD_MAX_HEIGHT = 320.dp
private const val FLEET_CARD_MIN_SCREEN_DP = 600

internal data class MapInsets(val top: Int, val bottom: Int)

internal data class Letterbox(val start: Dp, val end: Dp)

internal data class StickInsets(val rowStart: Dp, val rowEnd: Dp, val trailingGap: Dp, val overlaysStart: Dp, val overlaysEnd: Dp, val ornamentEnd: Dp?)

internal fun stickInsets(pad: Dp?, fullScreen: Boolean, letterbox: Letterbox, cutoutStart: Dp, cutoutEnd: Dp, controlsWidth: Dp): StickInsets {
    val startGap = letterbox.start + cutoutStart
    val endGap = letterbox.end + cutoutEnd
    val edge = cutoutEnd + AircastSpace.s3
    val inset = if (fullScreen) maxOf(startGap, endGap) + AircastSpace.s3 else maxOf(startGap + ACTION_RAIL_CLEARANCE, endGap + AircastSpace.s3 + controlsWidth + AircastSpace.s2)
    return StickInsets(
        rowStart = inset - letterbox.start,
        rowEnd = if (fullScreen) inset - letterbox.end else edge,
        trailingGap = (inset - letterbox.end - edge - controlsWidth).coerceAtLeast(AircastSpace.s2),
        overlaysStart = pad?.let { inset - startGap + it + AircastSpace.s3 } ?: ACTION_RAIL_CLEARANCE,
        overlaysEnd = pad?.let { inset - endGap + it + AircastSpace.s3 } ?: AircastSpace.s3,
        ornamentEnd = pad?.takeIf { !fullScreen }?.let { inset - letterbox.end + it + AircastSpace.s2 },
    )
}

private fun letterboxOf(view: android.view.View, coordinates: LayoutCoordinates, density: androidx.compose.ui.unit.Density): Letterbox {
    val activity = generateSequence(view.context) { (it as? android.content.ContextWrapper)?.baseContext }.filterIsInstance<android.app.Activity>().firstOrNull()
    if (activity?.isInMultiWindowMode == true) return Letterbox(0.dp, 0.dp)
    val left = coordinates.positionOnScreen().x
    val right = screenWidthPx(view) - left - coordinates.size.width
    val ltr = view.layoutDirection == android.view.View.LAYOUT_DIRECTION_LTR
    return with(density) { Letterbox(start = (if (ltr) left else right).coerceAtLeast(0f).toDp(), end = (if (ltr) right else left).coerceAtLeast(0f).toDp()) }
}

private fun screenWidthPx(view: android.view.View): Float =
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
        view.context.getSystemService(android.view.WindowManager::class.java).maximumWindowMetrics.bounds.width().toFloat()
    } else {
        @Suppress("DEPRECATION")
        android.graphics.Point().also { view.display?.getRealSize(it) }.x.toFloat()
    }

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
