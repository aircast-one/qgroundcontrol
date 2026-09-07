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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath

private const val HOST_FACT = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

internal fun supportHostIsUsable(host: String) = host.isNotBlank() && !host.contains(' ')

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
    val forwarding by qgcBool("links.mavlinkSupportForwardingEnabled")
    var reloads by remember { mutableIntStateOf(0) }
    var confirming by remember { mutableStateOf(false) }
    val json by qgcPath(HOST_FACT)
    val host: Fact? = json?.let { Qgc.factAt(HOST_FACT, it) }

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
        Text(
            text = "Forwards this vehicle's MAVLink telemetry to an ArduPilot support " +
                "engineer so they can see what you see.",
            style = MaterialTheme.typography.bodyMedium,
        )

        HorizontalDivider()

        if (host == null) {
            Text("Reading the support address.", style = MaterialTheme.typography.bodyMedium)
            return@Column
        }

        FactRow(host) { reloads++ }

        HorizontalDivider()

        Text(
            text = if (forwarding) {
                "Forwarding is running. Remove the forwarding link from Comm Links to stop it."
            } else {
                "Not forwarding."
            },
            style = MaterialTheme.typography.bodyMedium,
        )

        Button(
            onClick = { confirming = true },
            enabled = !forwarding && supportHostIsUsable(host.valueString),
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Start forwarding") }

        if (!supportHostIsUsable(host.valueString)) {
            Text(
                text = "Enter the address your support engineer gave you first.",
                style = MaterialTheme.typography.bodySmall,
            )
        }
    }
}
