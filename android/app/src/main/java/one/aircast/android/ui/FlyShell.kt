package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.zIndex
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
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
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
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.map.AircastSpace

private const val FLY_STORE = "fly"
private const val FLY_VIEW_KEY = "view"
private const val FLY_SCRIM_ALPHA = 0.55f
private val STATUS_ROW_HEIGHT = 32.dp
private val SEGMENT_HEIGHT = 32.dp
internal val MAP_PIP_SIZE = 120.dp
private const val MAP_PIP_KEY = "mapPip"
private const val VIDEO_PIP_KEY = "videoPip"
private val VIDEO_PIP_WIDTH = 156.dp
private val VIDEO_PIP_HEIGHT = 96.dp
private val MAP_LAYERS_BUTTON = 48.dp
private val MINIMAP_WIDTH = 184.dp
private val MINIMAP_HEIGHT = 112.dp
private const val COMPASS_DIAL_KEY = "compassDial"
private val ACTION_RAIL_CLEARANCE = 72.dp
private val STOP_CLEARANCE = 56.dp
private val TOP_SCRIM_HEIGHT = 96.dp
private const val TOP_SCRIM_ALPHA = 0.6f
private val PIP_TOGGLE_SIZE = 28.dp
private const val PIP_EXPANDED_KEY = "IsPIPVisible"
private const val LANDSCAPE_MAP_SHOWN_KEY = "LandscapeMapShown"

internal fun pipExpandedKey(landscape: Boolean): Pair<String, Boolean> =
    if (landscape) LANDSCAPE_MAP_SHOWN_KEY to false else PIP_EXPANDED_KEY to true

internal fun loadPipExpanded(context: Context, landscape: Boolean): Boolean =
    pipExpandedKey(landscape).let { (key, default) -> context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).getBoolean(key, default) }

