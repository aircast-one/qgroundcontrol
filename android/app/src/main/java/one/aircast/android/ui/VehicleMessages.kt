package one.aircast.android.ui

import androidx.compose.foundation.layout.fillMaxSize
import one.aircast.android.bridge.VehicleCommands
import androidx.compose.runtime.LaunchedEffect
import one.aircast.android.R

import androidx.compose.ui.res.painterResource

import androidx.compose.foundation.layout.size

import androidx.compose.material3.Surface
import androidx.compose.material3.LocalContentColor
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Snackbar
import androidx.compose.material3.SnackbarData
import androidx.compose.material3.SnackbarDuration
import androidx.compose.foundation.shape.RoundedCornerShape

import one.aircast.map.AircastSheet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal const val MESSAGES = "view.messages"

enum class MessageSeverity { Error, Warning, Normal }

data class VehicleMessage(
    val index: Int,
    val time: String,
    val severity: String,
    val level: MessageSeverity,
    val text: String,
)

private const val PREARM_PREFIX = "PreArm:"

internal fun chipBlocker(blocker: String): String =
    blocker.trim().removePrefix(PREARM_PREFIX).trim().substringBefore(". ").trimEnd('.')

internal fun bannerMessages(unread: List<VehicleMessage>): List<VehicleMessage> =
    unread.filter { (it.level == MessageSeverity.Error || it.level == MessageSeverity.Warning) && !it.text.trimStart().startsWith(PREARM_PREFIX) }

internal fun bannerText(messages: List<VehicleMessage>): String? {
    if (messages.isEmpty()) return null
    val worst = messages.lastOrNull { it.level == MessageSeverity.Error }
        ?: messages.lastOrNull { it.level == MessageSeverity.Warning }
    val count = messageCountText(messages.size)
    return worst?.text?.takeIf { it.isNotBlank() } ?: count
}

internal fun severitySummary(messages: List<VehicleMessage>): String {
    fun counted(n: Int, one: String) = "$n $one${if (n == 1) "" else "s"}"
    val errors = messages.count { it.level == MessageSeverity.Error }
    val warnings = messages.count { it.level == MessageSeverity.Warning }
    val rest = messages.size - errors - warnings
    return listOfNotNull(
        counted(errors, "error").takeIf { errors > 0 },
        counted(warnings, "warning").takeIf { warnings > 0 },
        counted(rest, "message").takeIf { rest > 0 },
    ).joinToString(" \u00b7 ").ifBlank { messageCountText(0) }
}

internal fun messageCountText(count: Int): String = "$count message${if (count == 1) "" else "s"} from the vehicle"

internal fun unreadMessages(messages: List<VehicleMessage>, unread: Int): List<VehicleMessage> =
    messages.takeLast(unread.coerceAtLeast(0))

internal fun unreadCount(view: JSONObject?): Int = view?.optInt("unread", 0) ?: 0

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
                    time = messageTime(item.optText("time")),
                    severity = item.optText("severity"),
                    level = levelOf(item.optText("level")),
                    text = text,
                )
            }
        }
    }
    return if (view.optText("order") == OLDEST_FIRST) read else read.asReversed()
}


internal const val WARNINGS = "view.warnings"

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

private val CLOCK_WITH_MILLIS = Regex("""^(\d{1,2}:\d{2}:\d{2})\.\d+$""")

internal fun messageTime(served: String): String =
    CLOCK_WITH_MILLIS.matchEntire(served.trim())?.groupValues?.get(1) ?: served

@Composable
fun VehicleMessageBanner(modifier: Modifier = Modifier) {
    val messagesJson by qgcPath(MESSAGES)
    val messages = remember(messagesJson) { vehicleMessages(messagesJson) }
    var showing by remember { mutableStateOf(false) }
    val shown = bannerMessages(unreadMessages(messages, unreadCount(messagesJson)))

    if (showing) VehicleMessagesSheet { showing = false }
    if (shown.isEmpty()) return

    val urgent = shown.any { it.level == MessageSeverity.Error }
    Surface(
        onClick = {
            showing = true
            offMainDetached { VehicleCommands.resetAllMessages() }
        },
        modifier = modifier,
        shape = if (urgent) RoundedCornerShape(ALERT_CORNER) else CircleShape,
        color = osdBackdrop(if (urgent) MaterialTheme.colorScheme.errorContainer else MaterialTheme.aircast.warningContainer),
        contentColor = if (urgent) osdTint(MaterialTheme.colorScheme.onErrorContainer, MaterialTheme.colorScheme.error) else osdTint(MaterialTheme.colorScheme.onSurface, MaterialTheme.aircast.warning),
    ) {
        Row(
            Modifier.heightIn(min = 40.dp).padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Icon(
                painterResource(if (urgent) R.drawable.ic_error else R.drawable.ic_warning),
                null,
                tint = if (urgent) LocalContentColor.current else MaterialTheme.aircast.warning,
                modifier = Modifier.size(24.dp),
            )
            Text(
                text = bannerText(shown) ?: messageCountText(shown.size),
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.weight(1f, fill = false),
            )
            IconButton(onClick = { offMainDetached { VehicleCommands.resetAllMessages() } }, modifier = Modifier.size(32.dp)) {
                Icon(painterResource(R.drawable.ic_close), contentDescription = "Dismiss messages", modifier = Modifier.size(24.dp))
            }
        }
    }
}

