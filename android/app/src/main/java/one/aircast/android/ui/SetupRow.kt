package one.aircast.android.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import one.aircast.map.aircast
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.Spacer
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalContext
import coil3.compose.AsyncImage
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

internal enum class SetupState { NeedsAttention, Done, Neutral, Unavailable }

@Composable
internal fun setupStateColor(state: SetupState): Color = when (state) {
    SetupState.NeedsAttention -> MaterialTheme.aircast.warning
    SetupState.Done -> MaterialTheme.aircast.success
    SetupState.Neutral -> MaterialTheme.colorScheme.onSurfaceVariant
    SetupState.Unavailable -> MaterialTheme.colorScheme.onSurfaceVariant
}

@Composable
internal fun SectionHeader(text: String, image: String = "") {
    val band = LocalSettingsList.current
    Row(
        if (band) {
            Modifier.padding(top = 12.dp).fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh).padding(horizontal = 16.dp, vertical = 10.dp)
        } else {
            Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp)
        },
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
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
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
    selected: Boolean = false,
) {
    val row = Modifier
        .fillMaxWidth()
        .background(if (selected) MaterialTheme.colorScheme.surfaceContainerHighest else Color.Transparent)
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
        summaryGlanceParts(summary).takeIf { it.isNotEmpty() }?.let {
            Text(glanceText(it, MaterialTheme.colorScheme.error), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
    if (status.isNotBlank()) {
        Text(
            text = status,
            style = MaterialTheme.typography.labelMedium,
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

internal val LocalTwoPane = staticCompositionLocalOf { false }

internal class PageHeading(val title: String, val back: () -> Unit)

internal val LocalPageHeading = staticCompositionLocalOf<androidx.compose.runtime.MutableState<PageHeading?>?> { null }

@Composable
internal fun OverridePageHeading(title: String, onBack: () -> Unit) {
    val host = LocalPageHeading.current
    val back by androidx.compose.runtime.rememberUpdatedState(onBack)
    androidx.compose.runtime.DisposableEffect(host, title) {
        host?.value = PageHeading(title) { back() }
        onDispose { host?.value = null }
    }
}

@Composable
internal fun AdvancedToggle(open: Boolean, modifier: Modifier = Modifier, label: String = "Advanced", onToggle: () -> Unit) {
    androidx.compose.material3.TextButton(onClick = onToggle, modifier = modifier.padding(horizontal = 8.dp, vertical = 8.dp)) {
        Text(if (open) "Hide ${label.replaceFirstChar { it.lowercase() }}" else label)
        Icon(
            if (open) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown,
            contentDescription = null,
        )
    }
}

@Composable
internal fun PageTopBar(title: String, backLabel: String, onBack: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 64.dp).padding(horizontal = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (LocalTwoPane.current) Spacer(Modifier.width(12.dp)) else IconButton(onClick = onBack) { Icon(painterResource(R.drawable.ic_arrow_back), backLabel) }
        Text(title, style = MaterialTheme.typography.titleLarge, maxLines = 1)
    }
}

@Composable
internal fun EmptyState(@DrawableRes icon: Int, title: String, text: String, modifier: Modifier = Modifier) {
    Column(
        modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier.size(96.dp).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(painterResource(icon), null, tint = MaterialTheme.colorScheme.onSecondaryContainer, modifier = Modifier.size(48.dp))
        }
        Text(sentenceCase(title), style = MaterialTheme.typography.titleLarge, textAlign = TextAlign.Center)
        if (text.isNotBlank()) {
            Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        }
    }
}

internal data class SummaryLine(val label: String, val value: String, val warn: Boolean = false)

private val UNREAD_VALUES = setOf("Unknown", "--", "")
private const val GLANCE_SEPARATOR = " \u00b7 "

internal fun summaryGlanceParts(summary: List<SummaryLine>): List<SummaryLine> {
    val read = summary.map { it.copy(label = it.label.trimEnd(':', ' '), value = it.value.trim().replace(Regex("(\\S)\\("), "$1 (")) }.filterNot { it.value in UNREAD_VALUES }
    val shared = read.map { it.value.substringBefore(' ', "") }.distinct().singleOrNull()?.takeIf { it.isNotEmpty() && read.size > 1 }
    val parts = shared?.let { prefix -> read.map { it.copy(value = "${it.label} ${it.value.removePrefix(prefix).trim()}") } } ?: read.distinctBy { it.value }
    return parts.map { it.copy(value = sentenceCase(it.value)) }
}

internal fun summaryGlance(summary: List<SummaryLine>): String? =
    summaryGlanceParts(summary).takeIf { it.isNotEmpty() }?.joinToString(GLANCE_SEPARATOR) { it.value }

private fun glanceText(parts: List<SummaryLine>, warn: Color): AnnotatedString =
    parts.map { AnnotatedString(it.value, if (it.warn) SpanStyle(color = warn) else SpanStyle()) }
        .reduce { line, part -> line + AnnotatedString(GLANCE_SEPARATOR) + part }

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

@Composable
internal fun SearchPill(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
) {
    TextField(
        value = value,
        onValueChange = onValueChange,
        placeholder = { Text(placeholder) },
        leadingIcon = { Icon(painterResource(R.drawable.ic_search), null) },
        singleLine = true,
        shape = CircleShape,
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
        colors = TextFieldDefaults.colors(
            focusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
            unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
            focusedIndicatorColor = Color.Transparent,
            unfocusedIndicatorColor = Color.Transparent,
        ),
        modifier = modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
    )
}