internal fun savePipExpanded(context: Context, landscape: Boolean, expanded: Boolean) =
    context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).edit().putBoolean(pipExpandedKey(landscape).first, expanded).apply()

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
internal fun FlyViewSwitcher(view: FlyView, onView: (FlyView) -> Unit, modifier: Modifier = Modifier) {
    Surface(
        modifier,
        shape = CircleShape,
        color = osdBackdrop(Color.Black.copy(alpha = FLY_SCRIM_ALPHA)),
        contentColor = MaterialTheme.aircast.outdoorForeground,
    ) {
        Row(
            Modifier.padding(3.dp).selectableGroup(),
            horizontalArrangement = Arrangement.spacedBy(2.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val viewer3dJson by one.aircast.android.bridge.qgcPath(VIEWER3D_VIEW)
            val enabled3d = viewer3dEnabled(viewer3dJson)
            LaunchedEffect(enabled3d, view) { flyViewAllowed(enabled3d, view).takeIf { it != view }?.let(onView) }
            val offered = flyViewsOffered(enabled3d, view)
            offered.map { entry ->
                val selected = entry == view
                Surface(
                    shape = CircleShape,
                    color = if (selected) osdBackdrop(MaterialTheme.colorScheme.secondaryContainer) else Color.Transparent,
                    contentColor = if (selected) osdTint(MaterialTheme.colorScheme.onSecondaryContainer, MaterialTheme.aircast.outdoorForeground) else MaterialTheme.aircast.outdoorForeground,
                    modifier = Modifier.selectable(selected = selected, role = Role.Tab) { onView(entry) },
                ) {
                    Row(
                        Modifier.height(SEGMENT_HEIGHT).padding(start = if (selected) AircastSpace.s3 else AircastSpace.s2, end = if (selected) AircastSpace.s3 else AircastSpace.s2),
                        horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(painterResource(entry.icon), entry.label, Modifier.size(18.dp))
                        if (selected) Text(entry.label, style = MaterialTheme.typography.labelLarge)
                    }
                }
            }
        }
    }
}

@Composable
internal fun FlyScreen(
    view: FlyView,
    onView: (FlyView) -> Unit,
    landscape: Boolean,
    status: @Composable RowScope.() -> Unit,
    video: @Composable (Modifier, Boolean) -> Unit,
    map: @Composable (Modifier) -> Unit,
    keyRow: @Composable () -> Unit,
    keyRowEnd: @Composable () -> Unit,
    rail: @Composable () -> Unit,
    heading: @Composable () -> Unit,
    overlays: @Composable () -> Unit,
    actions: @Composable (FlyDeckLayout) -> Unit,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val videoJson by one.aircast.android.bridge.qgcPath(VIDEO_VIEW)
    val hasVideo = videoJson?.optBoolean("available") == true
    var pipExpanded by remember(landscape) { mutableStateOf(loadPipExpanded(context, landscape)) }
    val togglePip: () -> Unit = {
        pipExpanded = !pipExpanded
        savePipExpanded(context, landscape, pipExpanded)
    }
    val layout = LocalFlyScreenState.current.layout
    val flyJson by one.aircast.android.bridge.qgcPath(FLY_STATE)
    val armed = flyState(flyJson)?.armed == true
    LaunchedEffect(armed) { layout.lockWhileArmed(armed) }
    val stage: @Composable (Modifier) -> Unit = { stageModifier ->
        Box(stageModifier) {
            val mapIsPip = view == FlyView.Video
            val videoIsPip = view == FlyView.Map
            val mapShown = view == FlyView.Map || (mapIsPip && pipExpanded)
            val videoShown = view != FlyView.ThreeD && (view != FlyView.Map || videoPipShown(hasVideo, pipExpanded))
            if (view == FlyView.ThreeD) one.aircast.map.Viewer3DPane(Modifier.fillMaxSize())
            val pipCorner = Modifier.windowInsetsPadding(WindowInsets.statusBars)
                .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, end = AircastSpace.s3)
            val mapPipAlign = if (landscape) Alignment.BottomStart else Alignment.BottomEnd
            val mapPipShape = if (landscape) MaterialTheme.shapes.medium else CircleShape
            val mapPipSize = if (landscape) Modifier.size(MINIMAP_WIDTH, MINIMAP_HEIGHT) else Modifier.size(MAP_PIP_SIZE)
            val videoPipAlign = if (landscape) Alignment.BottomStart else Alignment.TopEnd
            val videoPipPlace = if (landscape) Modifier.padding(AircastSpace.s3) else pipCorner
            val videoPipSize = if (landscape) Modifier.size(MINIMAP_WIDTH, MINIMAP_HEIGHT) else Modifier.size(VIDEO_PIP_WIDTH, VIDEO_PIP_HEIGHT)

            if (mapShown) {
                map(
                    if (mapIsPip) {
                        Modifier
                            .zIndex(1f)
                            .align(mapPipAlign)
                            .padding(AircastSpace.s3)
                            .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                            .then(mapPipSize)
                            .clip(mapPipShape)
                            .border(2.dp, MaterialTheme.colorScheme.onSurface, mapPipShape)
                    } else {
                        Modifier.fillMaxSize()
                    },
                )
            }
            if (videoShown) {
                video(
                    if (videoIsPip) {
                        Modifier
                            .zIndex(1f)
                            .align(videoPipAlign)
                            .then(videoPipPlace)
                            .then(layoutPlacement(VIDEO_PIP_KEY, keepOnScreen = false))
                            .then(videoPipSize)
                            .clip(MaterialTheme.shapes.medium)
                    } else {
                        Modifier.fillMaxSize()
                    },
                    !videoIsPip,
                )
            }
            if (mapIsPip) {
                if (pipExpanded) {
                    Box(
                        Modifier
                            .zIndex(2f)
                            .align(mapPipAlign)
                            .padding(AircastSpace.s3)
                            .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                            .then(mapPipSize)
                            .clip(mapPipShape)
                            .clickable { onView(FlyView.Map) },
                    )
                }
                if (landscape && !pipExpanded) {
                    Box(
                        Modifier
                            .zIndex(3f)
                            .align(Alignment.BottomStart)
                            .padding(AircastSpace.s3)
                            .avoidedByVideoMessage(COMPASS_DIAL_KEY)
                            .clip(CircleShape)
                            .clickable(onClickLabel = "Show the map") { togglePip() }
                            .semantics { contentDescription = "Compass" },
                    ) { OsdCompassDial(MINIMAP_HEIGHT) }
                }
                if (pipExpanded || !landscape) Box(Modifier.zIndex(3f).align(mapPipAlign).padding(AircastSpace.s3).then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = true)).then(mapPipSize).avoidedByVideoMessage(MAP_PIP_KEY).holdToEditLayout()) {
                    PipToggle(pipExpanded, togglePip, Modifier.align(if (pipExpanded) Alignment.TopStart else Alignment.BottomEnd))
                    if (pipExpanded) LayoutPipEditor(MAP_PIP_KEY, mapPipShape, Modifier.matchParentSize())
                }
            }
            if (videoIsPip && hasVideo) {
                Box(Modifier.zIndex(3f).align(videoPipAlign).then(videoPipPlace).then(layoutPlacement(VIDEO_PIP_KEY, keepOnScreen = true)).then(videoPipSize).holdToEditLayout()) {
                    PipToggle(pipExpanded, togglePip, Modifier.align(if (pipExpanded) Alignment.TopStart else if (landscape) Alignment.BottomStart else Alignment.TopEnd))
                    if (pipExpanded) LayoutPipEditor(VIDEO_PIP_KEY, MaterialTheme.shapes.medium, Modifier.matchParentSize())
                }
            }
            if (view == FlyView.Map) {
                var layers by remember { mutableStateOf(false) }
                Box(if (landscape) Modifier.align(Alignment.BottomStart).padding(start = AircastSpace.s3, bottom = MINIMAP_HEIGHT + AircastSpace.s3 * 2) else Modifier.align(Alignment.BottomEnd).padding(AircastSpace.s3)) {
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

            if (landscape) {
                Box(
                    Modifier
                        .align(Alignment.TopCenter)
                        .fillMaxWidth()
                        .height(TOP_SCRIM_HEIGHT)
                        .background(androidx.compose.ui.graphics.Brush.verticalGradient(listOf(Color.Black.copy(alpha = TOP_SCRIM_ALPHA), Color.Transparent))),
                )
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
                            CompositionLocalProvider(LocalCompactStatus provides landscape) {
                                Row(
                                    Modifier.fillMaxWidth().heightIn(min = STATUS_ROW_HEIGHT),
                                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                                    verticalAlignment = Alignment.CenterVertically,
                                    content = status,
                                )
                            }
                            if (!landscape) Row(Modifier.padding(top = AircastSpace.s2)) {
                                LayoutWidget("viewSwitcher", hideable = false) { FlyViewSwitcher(view, onView) }
                            }
                            Row(
                                Modifier.horizontalScroll(rememberScrollState()),
                                horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                                verticalAlignment = Alignment.CenterVertically,
                            ) { keyRow() }
                        }
                    },
                    overlays = {
                        Column(
                            Modifier.fillMaxWidth().clipToBounds().padding(start = if (landscape) ACTION_RAIL_CLEARANCE else AircastSpace.s3, end = AircastSpace.s3),
                            verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                        ) {
                            CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) {
                                if (!landscape) heading()
                                overlays()
                            }
                        }
                    },
                    bottom = {
                        Box(Modifier.padding(AircastSpace.s3)) { keyRowEnd() }
                    },
                    bottomAlignment = if (landscape) Alignment.End else Alignment.Start,
                    overlaysAboveBottom = true,
                    message = if (videoMessage) ({ Box(Modifier.osdShadow()) { FlyNoVideoMessage() } }) else null,
                    messageAboveBottom = !landscape,
                    obstacles = { flyScreen.obstacles.values },
                    onInsets = { insets -> if (insets != flyScreen.mapInsets) flyScreen.mapInsets = insets },
                )
                Box(
                    Modifier
                        .zIndex(1f)
                        .fillMaxHeight()
                        .align(Alignment.CenterEnd)
                        .windowInsetsPadding(WindowInsets.statusBars)
                        .padding(
                            top = STATUS_ROW_HEIGHT + AircastSpace.s4 + when {
                                landscape -> 0.dp
                                videoIsPip && hasVideo && pipExpanded -> VIDEO_PIP_HEIGHT + AircastSpace.s3
                                else -> 0.dp
                            },
                            bottom = when {
                                landscape -> STOP_CLEARANCE
                                mapIsPip && pipExpanded -> MAP_PIP_SIZE
                                else -> MAP_LAYERS_BUTTON
                            } + AircastSpace.s3 * 2,
                            end = AircastSpace.s3,
                        ),
                    contentAlignment = Alignment.Center,
                ) {
                    CompositionLocalProvider(LocalAvoidedByVideoMessage provides true) { rail() }
                }
                if (landscape) {
                    Box(
                        Modifier
                            .zIndex(2f)
                            .fillMaxSize()
                            .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, bottom = 0.dp),
                    ) { actions(FlyDeckLayout.Rail) }
                }
            }
        }
    }

    if (landscape) {
        stage(Modifier.fillMaxSize())
    } else {
        Column(Modifier.fillMaxSize()) {
            stage(Modifier.fillMaxWidth().weight(1f))
            run {
                Surface(
                    Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surfaceContainerLow,
                ) { actions(FlyDeckLayout.Bottom) }
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


@Composable
internal fun FlyTabMenu(tabs: List<Pair<String, () -> Unit>>) {
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
            tabs.map { (label, select) ->
                androidx.compose.material3.DropdownMenuItem(text = { Text(label) }, onClick = {
                    open = false
                    select()
                })
            }
        }
    }
}
