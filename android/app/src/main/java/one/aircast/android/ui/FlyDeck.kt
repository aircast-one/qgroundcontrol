package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.launch
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.BorderStroke
import androidx.compose.ui.platform.LocalContext
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import one.aircast.map.AircastSheet
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import one.aircast.map.AircastSpace

internal const val CHECKLIST = "checklist"

enum class FlyDeckLayout { Bottom, Rail }
private const val DISABLED_ALPHA = 0.38f
private val DECK_BUTTON_HEIGHT = 80.dp
private val DECK_ICON_SIZE = 28.dp

internal class DeckEntry(
    val id: String,
    val label: String,
    @DrawableRes val icon: Int,
    val enabled: Boolean,
    val warning: Boolean = false,
    val onHold: (() -> Unit)? = null,
    val onClick: () -> Unit,
)

internal const val HOLD_TO_TAKE_OFF = "Hold to take off"
internal const val DECK_HOLD_MS = 1500
private const val DECK_HOLD_FILL_ALPHA = 0.3f
private const val NANOS_PER_MILLI = 1_000_000L

internal fun holdTakeoffHeight(takeoff: GuidedTakeoff?): Double? =
    takeoff?.takeIf(::takeoffRangeUsable)?.initial

@Composable
internal fun rememberHold(key: Any, enabled: Boolean, onTap: () -> Unit, onHold: (() -> Unit)?, label: String): Pair<Float, Modifier> {
    val progress = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val haptics = LocalHapticFeedback.current
    val tap by rememberUpdatedState(onTap)
    val hold by rememberUpdatedState(onHold)
    val gesture = when {
        onHold != null && enabled -> Modifier
            .pointerInput(key) {
                detectTapGestures(onPress = {
                    val pressedAt = System.nanoTime()
                    val fill = scope.launch {
                        progress.animateTo(1f, tween(DECK_HOLD_MS, easing = LinearEasing))
                        haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                        hold?.invoke()
                    }
                    val released = tryAwaitRelease()
                    val fired = fill.isCompleted && !fill.isCancelled
                    fill.cancel()
                    progress.snapTo(0f)
                    val quick = (System.nanoTime() - pressedAt) / NANOS_PER_MILLI < viewConfiguration.longPressTimeoutMillis
                    if (released && !fired && quick) tap()
                })
            }
            .semantics(mergeDescendants = true) {
                role = Role.Button
                onClick { tap(); true }
                onLongClick(label = label) { hold?.invoke(); true }
            }
        else -> Modifier.clickable(enabled = enabled, role = Role.Button, onClick = onTap)
    }
    return progress.value to gesture
}

@Composable
internal fun HoldFill(progress: Float, color: Color, modifier: Modifier = Modifier) {
    if (progress > 0f) Box(modifier.fillMaxHeight().fillMaxWidth(progress).background(color))
}

@Composable
private fun DeckPress(entry: DeckEntry, content: @Composable () -> Unit) {
    val (progress, gesture) = rememberHold(entry.id, entry.enabled, entry.onClick, entry.onHold, entry.label)
    Box(Modifier.fillMaxSize().then(gesture), contentAlignment = Alignment.Center) {
        HoldFill(progress, LocalContentColor.current.copy(alpha = DECK_HOLD_FILL_ALPHA), Modifier.align(Alignment.CenterStart))
        content()
    }
}

internal fun deckIds(shown: Set<String>, armed: Boolean): List<Pair<String, Boolean>> {
    val flying = armed && ("rtl" in shown || "land" in shown)
    val ground = listOf(CHECKLIST, if ("takeoff" in shown) "takeoff" else "arm")
    val chosen = (if (flying) listOf(PAUSE, "rtl", "land") else ground).filter { it in shown }
    val primary = if (flying) "rtl".takeIf { it in chosen } else chosen.lastOrNull()
    return chosen.map { it to (it == primary) }
}

