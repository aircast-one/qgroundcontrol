package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import org.json.JSONObject
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText

internal const val MESSAGES = "view.messages"

enum class MessageSeverity { Error, Warning, Normal }

data class VehicleMessage(
    val index: Int,
    val time: String,
    val severity: String,
    val level: MessageSeverity,
    val text: String,
)

internal fun bannerText(blocker: String?, messages: List<VehicleMessage>): String? {
    if (blocker != null) return blocker
    if (messages.isEmpty()) return null
    val worst = messages.lastOrNull { it.level == MessageSeverity.Error }
        ?: messages.lastOrNull { it.level == MessageSeverity.Warning }
    val count = "${messages.size} message${if (messages.size == 1) "" else "s"} from the vehicle"
    return worst?.text?.takeIf { it.isNotBlank() }?.let { "$it · $count" } ?: count
}

internal fun levelOf(name: String): MessageSeverity = when (name) {
    "error" -> MessageSeverity.Error
    "warning" -> MessageSeverity.Warning
    else -> MessageSeverity.Normal
}

internal const val OLDEST_FIRST = "oldestFirst"

internal fun vehicleMessages(view: JSONObject?): List<VehicleMessage> {
    val items = view?.optJSONArray("items") ?: return emptyList()
    val read = (0 until items.length()).mapNotNull { at ->
        items.optJSONObject(at)?.let { item ->
            val text = item.optText("text")
            if (text.isBlank()) {
                null
            } else {
                VehicleMessage(
                    index = item.optInt("index", at),
                    time = item.optText("time"),
                    severity = item.optText("severity"),
                    level = levelOf(item.optText("level")),
                    text = text,
                )
            }
        }
    }
    return if (view.optText("order") == OLDEST_FIRST) read else read.asReversed()
}


private const val WARNINGS = "view.warnings"

internal fun armingBlocker(view: JSONObject?): String? =
    view?.takeIf { !it.isNull("armingBlocker") }
        ?.optText("armingBlocker")
        ?.ifBlank { null }

internal data class ArmingCheck(val message: String, val description: String, val severity: String)

internal fun armingChecks(view: JSONObject?): List<ArmingCheck>? {
    val listed = view?.takeIf { !it.isNull("armingChecks") }?.optJSONArray("armingChecks") ?: return null
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let { problem ->
            ArmingCheck(
                message = problem.optText("message"),
                description = problem.optText("description"),
                severity = problem.optText("severity"),
            )
        }
    }.filter { it.message.isNotBlank() }
}

@Composable
fun VehicleMessageBanner(modifier: Modifier = Modifier) {
    val warnings by qgcPath(WARNINGS)
    val messagesJson by qgcPath(MESSAGES)
    val messages = remember(messagesJson) { vehicleMessages(messagesJson) }

    var showing by remember { mutableStateOf(false) }

    val blocker = armingBlocker(warnings)
    val checks = remember(warnings) { armingChecks(warnings).orEmpty() }
    val count = messages.size
    val hasError = messages.any { it.level == MessageSeverity.Error }
    val hasWarning = messages.any { it.level == MessageSeverity.Warning }

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
            text = bannerText(blocker, messages).orEmpty(),
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = if (urgent) FontWeight.Bold else FontWeight.Normal,
            color = tint,
            modifier = Modifier.weight(1f),
        )
    }

    if (showing) {
        VehicleMessageLog(messages = messages, checks = checks, onDismiss = { showing = false })
    }
}

@Composable
private fun VehicleMessageLog(
    messages: List<VehicleMessage>,
    checks: List<ArmingCheck>,
    onDismiss: () -> Unit,
) {
    val lines = remember(messages) { messages.asReversed() }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (checks.isEmpty()) "Vehicle messages" else "Why it will not arm") },
        text = {
            if (lines.isEmpty() && checks.isEmpty()) {
                Text("The vehicle has not said anything yet.")
            } else {
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    items(checks) { check ->
                        Column(Modifier.padding(vertical = 6.dp)) {
                            Text(
                                text = check.message,
                                style = MaterialTheme.typography.bodySmall,
                                color = if (check.severity == "error") {
                                    MaterialTheme.colorScheme.error
                                } else {
                                    MaterialTheme.colorScheme.tertiary
                                },
                            )
                            if (check.description.isNotBlank()) {
                                Text(
                                    text = check.description,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        androidx.compose.material3.HorizontalDivider()
                    }
                    itemsIndexed(lines) { index, message ->
                        Column(Modifier.padding(vertical = 6.dp)) {
                            Text(
                                text = listOf(message.time, message.text)
                                    .filter { it.isNotBlank() }
                                    .joinToString("  "),
                                style = MaterialTheme.typography.bodySmall,
                                color = when (message.level) {
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
