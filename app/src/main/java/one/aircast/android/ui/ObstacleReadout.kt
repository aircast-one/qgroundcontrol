package one.aircast.android.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

private const val OBSTACLE = "view.obstacle"

@Composable
fun ObstacleReadout(modifier: Modifier = Modifier) {
    val view by qgcPath(OBSTACLE)
    val warning = obstacleWarning(view) ?: return

    Surface(
        modifier = modifier,
        color = when {
            warning.close -> MaterialTheme.colorScheme.errorContainer
            else -> MaterialTheme.colorScheme.surfaceVariant
        },
    ) {
        Text(
            text = warning.label,
            style = MaterialTheme.typography.labelLarge,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
        )
    }
}