@Composable
internal fun VehicleMessagesSheet(onDismiss: () -> Unit) {
    val warnings by qgcPath(WARNINGS)
    val messagesJson by qgcPath(MESSAGES)
    val messages = remember(messagesJson) { vehicleMessages(messagesJson) }
    val checks = remember(warnings) { armingChecks(warnings).orEmpty() }
    VehicleMessageLog(messages = messages, checks = checks, blocker = armingBlocker(warnings), onDismiss = onDismiss)
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun VehicleMessageLog(
    messages: List<VehicleMessage>,
    checks: List<ArmingCheck>,
    blocker: String?,
    onDismiss: () -> Unit,
) {
    val blocking = checks.ifEmpty { listOfNotNull(blocker?.let { ArmingCheck(it, "", "error") }) }
    val lines = remember(messages) { messages.asReversed() }
    var editing by remember { mutableStateOf<String?>(null) }

    editing?.let { name -> ParameterEditDialog(name, EDIT_PARAMETER_TITLE) { editing = null } }

    AircastSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 24.dp).padding(bottom = 24.dp)) {
            Text(if (blocking.isEmpty()) "Messages" else "Why it will not arm", style = MaterialTheme.typography.headlineSmall)
            Text(severitySummary(lines), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (lines.isEmpty() && blocking.isEmpty()) {
                Text("The vehicle has not said anything yet.", Modifier.padding(vertical = 16.dp))
            } else {
                LazyColumn(Modifier.heightIn(max = 420.dp).padding(top = 12.dp)) {
                    items(blocking) { check ->
                        MessageLine(
                            level = if (check.severity == "error") MessageSeverity.Error else MessageSeverity.Warning,
                            time = "",
                        ) {
                            Text(check.message, style = MaterialTheme.typography.bodyMedium)
                            if (check.description.isNotBlank()) {
                                LinkedText(
                                    html = check.description,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    onParameter = { editing = it },
                                )
                            }
                        }
                    }
                    items(lines) { message ->
                        MessageLine(level = message.level, time = message.time) {
                            Text(message.text, style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }
            }
            Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.End) {
                TextButton(onClick = {
                    offMainDetached { VehicleCommands.clearMessages() }
                    onDismiss()
                }) { Text("Clear") }
                TextButton(onClick = onDismiss) { Text("Close") }
            }
        }
    }
}

@Composable
internal fun VehicleMessagesPage(modifier: Modifier = Modifier) {
    val messagesJson by qgcPath(MESSAGES)
    val lines = remember(messagesJson) { vehicleMessages(messagesJson).asReversed() }
    LaunchedEffect(Unit) { offMainDetached { VehicleCommands.resetAllMessages() } }

    if (lines.isEmpty()) {
        EmptyState(R.drawable.ic_description, "No messages", "The vehicle has not said anything yet.", modifier)
        return
    }
    LazyColumn(modifier.fillMaxSize().padding(horizontal = 16.dp)) {
        items(lines) { message ->
            MessageLine(level = message.level, time = message.time) {
                Text(message.text, style = MaterialTheme.typography.bodyMedium)
            }
        }
        item {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                TextButton(onClick = { offMainDetached { VehicleCommands.clearMessages() } }) { Text("Clear") }
            }
        }
    }
}

@Composable
internal fun MessageLine(level: MessageSeverity, time: String, content: @Composable () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(vertical = 10.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Icon(
            painterResource(
                when (level) {
                    MessageSeverity.Error -> R.drawable.ic_error
                    MessageSeverity.Warning -> R.drawable.ic_warning
                    MessageSeverity.Normal -> R.drawable.ic_check_circle
                },
            ),
            null,
            tint = when (level) {
                MessageSeverity.Error -> MaterialTheme.colorScheme.error
                MessageSeverity.Warning -> MaterialTheme.aircast.warning
                MessageSeverity.Normal -> MaterialTheme.aircast.success
            },
            modifier = Modifier.size(24.dp),
        )
        Column(Modifier.weight(1f)) {
            if (time.isNotBlank()) Text(time, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            content()
        }
    }
}

@Composable
internal fun AppSnackbar(data: SnackbarData) {
    if (data.visuals.duration != SnackbarDuration.Indefinite) return Snackbar(data)
    Snackbar(
        modifier = Modifier.padding(12.dp),
        dismissAction = {
            IconButton(onClick = data::dismiss) { Icon(painterResource(R.drawable.ic_close), contentDescription = "Dismiss") }
        },
        shape = RoundedCornerShape(ALERT_CORNER),
        containerColor = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        dismissActionContentColor = MaterialTheme.colorScheme.onErrorContainer,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Icon(painterResource(R.drawable.ic_error), null, Modifier.size(24.dp))
            Text(data.visuals.message, style = MaterialTheme.typography.labelLarge)
        }
    }
}

private val ALERT_CORNER = 12.dp
