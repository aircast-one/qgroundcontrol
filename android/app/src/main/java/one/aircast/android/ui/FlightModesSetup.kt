package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

@Composable
fun FlightModesSetup(modifier: Modifier = Modifier) {
    val view by qgcPath(MODE_SLOTS)
    val live = liveSlotText(modeSlotsView(view))

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
        ParameterForm(FLIGHT_MODES_PAGE, Modifier.fillMaxWidth())
    }
}
