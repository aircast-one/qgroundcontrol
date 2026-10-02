package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

internal fun sentIsStillShowing(name: String?, snapshotAtSend: String?, live: String?): Boolean =
    name != null && snapshotAtSend != null && snapshotAtSend == live

internal fun sentText(name: String): String = "Sent · $name"

internal fun slideLabel(name: String): String = name.ifBlank { null }?.let { "Slide to ${it.lowercase()}" } ?: "Slide to confirm"

@Composable
internal fun ConfirmTrack(
    action: GuidedAction,
    onSent: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(action.name, style = MaterialTheme.typography.titleLarge)
        if (action.confirm.isNotBlank()) {
            Text(action.confirm, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        SlideToConfirm(
            label = slideLabel(action.name),
            destructive = action.destructive,
            modifier = Modifier.padding(top = 8.dp),
        ) {
            action.run()
            onSent()
        }
        TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
    }
}

@Composable
internal fun SentNotice(name: String, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    Text(
        text = sentText(name),
        style = MaterialTheme.typography.bodyMedium,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.primary,
        modifier = modifier.fillMaxWidth().clickable { onDismiss() },
    )
}
