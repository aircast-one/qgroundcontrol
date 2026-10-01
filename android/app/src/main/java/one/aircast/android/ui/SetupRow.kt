package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

internal enum class SetupState { NeedsAttention, Done, Neutral, Unavailable }

@Composable
internal fun setupStateColor(state: SetupState): Color = when (state) {
    SetupState.NeedsAttention -> MaterialTheme.aircast.warning
    SetupState.Done -> MaterialTheme.colorScheme.primary
    SetupState.Neutral -> MaterialTheme.colorScheme.onSurfaceVariant
    SetupState.Unavailable -> MaterialTheme.colorScheme.onSurfaceVariant
}

@Composable
internal fun SectionHeader(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp),
    )
}

@Composable
internal fun SetupRow(
    title: String,
    status: String = "",
    state: SetupState = SetupState.Neutral,
    onClick: (() -> Unit)? = null,
    summary: List<SummaryLine> = emptyList(),
    @DrawableRes icon: Int? = null,
) {
    val row = Modifier
        .fillMaxWidth()
        .let { if (onClick == null) it else it.clickable(onClick = onClick) }
        .heightIn(min = 72.dp)
        .padding(horizontal = 16.dp, vertical = 12.dp)

    Row(row, horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
    icon?.let {
        Box(
            Modifier.size(40.dp).background(
                if (state == SetupState.NeedsAttention) MaterialTheme.aircast.warningContainer else MaterialTheme.colorScheme.secondaryContainer,
                CircleShape,
            ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                painterResource(it),
                null,
                tint = if (state == SetupState.NeedsAttention) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSecondaryContainer,
                modifier = Modifier.size(24.dp),
            )
        }
    }
    Column(Modifier.weight(1f)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium,
                color = if (state == SetupState.Unavailable) {
                    MaterialTheme.colorScheme.onSurfaceVariant
                } else {
                    MaterialTheme.colorScheme.onSurface
                },
                modifier = Modifier.weight(1f),
            )
            if (status.isNotBlank()) {
                Text(
                    text = status,
                    style = MaterialTheme.typography.labelLarge,
                    color = setupStateColor(state),
                )
            }
            if (onClick != null) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }
        summary.forEach { line ->
            Row(Modifier.fillMaxWidth().padding(top = 2.dp)) {
                Text(line.label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
                Text(line.value, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
    }
}

internal data class SummaryLine(val label: String, val value: String)

@Composable
internal fun FootNote(text: String) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
