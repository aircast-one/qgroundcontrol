package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
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
