package one.aircast.android.ui

import one.aircast.android.R
import one.aircast.android.bridge.LinkCommands
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Row
import one.aircast.android.bridge.settingControl
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.json.JSONObject
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText

private const val HOST_FACT = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

internal const val SUPPORT_HOST_DEBOUNCE_MS = 250L

internal const val FORWARDING_UNTIL_RESTART = "Forwarding traffic: Mavlink traffic will keep being forwarded until application restarts"

internal data class SupportHostVerdict(val valid: Boolean, val error: String)

internal fun supportHostPath(host: String): String = "view.supportHost($host)"

internal fun supportHostCannotBeAsked(host: String): String? = when {
    host.isBlank() -> null
    host != host.trim() -> "Remove the space before or after the address."
    else -> null
}

internal fun supportHostVerdict(view: JSONObject?): SupportHostVerdict? {
    if (view == null || view.optText("class") != "SupportHost") {
        return null
    }
    return SupportHostVerdict(view.optBoolean("valid"), view.optText("error"))
}

@Composable
fun RemoteSupportScreen(modifier: Modifier = Modifier) {
    val linksJson by qgcPath("view.links")
    val forwarding = linksJson?.optBoolean("supportForwarding") == true
    val json by qgcPath(settingControl(HOST_FACT))
    val host: Fact? = remember(json) { json?.takeIf { it.optText("kind") == "object" }?.let(::factFromControl) }
    var verdict by remember { mutableStateOf<SupportHostVerdict?>(null) }
    val typed = host?.valueString.orEmpty()

    LaunchedEffect(typed) {
        supportHostCannotBeAsked(typed)?.let { refusal ->
            verdict = SupportHostVerdict(false, refusal)
            return@LaunchedEffect
        }
        delay(SUPPORT_HOST_DEBOUNCE_MS)
        verdict = withContext(Dispatchers.Default) {
            supportHostVerdict(Qgc.get(supportHostPath(typed)))
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        EmptyState(
            R.drawable.ic_link,
            if (forwarding) "Forwarding to support" else "Not forwarding",
            "Sends live telemetry, including position, to an ArduPilot support engineer.",
        )

        if (host == null) {
            Text("Reading the support address.", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 16.dp))
            return@Column
        }

        FactRow(host) {}

        if (!forwarding && verdict?.valid != true) {
            Text(
                text = verdict?.error?.ifBlank { null }
                    ?: "Enter the address your support engineer gave you first.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 32.dp),
            )
        }

        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), horizontalArrangement = Arrangement.End) {
            Button(
                onClick = { offMainDetached { LinkCommands.createSupportForwarding() } },
                enabled = !forwarding && verdict?.valid == true,
            ) { Text("Connect") }
        }
        if (forwarding) {
            Text(
                FORWARDING_UNTIL_RESTART,
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
    }
}