@Composable
internal fun DeckButton(entry: DeckEntry, primary: Boolean, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier.height(DECK_BUTTON_HEIGHT).alpha(if (entry.enabled) 1f else DISABLED_ALPHA),
        shape = MaterialTheme.shapes.large,
        color = if (primary) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerHigh,
        contentColor = if (primary) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
    ) {
        DeckPress(entry) {
            Column(
                verticalArrangement = Arrangement.spacedBy(AircastSpace.s1, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Icon(painterResource(entry.icon), null, Modifier.size(DECK_ICON_SIZE))
                Text(entry.label, style = MaterialTheme.typography.labelLarge, maxLines = 1)
            }
        }
    }
}

private val RAIL_BUTTON_SIZE = 48.dp
private val RAIL_ICON_SIZE = 24.dp
private const val RAIL_SCRIM_ALPHA = 0.45f
private const val RAIL_BORDER_ALPHA = 0.4f

@Composable
internal fun RailDeckButton(entry: DeckEntry, primary: Boolean) {
    Surface(
        modifier = Modifier.size(RAIL_BUTTON_SIZE).alpha(if (entry.enabled) 1f else DISABLED_ALPHA),
        shape = CircleShape,
        color = if (primary) MaterialTheme.colorScheme.primary else Color.Black.copy(alpha = RAIL_SCRIM_ALPHA),
        contentColor = if (primary) MaterialTheme.colorScheme.onPrimary else MaterialTheme.aircast.outdoorForeground,
        border = if (primary) null else BorderStroke(1.dp, MaterialTheme.aircast.outdoorForeground.copy(alpha = RAIL_BORDER_ALPHA)),
    ) {
        DeckPress(entry) { Icon(painterResource(entry.icon), entry.label, Modifier.size(RAIL_ICON_SIZE)) }
    }
}

private const val MORE_COLUMNS = 4
private val MORE_TILE_HEIGHT = 80.dp

internal class MoreTile(
    val label: String,
    @DrawableRes val icon: Int,
    val enabled: Boolean,
    val warning: Boolean = false,
    val onClick: () -> Unit,
)

internal fun guidedIcon(id: String): Int = when (id) {
    "startMission", "continueMission", "resumeMission" -> R.drawable.ic_route
    "cancelRoi" -> R.drawable.ic_close
    PAUSE -> R.drawable.ic_pause
    "landAbort" -> R.drawable.ic_flight_takeoff
    "release" -> R.drawable.ic_download
    "grab" -> R.drawable.ic_upload
    "hold" -> R.drawable.ic_stop_circle
    "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor" -> R.drawable.ic_swap_horiz
    "forceArm" -> R.drawable.ic_bolt
    else -> R.drawable.ic_send
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MoreActionsSheet(tiles: List<MoreTile>, onDismiss: () -> Unit, footer: @Composable () -> Unit) {
    AircastSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier
                .verticalScroll(rememberScrollState())
                .padding(start = AircastSpace.s4, end = AircastSpace.s4, bottom = AircastSpace.s6),
            verticalArrangement = Arrangement.spacedBy(AircastSpace.s3),
        ) {
            Column(Modifier.padding(horizontal = AircastSpace.s2)) {
                Text("More actions", style = MaterialTheme.typography.titleLarge)
                Text("Everything that changes what the drone does", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            tiles.chunked(MORE_COLUMNS).map { row ->
                Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s3)) {
                    row.map { tile ->
                        Surface(
                            onClick = {
                                onDismiss()
                                tile.onClick()
                            },
                            enabled = tile.enabled,
                            modifier = Modifier.weight(1f).height(MORE_TILE_HEIGHT).alpha(if (tile.enabled) 1f else DISABLED_ALPHA),
                            shape = MaterialTheme.shapes.large,
                            color = MaterialTheme.colorScheme.surfaceContainerHigh,
                            contentColor = if (tile.warning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface,
                        ) {
                            Column(
                                Modifier.padding(horizontal = AircastSpace.s1),
                                verticalArrangement = Arrangement.spacedBy(AircastSpace.s2, Alignment.CenterVertically),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                Icon(painterResource(tile.icon), null, Modifier.size(24.dp))
                                Text(sentenceCase(tile.label), style = MaterialTheme.typography.labelMedium, maxLines = 2, textAlign = TextAlign.Center)
                            }
                        }
                    }
                    repeat(MORE_COLUMNS - row.size) { Spacer(Modifier.weight(1f)) }
                }
            }
            footer()
        }
    }
}
