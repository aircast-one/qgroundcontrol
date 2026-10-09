package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.widthIn
import one.aircast.map.aircast
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.qgcPath
import one.aircast.map.VEHICLES_VIEW
import one.aircast.map.vehicleChoices


internal fun connectingTitle(name: String?): String =
    name?.takeIf { it.isNotBlank() }?.let { "Connecting to $it" } ?: "Connecting"

internal data class LoadingWords(val title: String, val detail: String, val dismiss: String)

internal fun loadingWords(name: String?, lost: Boolean): LoadingWords =
    if (lost) LoadingWords(SIGNAL_LOST, "Loading stopped. It picks up where it left off when the aircraft answers. $LOST_LINK_HINT", "Hide")
    else LoadingWords(connectingTitle(name), "Loading its settings.", "Fly now, finish loading in background")

@Composable
internal fun ConnectingCard(modifier: Modifier = Modifier) {
    val loading = rememberVehicleLoading()
    var dismissed by remember { mutableStateOf(false) }
    val vehiclesJson by qgcPath(VEHICLES_VIEW)
    val name = remember(vehiclesJson) { vehicleChoices(vehiclesJson).active?.name }
    val flyJson by qgcPath(FLY_STATE)
    val lost = remember(flyJson) { flyState(flyJson)?.contactLost == true }
    val words = loadingWords(name, lost)

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
                Icon(painterResource(if (lost) R.drawable.ic_warning else R.drawable.ic_link), null, tint = MaterialTheme.colorScheme.onPrimaryContainer, modifier = Modifier.size(36.dp))
            }
            Text(words.title, style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center, color = if (lost) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            Text(words.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
            Row(Modifier.fillMaxWidth()) {
                Text("Parameters", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
                Text("${(progress * 100).toInt()}%", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            LinearProgressIndicator(progress = { progress }, modifier = Modifier.fillMaxWidth(), color = if (lost) MaterialTheme.colorScheme.outline else androidx.compose.material3.ProgressIndicatorDefaults.linearColor)
            TextButton(onClick = { dismissed = true }) { Text(words.dismiss) }
        }
    }
}

internal const val LOOKING_TITLE = "Looking for your aircraft"
internal const val LOOKING_HINT = "Turn on the aircraft. A USB cable or telemetry radio connects by itself; for Wi-Fi, add a link."
private const val CONNECTION_SETTINGS = "Connections"

@Composable
internal fun LookingForAircraft() {
    val navigation = LocalAppNavigation.current
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            Modifier.widthIn(max = LOOKING_MAX_WIDTH).padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            androidx.compose.material3.CircularProgressIndicator(Modifier.size(28.dp), strokeWidth = 3.dp, color = MaterialTheme.aircast.outdoorForeground)
            Text(LOOKING_TITLE, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.aircast.outdoorForeground, textAlign = TextAlign.Center)
            Text(LOOKING_HINT, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.outdoorForeground, textAlign = TextAlign.Center)
            TextButton(onClick = { navigation.settingsPage = CONNECTION_SETTINGS }) { Text("Add a link") }
            TextButton(onClick = { navigation.settingsPage = FLASHER_PAGE }) { Text("New drone? Flash an Aircast card") }
        }
    }
}

private val LOOKING_MAX_WIDTH = 360.dp
