package one.aircast.android.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

@Composable
fun FollowMeReadout(modifier: Modifier = Modifier) {
    val view by qgcPath(FOLLOW_ME_VIEW)
    val reading = remember(view) { followMeReading(view) }
    val label = remember(reading) { followMeLabel(reading) } ?: return
    val sending = reading?.wouldSend == true

    Surface(
        modifier = modifier,
        color = when {
            sending -> MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.92f)
            else -> MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.92f)
        },
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            maxLines = 2,
            modifier = Modifier.widthIn(max = 300.dp).padding(horizontal = 10.dp, vertical = 6.dp),
        )
    }
}
