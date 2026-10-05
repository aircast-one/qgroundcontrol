package one.aircast.android.ui

import androidx.compose.ui.zIndex

import one.aircast.mapspike.MapLayersSheet

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember

import androidx.compose.runtime.mutableStateOf

import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.runtime.setValue

import androidx.compose.runtime.getValue

import android.content.Context
import android.content.res.Configuration
import androidx.annotation.DrawableRes
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.ui.layout.boundsInParent
import androidx.compose.ui.layout.onGloballyPositioned
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
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.mapspike.AircastSpace

private const val FLY_STORE = "fly"
private const val FLY_VIEW_KEY = "view"
private const val FLY_SCRIM_ALPHA = 0.55f
private val STATUS_ROW_HEIGHT = 32.dp
private val SEGMENT_HEIGHT = 32.dp
private val SIDE_PANEL_WIDTH = 168.dp
internal val MAP_PIP_SIZE = 120.dp
private const val MAP_PIP_KEY = "mapPip"
private const val VIDEO_PIP_KEY = "videoPip"
private val VIDEO_PIP_WIDTH = 156.dp
private val VIDEO_PIP_HEIGHT = 96.dp
private val PIP_TOGGLE_SIZE = 28.dp
private const val PIP_EXPANDED_KEY = "IsPIPVisible"

internal fun loadPipExpanded(context: Context): Boolean =
    context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).getBoolean(PIP_EXPANDED_KEY, true)

