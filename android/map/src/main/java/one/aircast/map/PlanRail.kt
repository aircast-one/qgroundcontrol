package one.aircast.map

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

internal val RAIL_WIDTH = 64.dp
private val RAIL_ITEM_WIDTH = 56.dp
private val RAIL_ICON = 22.dp
private const val RAIL_ALPHA = 0.94f
private const val DISABLED_ALPHA = 0.38f

@Composable
internal fun PlanRail(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Surface(
        modifier,
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = RAIL_ALPHA),
    ) {
        Column(
            Modifier.width(RAIL_WIDTH).verticalScroll(rememberScrollState()).padding(vertical = 4.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            content = content,
        )
    }
}

@Composable
internal fun RailButton(
    @DrawableRes icon: Int,
    label: String,
    onClick: () -> Unit,
    enabled: Boolean = true,
    chosen: Boolean = false,
) {
    Surface(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(14.dp),
        color = if (chosen) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
        contentColor = if (chosen) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
        modifier = Modifier.padding(vertical = 2.dp).alpha(if (enabled) 1f else DISABLED_ALPHA).semantics { selected = chosen },
    ) {
        Column(Modifier.width(RAIL_ITEM_WIDTH).padding(vertical = 6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(painterResource(icon), contentDescription = null, modifier = Modifier.size(RAIL_ICON))
            Text(label, style = MaterialTheme.typography.labelSmall, maxLines = 1, overflow = TextOverflow.Clip)
        }
    }
}

@Composable
internal fun RailDivider() {
    HorizontalDivider(Modifier.width(32.dp).padding(vertical = 4.dp))
}
