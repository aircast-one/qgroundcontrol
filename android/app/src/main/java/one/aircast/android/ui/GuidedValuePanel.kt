package one.aircast.android.ui

import androidx.compose.foundation.layout.padding

import androidx.compose.material3.Surface

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
internal fun GuidedValuePanel(
    title: String,
    sentence: String,
    commitLabel: String,
    commitEnabled: Boolean,
    onCommit: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    Surface(
        modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        shape = MaterialTheme.shapes.extraLarge,
    ) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 20.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        if (sentence.isNotBlank()) {
            Text(sentence, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        content()
        SlideToConfirm(label = slideLabel(commitLabel), enabled = commitEnabled, onConfirm = onCommit)
        TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
    }
    }
}
