package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString

private val SEVERITY_TOKEN_RE = Regex("<#[ENI]>")

private val TAG_RE = Regex("<[^>]*>")

enum class MessageSeverity { Error, Warning, Normal }

data class VehicleMessage(val text: String, val severity: MessageSeverity)

private val ENTITIES = listOf(
    "&lt;" to "<",
    "&gt;" to ">",
    "&quot;" to "\"",
    "&#39;" to "'",
    "&nbsp;" to " ",
    "&amp;" to "&",
)

internal fun severityOf(chunk: String): MessageSeverity = when {
    chunk.contains("<#E>") -> MessageSeverity.Error
    chunk.contains("<#I>") -> MessageSeverity.Warning
    else -> MessageSeverity.Normal
}

internal fun vehicleMessages(html: String): List<VehicleMessage> =
    html.split("<br/>", "<br>", ignoreCase = true)
        .mapNotNull { chunk ->
            val stripped = TAG_RE.replace(SEVERITY_TOKEN_RE.replace(chunk, ""), "")
            val text = ENTITIES.fold(stripped) { acc, (from, to) -> acc.replace(from, to) }.trim()
            if (text.isBlank()) null else VehicleMessage(text, severityOf(chunk))
        }

internal fun vehicleMessageLines(html: String): List<String> = vehicleMessages(html).map { it.text }

internal fun flightBlocker(
    armed: Boolean,
    prearmError: String,
    allSensorsHealthy: Boolean,
    readyToFlyAvailable: Boolean,
    readyToFly: Boolean,
    requiresGpsFix: Boolean,
    hasPositionFix: Boolean,
): String? = when {
    armed -> null
    prearmError.isNotBlank() -> prearmError
    requiresGpsFix && !hasPositionFix -> "No GPS lock. This vehicle needs a position fix before it will arm."
    !allSensorsHealthy -> "A sensor is reporting unhealthy. The vehicle will refuse to arm."
    readyToFlyAvailable && !readyToFly -> "The vehicle is not ready to fly yet."
    else -> null
}

@Composable
fun VehicleMessageBanner(modifier: Modifier = Modifier) {
    val armed by qgcBool("vehicle.armed")
    val prearmError by qgcString("vehicle.prearmError")
    val allSensorsHealthy by qgcBool("vehicle.allSensorsHealthy")
    val readyToFlyAvailable by qgcBool("vehicle.readyToFlyAvailable")
    val readyToFly by qgcBool("vehicle.readyToFly")
    val requiresGpsFix by qgcBool("vehicle.requiresGpsFix")
    val coordinate by qgcPath("vehicle.coordinate")
    val messageCount by qgcDouble("vehicle.messageCount", 0.0)
    val hasError by qgcBool("vehicle.messageTypeError")
    val hasWarning by qgcBool("vehicle.messageTypeWarning")

    var showing by remember { mutableStateOf(false) }

    val blocker = flightBlocker(
        armed, prearmError, allSensorsHealthy, readyToFlyAvailable, readyToFly,
        requiresGpsFix, coordinate?.optBoolean("valid") == true,
    )
    val count = messageCount.toInt()

    if (blocker == null && count == 0) return

    val urgent = blocker != null || hasError
    val tint = when {
        urgent -> MaterialTheme.colorScheme.error
        hasWarning -> MaterialTheme.colorScheme.tertiary
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }

    Row(
        modifier
            .fillMaxWidth()
            .clickable { showing = true }
            .heightIn(min = 48.dp)
            .padding(horizontal = 4.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (urgent || hasWarning) {
            Icon(Icons.Filled.Warning, contentDescription = null, tint = tint)
        }
        Text(
            text = blocker ?: "$count message${if (count == 1) "" else "s"} from the vehicle",
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = if (urgent) FontWeight.Bold else FontWeight.Normal,
            color = tint,
            modifier = Modifier.weight(1f),
        )
    }

    if (showing) {
        VehicleMessageLog(onDismiss = { showing = false })
    }
}

@Composable
private fun VehicleMessageLog(onDismiss: () -> Unit) {
    val formatted by qgcString("vehicle.formattedMessages")
    val lines = remember(formatted) { vehicleMessages(formatted).asReversed() }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Vehicle messages") },
        text = {
            if (lines.isEmpty()) {
                Text("The vehicle has not said anything yet.")
            } else {
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    itemsIndexed(lines) { index, message ->
                        Column(Modifier.padding(vertical = 6.dp)) {
                            Text(
                                text = message.text,
                                style = MaterialTheme.typography.bodySmall,
                                color = when (message.severity) {
                                    MessageSeverity.Error -> MaterialTheme.colorScheme.error
                                    MessageSeverity.Warning -> MaterialTheme.colorScheme.tertiary
                                    MessageSeverity.Normal -> MaterialTheme.colorScheme.onSurface
                                },
                            )
                        }
                        if (index < lines.lastIndex) {
                            androidx.compose.material3.HorizontalDivider()
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                offMainDetached { Qgc.invoke("vehicle.clearMessages") }
                onDismiss()
            }) { Text("Clear") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
