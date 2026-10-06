package one.aircast.android.ui

import androidx.compose.ui.draw.alpha

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp

private const val DISABLED_HOLD_ALPHA = 0.38f

private val TRACK_HEIGHT = 64.dp

private const val HOLD_TRACK_FILL_ALPHA = 0.45f

internal fun holdLabel(name: String): String = name.ifBlank { null }?.let { "Hold to ${it.lowercase()}" } ?: "Hold to confirm"

@Composable
fun HoldToConfirm(
    label: String,
    modifier: Modifier = Modifier,
    destructive: Boolean = false,
    enabled: Boolean = true,
    onConfirm: () -> Unit,
) {
    val accent = if (destructive) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary
    val track = if (destructive) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.primaryContainer
    val onTrack = if (destructive) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onPrimaryContainer
    val (progress, gesture) = rememberHold(label, enabled, onTap = {}, onHold = onConfirm, label = label)
    Box(
        modifier
            .fillMaxWidth()
            .height(TRACK_HEIGHT)
            .alpha(if (enabled) 1f else DISABLED_HOLD_ALPHA)
            .clip(CircleShape)
            .background(track)
            .then(gesture),
        contentAlignment = Alignment.Center,
    ) {
        HoldFill(progress, accent.copy(alpha = HOLD_TRACK_FILL_ALPHA), Modifier.align(Alignment.CenterStart))
        Text(text = label, style = MaterialTheme.typography.labelLarge, color = onTrack)
    }
}
