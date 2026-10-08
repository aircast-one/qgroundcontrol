package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSheet
import one.aircast.map.aircast

private val TRAFFIC_BANNER_MAX_WIDTH = 560.dp
private val TRAFFIC_BANNER_CORNER = 12.dp

@Composable
internal fun TrafficBanner(modifier: Modifier = Modifier) {
    val view by qgcPath(TRAFFIC_VIEW)
    val reading = remember(view) { trafficReading(view) }
    var listed by remember { mutableStateOf(false) }
    OpenOnRequest(TRAFFIC_SHEET) { listed = true }
    reading?.takeIf { listed && it.contacts.isNotEmpty() }?.let { TrafficSheet(it) { listed = false } }

    val alert = reading?.let(::trafficAlert) ?: return
    val urgent = alert.level >= TrafficLevel.Warning
    Surface(
        onClick = { listed = true },
        enabled = reading.contacts.isNotEmpty(),
        modifier = modifier.widthIn(max = TRAFFIC_BANNER_MAX_WIDTH).fillMaxWidth(),
        shape = RoundedCornerShape(TRAFFIC_BANNER_CORNER),
        color = if (urgent) MaterialTheme.colorScheme.errorContainer else MaterialTheme.aircast.warningContainer,
        contentColor = if (urgent) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onSurface,
    ) {
        Row(
            Modifier.heightIn(min = 48.dp).padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Icon(
                painterResource(if (urgent) R.drawable.ic_flight else R.drawable.ic_warning),
                contentDescription = null,
                tint = if (urgent) MaterialTheme.colorScheme.error else MaterialTheme.aircast.warning,
                modifier = Modifier.size(24.dp),
            )
            Column(Modifier.weight(1f)) {
                Text(alert.title, style = MaterialTheme.typography.titleSmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (alert.detail.isNotBlank()) {
                    Text(alert.detail, style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TrafficSheet(reading: TrafficReading, onDismiss: () -> Unit) {
    val urgent = MaterialTheme.colorScheme.error
    AircastSheet(onDismissRequest = onDismiss) {
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
