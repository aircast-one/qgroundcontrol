package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMain
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcString

private const val LOG_ROOT = "logDownload"
private const val LOG_MODEL = "logDownload.model"

internal data class LogEntry(
    val index: Int,
    val id: Int,
    val time: String,
    val sizeStr: String,
    val received: Boolean,
    val selected: Boolean,
    val status: String,
)

internal fun parseLogEntries(model: JSONObject?): List<LogEntry> {
    val elements = model?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        elements.optJSONObject(index)?.let { entry ->
            LogEntry(
                index = index,
                id = entry.optInt("id"),
                time = entry.optString("time"),
                sizeStr = entry.optString("sizeStr"),
                received = entry.optBoolean("received"),
                selected = entry.optBoolean("selected"),
                status = entry.optString("status"),
            )
        }
    }
}

internal fun formatLogTime(raw: String): String =
    raw.replace('T', ' ').substringBefore('.').ifBlank { "Unknown date" }

@Composable
private fun EraseConfirmDialog(onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Erase all logs?") },
        text = {
            Text("This permanently deletes every log on the vehicle. It cannot be undone.")
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(); onDismiss() }) { Text("Erase all") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun LogRow(entry: LogEntry, enabled: Boolean, onToggle: (Boolean) -> Unit) {
    val selectable = enabled && entry.received
    ListItem(
        modifier = Modifier.clickable(enabled = selectable) { onToggle(!entry.selected) },
        leadingContent = {
            Checkbox(
                checked = entry.selected,
                onCheckedChange = onToggle,
                enabled = selectable,
            )
        },
        headlineContent = { Text("Log ${entry.id}") },
        supportingContent = { Text(formatLogTime(entry.time)) },
        trailingContent = {
            Column(horizontalAlignment = Alignment.End) {
                Text(entry.sizeStr, style = MaterialTheme.typography.labelLarge)
                Text(entry.status, style = MaterialTheme.typography.labelMedium)
            }
        },
    )
}

@Composable
private fun Message(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

internal fun shouldAutoRefreshLogs(hasVehicle: Boolean, hasEntries: Boolean, busy: Boolean) =
    hasVehicle && !hasEntries && !busy

@Composable
fun LogDownloadScreen(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val root by qgcPath(LOG_ROOT)
    val model by qgcPath(LOG_MODEL)
    val savePath by qgcString("settings.appSettings.logSavePath")
    var confirmErase by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val listing = root?.optBoolean("requestingList") == true
    val downloading = root?.optBoolean("downloadingLogs") == true
    val entries = parseLogEntries(model)
    val selectedCount = entries.count { it.selected }
    val selectable = entries.filter { it.received }
    val busy = listing || downloading
    val anyDownloaded = entries.any { it.status == "Downloaded" }

    if (!hasVehicle) {
        Message("Connect a vehicle to download its flight logs.", modifier)
        return
    }

    if (confirmErase) {
        EraseConfirmDialog(
            onConfirm = { scope.offMain { Qgc.invoke("$LOG_ROOT.eraseAll") } },
            onDismiss = { confirmErase = false },
        )
    }

    LaunchedEffect(hasVehicle) {
        if (shouldAutoRefreshLogs(hasVehicle, entries.isNotEmpty(), busy)) {
            offMain { Qgc.invoke("$LOG_ROOT.refresh") }
        }
    }

    Column(modifier.fillMaxSize()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedButton(
                onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.refresh") } },
                enabled = !busy,
            ) { Text("Refresh") }

            Button(
                onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.download") } },
                enabled = !busy && selectedCount > 0,
            ) { Text(if (selectedCount > 0) "Download ($selectedCount)" else "Download") }

            if (busy) {
                OutlinedButton(onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.cancel") } }) { Text("Cancel") }
            }
        }

        if (selectable.isNotEmpty() && !busy) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable {
                        val selectAll = selectedCount < selectable.size
                        scope.offMain {
                            selectable.forEach { entry ->
                                Qgc.set("$LOG_MODEL.${entry.index}.selected", selectAll)
                            }
                        }
                    }
                    .padding(horizontal = 16.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Checkbox(
                    checked = selectedCount == selectable.size,
                    onCheckedChange = null,
                )
                Text(
                    text = if (selectedCount < selectable.size) "Select all" else "Clear selection",
                    style = MaterialTheme.typography.labelLarge,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }

        if (busy) {
            LinearProgressIndicator(Modifier.fillMaxWidth())
        }

        when {
            listing && entries.isEmpty() -> Message("Asking the vehicle for its log list.")
            entries.isEmpty() -> Message("This vehicle reports no flight logs.")
            else -> LazyColumn(Modifier.weight(1f)) {
                items(entries, key = { it.index }) { entry ->
                    LogRow(entry, enabled = !busy) { checked ->
                        scope.offMain { Qgc.set("$LOG_MODEL.${entry.index}.selected", checked) }
                    }
                    HorizontalDivider()
                }
                item(key = "erase") {
                    if (!busy) {
                        TextButton(
                            onClick = { confirmErase = true },
                            colors = ButtonDefaults.textButtonColors(
                                contentColor = MaterialTheme.colorScheme.error,
                            ),
                            modifier = Modifier.padding(horizontal = 12.dp),
                        ) { Text("Erase all logs from the vehicle") }
                    }
                }
            }
        }

        if (anyDownloaded && savePath.isNotBlank()) {
            Text(
                text = "Saved to $savePath",
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
            )
        }
    }
}
