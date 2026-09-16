package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import one.aircast.mapspike.optText

private const val HOST_FACT = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

internal const val SUPPORT_HOST_DEBOUNCE_MS = 250L

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
private fun StartForwardingDialog(host: String, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Forward telemetry to $host?") },
        text = {
            Text(
                "This sends your vehicle's live MAVLink telemetry, including its position, " +
                    "to an ArduPilot support engineer for as long as the link stays up.",
            )
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(); onDismiss() }) { Text("Start forwarding") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
fun RemoteSupportScreen(modifier: Modifier = Modifier) {
    val linksJson by qgcPath("view.links")
    val forwarding = linksJson?.optBoolean("supportForwarding") == true
    var confirming by remember { mutableStateOf(false) }
    val json by qgcPath(HOST_FACT)
    val host: Fact? = json?.let { Qgc.factAt(HOST_FACT, it) }
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

    if (confirming && host != null) {
        StartForwardingDialog(
            host = host.valueString,
            onConfirm = { offMainDetached { Qgc.invoke("links.createMavlinkForwardingSupportLink") } },
            onDismiss = { confirming = false },
        )
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {

        if (host == null) {
            Text("Reading the support address.", style = MaterialTheme.typography.bodyMedium)
            return@Column
        }

        FactRow(host) {}

        HorizontalDivider()

        Text(
            text = if (forwarding) "Forwarding" else "Not forwarding",
            style = MaterialTheme.typography.titleMedium,
            color = if (forwarding) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )

        if (forwarding) {
            Button(
                onClick = {
                    offMainDetached { Qgc.invoke("links.endMavlinkForwardingSupportLink") }
                },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Stop forwarding") }
        } else {
            Button(
                onClick = { confirming = true },
                enabled = verdict?.valid == true,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Start forwarding") }
        }

        if (!forwarding && verdict?.valid != true) {
            Text(
                text = verdict?.error?.ifBlank { null }
                    ?: "Enter the address your support engineer gave you first.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        FootNote(
            "Sends live telemetry, including position, to an ArduPilot support " +
                "engineer for as long as the link stays up.",
        )
    }
}
