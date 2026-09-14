package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

private const val LISTED = 5

@Composable
fun TrafficReadout(modifier: Modifier = Modifier) {
    val view by qgcPath(TRAFFIC_VIEW)
    val reading = remember(view) { trafficReading(view) } ?: return
    var expanded by remember { mutableStateOf(false) }

    if (!trafficShown(reading)) {
        return
    }

    val level = trafficLevel(reading)
    Surface(
        modifier = modifier.clickable { expanded = !expanded },
        color = when (level) {
            TrafficLevel.Critical, TrafficLevel.Warning -> MaterialTheme.colorScheme.errorContainer
            TrafficLevel.Caution -> MaterialTheme.colorScheme.surfaceVariant
            TrafficLevel.Good -> MaterialTheme.colorScheme.surface
        },
    ) {
        Column(
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp).widthIn(max = 260.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                text = listOf(trafficSummary(reading), trafficEmergencyText(reading.emergency))
                    .filter { it.isNotBlank() }
                    .joinToString(" · "),
                style = MaterialTheme.typography.labelLarge,
            )

            if (expanded) {
                if (reading.contacts.isNotEmpty()) {
                    Text(
                        text = trafficCaption(reading),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                reading.contacts.take(LISTED).forEach { contact ->
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            text = contact.name,
                            style = MaterialTheme.typography.labelMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Text(
                            text = trafficContactText(contact, reading.units),
                            style = MaterialTheme.typography.bodySmall,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                if (reading.contacts.size > LISTED) {
                    Text(
                        text = "${reading.contacts.size - LISTED} more",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        }
    }
}
