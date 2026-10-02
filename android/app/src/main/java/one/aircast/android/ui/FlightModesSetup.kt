package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

@Composable
fun FlightModesSetup(modifier: Modifier = Modifier) {
    val view by qgcPath(MODE_SLOTS)
    val slots = modeSlotsView(view)
    val live = listOfNotNull(liveSlotText(slots), liveSwitchesText(slots)).joinToString("\n").ifBlank { null }

    Column(modifier) {
        if (live != null) {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Text(
                    text = live,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
                )
            }
        }
        ParameterForm(FLIGHT_MODES_PAGE, Modifier.fillMaxWidth(), highlighted = slots?.activeParams.orEmpty())
        if (slots?.channelMonitor == true) ChannelMonitor()
    }
}

@Composable
private fun ChannelMonitor() {
    val json by qgcPath(RADIO_VIEW)
    val channels = radioView(json)?.channels.orEmpty()
    androidx.compose.runtime.LaunchedEffect(Unit) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.Default) { one.aircast.android.bridge.Qgc.invoke(radioCalAction("start")) }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp)) {
        Text("Channel monitor", style = MaterialTheme.typography.titleSmall)
        channels.forEach { channel ->
            Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(channel.label, style = MaterialTheme.typography.bodySmall, modifier = Modifier.width(28.dp))
                PwmBar(channel.fraction, Modifier.weight(1f))
            }
        }
    }
}
