package one.aircast.android.ui

import androidx.annotation.DrawableRes
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
import androidx.compose.ui.platform.LocalContext
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
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
import one.aircast.mapspike.AircastSpace

internal const val CHECKLIST = "checklist"
private const val DISABLED_ALPHA = 0.38f
private val DECK_BUTTON_HEIGHT = 80.dp
private val DECK_ICON_SIZE = 28.dp

internal class DeckEntry(
    val id: String,
    val label: String,
    @DrawableRes val icon: Int,
    val enabled: Boolean,
    val warning: Boolean = false,
    val onClick: () -> Unit,
)

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
        onClick = entry.onClick,
        enabled = entry.enabled,
        modifier = modifier.height(DECK_BUTTON_HEIGHT).alpha(if (entry.enabled) 1f else DISABLED_ALPHA),
        shape = MaterialTheme.shapes.large,
        color = if (primary) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerHigh,
        contentColor = if (primary) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(AircastSpace.s1, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(painterResource(entry.icon), null, Modifier.size(DECK_ICON_SIZE))
            Text(entry.label, style = MaterialTheme.typography.labelLarge, maxLines = 1)
        }
    }
}

private const val SIMPLE_TILE_COUNT = 4
private const val SIMPLE_SCRIM_ALPHA = 0.45f
private val SIMPLE_RETURN_HEIGHT = 96.dp

@Composable
internal fun SimpleTiles(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val chosen = remember { readChosen(context) }
    val displays = remember { readDisplays(context) }
    val view by qgcPath(instrumentsPath(chosen))
    val shown = remember(view, chosen) { if (showsInstruments(chosen)) instruments(view).take(SIMPLE_TILE_COUNT) else emptyList() }
    Column(modifier, verticalArrangement = Arrangement.spacedBy(AircastSpace.s3)) {
        shown.chunked(2).map { pair ->
            Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s4)) {
                pair.map { instrument ->
                    val display = displays[instrument.id] ?: ValueDisplay()
                    Surface(
                        Modifier.weight(1f),
                        shape = MaterialTheme.shapes.large,
                        color = Color.Black.copy(alpha = SIMPLE_SCRIM_ALPHA),
                        contentColor = MaterialTheme.aircast.outdoorForeground,
                    ) {
                        Column(Modifier.padding(horizontal = AircastSpace.s4 - 2.dp, vertical = AircastSpace.s2)) {
                            Text(instrument.label.uppercase(), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1)) {
                                Text(
                                    instrument.value,
                                    style = MaterialTheme.typography.displaySmall.copy(fontFeatureSettings = "tnum"),
                                    color = displayColour(display, instrument.raw)?.let { Color(it) } ?: Color.Unspecified,
                                    modifier = Modifier.alignByBaseline(),
                                )
                                if (instrument.units.isNotBlank()) Text(
                                    instrument.units,
                                    style = MaterialTheme.typography.titleMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.alignByBaseline(),
                                )
                            }
                        }
                    }
                }
                if (pair.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}

@Composable
internal fun SimpleDeck(deck: List<Pair<String, Boolean>>, entries: List<DeckEntry>, onMore: () -> Unit) {
    val chosen = deck.mapNotNull { (id, primary) -> entries.firstOrNull { it.id == id }?.let { it to primary } }
    Column(verticalArrangement = Arrangement.spacedBy(AircastSpace.s3)) {
        Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s3)) {
            chosen.filterNot { it.second }.map { (entry, _) -> SimpleButton(entry, primary = false, Modifier.weight(1f).height(DECK_BUTTON_HEIGHT)) }
            SimpleButton(DeckEntry("more", "More", R.drawable.ic_more_vert, true, onClick = onMore), primary = false, Modifier.width(DECK_BUTTON_HEIGHT).height(DECK_BUTTON_HEIGHT), iconOnly = true)
        }
        chosen.firstOrNull { it.second }?.let { (entry, _) ->
            SimpleButton(entry, primary = true, Modifier.fillMaxWidth().height(SIMPLE_RETURN_HEIGHT))
        }
    }
}

@Composable
private fun SimpleButton(entry: DeckEntry, primary: Boolean, modifier: Modifier, iconOnly: Boolean = false) {
    Surface(
        onClick = entry.onClick,
        enabled = entry.enabled,
        modifier = modifier.alpha(if (entry.enabled) 1f else DISABLED_ALPHA),
        shape = MaterialTheme.shapes.extraLarge,
        color = if (primary) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest,
        contentColor = if (primary) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s3, Alignment.CenterHorizontally),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(painterResource(entry.icon), if (iconOnly) entry.label else null, Modifier.size(if (primary) 36.dp else 32.dp))
            if (!iconOnly) Text(entry.label, style = if (primary) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.titleLarge, maxLines = 1)
        }
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
    "release", "grab", "hold" -> R.drawable.ic_tune
    "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor" -> R.drawable.ic_swap_horiz
    "forceArm" -> R.drawable.ic_bolt
    else -> R.drawable.ic_send
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MoreActionsSheet(tiles: List<MoreTile>, onDismiss: () -> Unit, footer: @Composable () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
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
                                Text(tile.label, style = MaterialTheme.typography.labelLarge, maxLines = 2, textAlign = TextAlign.Center)
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
