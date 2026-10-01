package one.aircast.android.ui

import android.content.Context
import android.content.res.Configuration
import androidx.annotation.DrawableRes
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
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
private val MAP_PIP_SIZE = 120.dp
private val VIDEO_PIP_WIDTH = 156.dp
private val VIDEO_PIP_HEIGHT = 96.dp

enum class FlyView(val label: String, @DrawableRes val icon: Int) {
    Video("Video", R.drawable.ic_videocam),
    Map("Map", R.drawable.ic_map),
    Simple("Simple", R.drawable.ic_speed),
}

internal fun flyViewNamed(name: String?): FlyView = FlyView.entries.firstOrNull { it.name == name } ?: FlyView.Video

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
            FlyView.entries.map { entry ->
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

@Composable
internal fun FlyPortrait(
    view: FlyView,
    onView: (FlyView) -> Unit,
    status: @Composable RowScope.() -> Unit,
    video: @Composable (Modifier, Boolean) -> Unit,
    map: @Composable (Modifier) -> Unit,
    keyRow: @Composable () -> Unit,
    keyRowEnd: @Composable () -> Unit,
    overlays: @Composable () -> Unit,
    actions: @Composable (Boolean) -> Unit,
) {
    val simple = view == FlyView.Simple
    Column(Modifier.fillMaxSize()) {
        Box(Modifier.fillMaxWidth().weight(1f)) {
            if (view == FlyView.Map) map(Modifier.fillMaxSize()) else video(Modifier.fillMaxSize(), true)

            when (view) {
                FlyView.Video -> Box(
                    Modifier
                        .align(Alignment.BottomEnd)
                        .padding(AircastSpace.s3)
                        .size(MAP_PIP_SIZE)
                        .clip(CircleShape)
                        .border(2.dp, MaterialTheme.colorScheme.onSurface, CircleShape),
                ) {
                    map(Modifier.fillMaxSize())
                    Box(Modifier.fillMaxSize().clickable { onView(FlyView.Map) })
                }
                FlyView.Map -> video(
                    Modifier
                        .align(Alignment.TopEnd)
                        .windowInsetsPadding(WindowInsets.statusBars)
                        .padding(top = STATUS_ROW_HEIGHT + AircastSpace.s4, end = AircastSpace.s3)
                        .size(VIDEO_PIP_WIDTH, VIDEO_PIP_HEIGHT)
                        .clip(MaterialTheme.shapes.medium),
                    false,
                )
                FlyView.Simple -> Unit
            }

            Column(
                Modifier
                    .align(Alignment.TopStart)
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
                if (simple) SimpleTiles(Modifier.fillMaxWidth().padding(top = AircastSpace.s3))
                Row(
                    Modifier.fillMaxWidth().padding(top = if (simple) AircastSpace.s4 else AircastSpace.s2),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s5),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    FlyViewSwitcher(view, onView)
                    if (simple) keyRowEnd()
                }
                if (!simple) Row(
                    Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    verticalAlignment = Alignment.CenterVertically,
                ) { keyRow() }
                Column(
                    Modifier.fillMaxWidth().padding(top = if (simple) AircastSpace.s8 else 0.dp),
                    verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    horizontalAlignment = if (simple) Alignment.CenterHorizontally else Alignment.Start,
                ) { overlays() }
            }

            if (simple) {
                Box(Modifier.align(Alignment.BottomCenter).fillMaxWidth()) { actions(true) }
            } else {
                Box(
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(AircastSpace.s3),
                ) { keyRowEnd() }
            }
        }

        if (!simple) {
            Surface(
                Modifier.fillMaxWidth(),
                color = MaterialTheme.colorScheme.surfaceContainerLow,
            ) { actions(false) }
        }
    }
}