internal fun savePipExpanded(context: Context, expanded: Boolean) =
    context.getSharedPreferences(FLY_STORE, Context.MODE_PRIVATE).edit().putBoolean(PIP_EXPANDED_KEY, expanded).apply()

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
    Simple("Simple", R.drawable.ic_speed),
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
        color = Color.Black.copy(alpha = FLY_SCRIM_ALPHA),
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
                    color = if (selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent,
                    contentColor = if (selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.aircast.outdoorForeground,
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

internal object FlyChrome {
    var topPx by mutableIntStateOf(0)
    var bottomPx by mutableIntStateOf(0)
    var controlsBottomPx by mutableFloatStateOf(0f)
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
    overlays: @Composable () -> Unit,
    actions: @Composable (FlyDeckLayout) -> Unit,
) {
    val simple = view == FlyView.Simple
    val context = androidx.compose.ui.platform.LocalContext.current
    val videoJson by one.aircast.android.bridge.qgcPath(VIDEO_VIEW)
    val hasVideo = videoJson?.optBoolean("available") == true
    var pipExpanded by remember { mutableStateOf(loadPipExpanded(context)) }
    val togglePip: () -> Unit = {
        pipExpanded = !pipExpanded
        savePipExpanded(context, pipExpanded)
    }
    val stage: @Composable (Modifier) -> Unit = { stageModifier ->
        Box(stageModifier) {
            val mapIsPip = view == FlyView.Video
            val videoIsPip = view == FlyView.Map
            val mapShown = view == FlyView.Map || (mapIsPip && pipExpanded)
            val videoShown = view != FlyView.ThreeD && (view != FlyView.Map || videoPipShown(hasVideo, pipExpanded))
            if (view == FlyView.ThreeD) one.aircast.mapspike.Viewer3DPane(Modifier.fillMaxSize())
            val pipCorner = Modifier.windowInsetsPadding(WindowInsets.statusBars)
                .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, end = AircastSpace.s3)

            if (mapShown) {
                map(
                    if (mapIsPip) {
                        Modifier
                            .zIndex(1f)
                            .align(Alignment.BottomEnd)
                            .padding(AircastSpace.s3)
                            .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                            .size(MAP_PIP_SIZE)
                            .clip(CircleShape)
                            .border(2.dp, MaterialTheme.colorScheme.onSurface, CircleShape)
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
                            .align(Alignment.TopEnd)
                            .then(pipCorner)
                            .then(layoutPlacement(VIDEO_PIP_KEY, keepOnScreen = false))
                            .size(VIDEO_PIP_WIDTH, VIDEO_PIP_HEIGHT)
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
                            .align(Alignment.BottomEnd)
                            .padding(AircastSpace.s3)
                            .then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = false))
                            .size(MAP_PIP_SIZE)
                            .clip(CircleShape)
                            .clickable { onView(FlyView.Map) },
                    )
                }
                Box(Modifier.zIndex(3f).align(Alignment.BottomEnd).padding(AircastSpace.s3).then(layoutPlacement(MAP_PIP_KEY, keepOnScreen = true)).size(MAP_PIP_SIZE)) {
                    PipToggle(pipExpanded, togglePip, Modifier.align(if (pipExpanded) Alignment.TopStart else Alignment.BottomEnd))
                    if (pipExpanded) LayoutPipEditor(MAP_PIP_KEY, CircleShape, Modifier.matchParentSize())
                }
            }
            if (videoIsPip && hasVideo) {
                Box(Modifier.zIndex(3f).align(Alignment.TopEnd).then(pipCorner).then(layoutPlacement(VIDEO_PIP_KEY, keepOnScreen = true)).size(VIDEO_PIP_WIDTH, VIDEO_PIP_HEIGHT)) {
                    PipToggle(pipExpanded, togglePip, Modifier.align(if (pipExpanded) Alignment.TopStart else Alignment.TopEnd))
                    if (pipExpanded) LayoutPipEditor(VIDEO_PIP_KEY, MaterialTheme.shapes.medium, Modifier.matchParentSize())
                }
            }
            if (view == FlyView.Map) {
                var layers by remember { mutableStateOf(false) }
                Surface(
                    onClick = { layers = true },
                    modifier = Modifier.align(Alignment.BottomEnd).padding(AircastSpace.s3).size(48.dp),
                    shape = CircleShape,
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                ) {
                    Box(contentAlignment = Alignment.Center) { Icon(painterResource(R.drawable.ic_layers), "Map layers") }
                }
                if (layers) MapLayersSheet { layers = false }
            }

            val sideBySide = simple && landscape
            var bottomReserve by remember { mutableIntStateOf(0) }
            val reserveDp = with(androidx.compose.ui.platform.LocalDensity.current) { (if (simple) 0 else bottomReserve).toDp() }
            Box(Modifier.align(Alignment.TopStart).fillMaxSize().padding(bottom = reserveDp).clipToBounds()) {
            Column(
                Modifier
                    .align(Alignment.TopStart)
                    .onGloballyPositioned { FlyChrome.topPx = it.boundsInParent().bottom.toInt() }
                    .windowInsetsPadding(WindowInsets.statusBars)
                    .padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s2),
                verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
            ) {
                Row(
                    Modifier.fillMaxWidth().heightIn(min = STATUS_ROW_HEIGHT),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    verticalAlignment = Alignment.CenterVertically,
                    content = status,
                )
                val half = Modifier.fillMaxWidth(if (sideBySide) SIMPLE_LANDSCAPE_SPLIT else 1f)
                if (simple) SimpleTiles(half.padding(top = AircastSpace.s3))
                Row(
                    half.padding(top = if (simple) AircastSpace.s4 else AircastSpace.s2)
                        .then(if (simple) Modifier.onGloballyPositioned { FlyChrome.controlsBottomPx = it.boundsInRoot().bottom } else Modifier),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s5),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    LayoutWidget("viewSwitcher", hideable = false) { FlyViewSwitcher(view, onView) }
                    if (simple) keyRowEnd()
                }
                if (!simple) Row(
                    Modifier.onGloballyPositioned { FlyChrome.controlsBottomPx = it.boundsInRoot().bottom }.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    verticalAlignment = Alignment.CenterVertically,
                ) { keyRow() }
                Column(
                    half.padding(top = if (simple && !sideBySide) AircastSpace.s8 else 0.dp),
                    verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    horizontalAlignment = if (simple) Alignment.CenterHorizontally else Alignment.Start,
                ) { overlays() }
            }
            }

            if (simple) {
                Box(
                    Modifier
                        .align(if (sideBySide) Alignment.BottomEnd else Alignment.BottomCenter)
                        .fillMaxWidth(if (sideBySide) SIMPLE_LANDSCAPE_SPLIT else 1f)
                        .onGloballyPositioned { FlyChrome.bottomPx = it.size.height },
                ) { actions(FlyDeckLayout.Simple) }
            } else {
                Box(
                    Modifier
                        .align(Alignment.BottomStart)
                        .onGloballyPositioned { FlyChrome.bottomPx = it.size.height; bottomReserve = it.size.height }
                        .padding(AircastSpace.s3),
                ) { keyRowEnd() }
            }
        }
    }

    if (landscape) {
        Row(Modifier.fillMaxSize()) {
            stage(Modifier.weight(1f).fillMaxHeight())
            if (!simple) {
                Surface(
                    Modifier.width(SIDE_PANEL_WIDTH).fillMaxHeight(),
                    color = MaterialTheme.colorScheme.surfaceContainerLow,
                ) { Box(Modifier.windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Vertical + WindowInsetsSides.End))) { actions(FlyDeckLayout.Side) } }
            }
        }
    } else {
        Column(Modifier.fillMaxSize()) {
            stage(Modifier.fillMaxWidth().weight(1f))
            if (!simple) {
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
    if (androidx.compose.ui.platform.LocalConfiguration.current.screenWidthDp < FLEET_CARD_MIN_SCREEN_DP) return
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surfaceContainer,
        modifier = Modifier.widthIn(max = FLEET_CARD_MAX_WIDTH).heightIn(max = FLEET_CARD_MAX_HEIGHT),
    ) {
        FleetPanel(Modifier.verticalScroll(rememberScrollState()).padding(vertical = AircastSpace.s2))
    }
}

private const val SIMPLE_LANDSCAPE_SPLIT = 0.5f
internal val MAP_LAYERS_CLEARANCE = 48.dp + AircastSpace.s3 + AircastSpace.s2
private val FLEET_CARD_MAX_WIDTH = 440.dp
private val FLEET_CARD_MAX_HEIGHT = 320.dp
private const val FLEET_CARD_MIN_SCREEN_DP = 600
