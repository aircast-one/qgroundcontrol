package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.ui.graphics.Color
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

    val urgent = MaterialTheme.colorScheme.error
    val summaryColour = when (trafficLevel(reading)) {
        TrafficLevel.Critical, TrafficLevel.Warning -> urgent
        TrafficLevel.Caution -> MaterialTheme.colorScheme.tertiary
        TrafficLevel.Good -> MaterialTheme.colorScheme.onSurface
    }

    Surface(
        modifier = modifier.clickable { expanded = !expanded },
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
    ) {
        Column(
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp).widthIn(max = 320.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                text = listOf(trafficSummary(reading), trafficEmergencyText(reading.emergency))
                    .filter { it.isNotBlank() }
                    .joinToString(" · "),
                style = MaterialTheme.typography.labelLarge,
                color = summaryColour,
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
                    val colour = if (trafficContactUrgent(contact)) urgent else Color.Unspecified
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            text = contact.name,
                            style = MaterialTheme.typography.labelMedium,
                            color = colour,
                            maxLines = 1,
                        )
                        Text(
                            text = trafficContactText(contact, reading.units),
                            style = MaterialTheme.typography.bodySmall,
                            color = colour,
                            maxLines = 2,
                            modifier = Modifier.weight(1f),
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
