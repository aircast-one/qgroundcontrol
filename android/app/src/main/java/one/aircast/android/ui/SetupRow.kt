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
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalContext
import coil3.compose.AsyncImage
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
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
internal fun SectionHeader(text: String, image: String = "") {
    Row(
        Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (image.isNotBlank()) {
            AsyncImage(
                model = sectionImageAsset(image),
                imageLoader = IconLoader.of(LocalContext.current),
                contentDescription = null,
                colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface),
                modifier = Modifier.size(width = 48.dp, height = 24.dp),
            )
        }
        Text(
            text = text,
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.primary,
        )
    }
}

internal fun sectionImageAsset(image: String): String = "file:///android_asset/SetupSections/$image"

@Composable
internal fun SetupRow(
    title: String,
    status: String = "",
    state: SetupState = SetupState.Neutral,
    onClick: (() -> Unit)? = null,
    summary: List<SummaryLine> = emptyList(),
    @DrawableRes icon: Int? = null,
    subtitle: String = "",
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
        Text(
            text = title,
            style = MaterialTheme.typography.bodyLarge,
            color = if (state == SetupState.Unavailable) {
                MaterialTheme.colorScheme.onSurfaceVariant
            } else {
                MaterialTheme.colorScheme.onSurface
            },
        )
        if (subtitle.isNotBlank()) {
            Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        summary.forEach { line ->
            Row(Modifier.fillMaxWidth().padding(top = 2.dp)) {
                Text(line.label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
                Text(line.value, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
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
        )
    }
    }
}

@Composable
internal fun PageTopBar(title: String, backLabel: String, onBack: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 64.dp).padding(horizontal = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClick = onBack) { Icon(painterResource(R.drawable.ic_arrow_back), backLabel) }
        Text(title, style = MaterialTheme.typography.titleLarge, maxLines = 1)
    }
}

@Composable
internal fun EmptyState(@DrawableRes icon: Int, title: String, text: String, modifier: Modifier = Modifier) {
    Column(
        modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            Modifier.size(96.dp).background(MaterialTheme.colorScheme.surfaceContainerHighest, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(painterResource(icon), null, tint = MaterialTheme.colorScheme.onSurface, modifier = Modifier.size(44.dp))
        }
        Text(title, style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
        if (text.isNotBlank()) {
            Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
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
