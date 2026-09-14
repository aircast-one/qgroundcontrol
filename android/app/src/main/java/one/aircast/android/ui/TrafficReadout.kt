package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrafficReadout(modifier: Modifier = Modifier) {
    val view by qgcPath(TRAFFIC_VIEW)
    val reading = remember(view) { trafficReading(view) } ?: return
    var listed by remember { mutableStateOf(false) }

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
        modifier = modifier.clickable(enabled = reading.contacts.isNotEmpty()) { listed = true },
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
    ) {
        Text(
            text = listOf(trafficSummary(reading), trafficEmergencyText(reading.emergency))
                .filter { it.isNotBlank() }
                .joinToString(" · "),
            style = MaterialTheme.typography.labelLarge,
            color = summaryColour,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
        )
    }

    if (listed && reading.contacts.isNotEmpty()) {
        ModalBottomSheet(onDismissRequest = { listed = false }) {
            Text(
                trafficSummary(reading),
                Modifier.padding(horizontal = 20.dp),
                style = MaterialTheme.typography.titleSmall,
            )
            Text(
                trafficCaption(reading),
                Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            LazyColumn(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
                items(reading.contacts, key = { it.icaoAddress }) { contact ->
                    val colour = if (trafficContactUrgent(contact)) urgent else Color.Unspecified
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Text(
                            contact.name,
                            style = MaterialTheme.typography.labelLarge,
                            color = colour,
                        )
                        Text(
                            trafficContactText(contact, reading.units),
                            style = MaterialTheme.typography.bodyMedium,
                            color = colour,
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            }
        }
    }
}
