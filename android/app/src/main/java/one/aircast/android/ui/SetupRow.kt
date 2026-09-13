package one.aircast.android.ui

import androidx.compose.foundation.clickable
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

internal enum class SetupState { NeedsAttention, Done, Neutral, Unavailable }

@Composable
internal fun setupStateColor(state: SetupState): Color = when (state) {
    SetupState.NeedsAttention -> MaterialTheme.colorScheme.error
    SetupState.Done -> MaterialTheme.colorScheme.primary
    SetupState.Neutral -> MaterialTheme.colorScheme.onSurfaceVariant
    SetupState.Unavailable -> MaterialTheme.colorScheme.onSurfaceVariant
}

@Composable
internal fun SectionHeader(text: String) {
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 8.dp),
    )
}

@Composable
internal fun SetupRow(
    title: String,
    status: String = "",
    state: SetupState = SetupState.Neutral,
    onClick: (() -> Unit)? = null,
) {
    val row = Modifier
        .fillMaxWidth()
        .let { if (onClick == null) it else it.clickable(onClick = onClick) }
        .heightIn(min = 64.dp)
        .padding(horizontal = 20.dp, vertical = 12.dp)

    Row(row, verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontSize = 18.sp,
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
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = if (state == SetupState.NeedsAttention) FontWeight.Bold else FontWeight.Normal,
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
}

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
