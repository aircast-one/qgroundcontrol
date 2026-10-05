package one.aircast.android.ui

import androidx.compose.foundation.background
import one.aircast.android.bridge.SetupCommands
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.VEHICLES_VIEW
import one.aircast.map.vehicleChoices

private const val CONNECTING_POLL_MS = 500L

internal fun connectingTitle(name: String?): String =
    name?.takeIf { it.isNotBlank() }?.let { "Connecting to $it" } ?: "Connecting"

@Composable
internal fun ConnectingCard(modifier: Modifier = Modifier) {
    var loading by remember { mutableStateOf<Float?>(null) }
    var dismissed by remember { mutableStateOf(false) }
    val vehiclesJson by qgcPath(VEHICLES_VIEW)
    val name = remember(vehiclesJson) { vehicleChoices(vehiclesJson).active?.name }

    LaunchedEffect(Unit) {
        while (isActive) {
            loading = withContext(Dispatchers.Default) { loadingProgress(SetupCommands.vehicleFields(listOf("initialConnectComplete", "loadProgress"))) }
            delay(CONNECTING_POLL_MS)
        }
    }
    LaunchedEffect(loading == null) { if (loading == null) dismissed = false }

    val progress = loading?.takeIf { !dismissed } ?: return
    Surface(
        modifier.fillMaxWidth().padding(16.dp),
        shape = MaterialTheme.shapes.extraLarge,
        color = MaterialTheme.colorScheme.surfaceContainer,
    ) {
        Column(
            Modifier.padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Box(
                Modifier.size(72.dp).background(MaterialTheme.colorScheme.primaryContainer, CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                Icon(painterResource(R.drawable.ic_link), null, tint = MaterialTheme.colorScheme.onPrimaryContainer, modifier = Modifier.size(36.dp))
            }
            Text(connectingTitle(name), style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
            Text("Loading its settings.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(Modifier.fillMaxWidth()) {
                Text("Parameters", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
                Text("${(progress * 100).toInt()}%", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            LinearProgressIndicator(progress = { progress }, modifier = Modifier.fillMaxWidth())
            TextButton(onClick = { dismissed = true }) { Text("Fly now, finish loading in background") }
        }
    }
}
