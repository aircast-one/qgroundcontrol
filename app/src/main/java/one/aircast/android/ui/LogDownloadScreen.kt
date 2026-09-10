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
private const val LOGS_VIEW = "view.logs"

internal data class LogEntry(
    val index: Int,
    val id: Int,
    val time: String,
    val sizeStr: String,
    val received: Boolean,
    val selected: Boolean,
    val status: String,
)

internal data class LogsView(
    val connected: Boolean,
    val entries: List<LogEntry>,
    val emptyText: String,
    val eraseWarning: String,
    val canRefresh: Boolean,
    val canDownload: Boolean,
    val canCancel: Boolean,
    val canErase: Boolean,
    val busy: Boolean,
    val anyDownloaded: Boolean,
)

internal fun logsView(view: JSONObject?): LogsView? {
    if (view == null) return null
    val items = view.optJSONArray("entries")
    return LogsView(
        connected = view.optBoolean("connected"),
        entries = (0 until (items?.length() ?: 0)).mapNotNull { index ->
            items!!.optJSONObject(index)?.let { entry ->
                LogEntry(
                    index = entry.optInt("index", index),
                    id = entry.optInt("id"),
                    time = entry.optString("timeText"),
                    sizeStr = entry.optString("sizeText"),
                    received = entry.optBoolean("received"),
                    selected = entry.optBoolean("selected"),
                    status = entry.optString("status"),
                )
            }
        },
        emptyText = view.optString("emptyText"),
        eraseWarning = view.optString("eraseWarning"),
        canRefresh = view.optBoolean("canRefresh"),
        canDownload = view.optBoolean("canDownload"),
        canCancel = view.optBoolean("canCancel"),
        canErase = view.optBoolean("canErase"),
        busy = view.optBoolean("busy"),
        anyDownloaded = view.optBoolean("anyDownloaded"),
    )
}

@Composable
private fun EraseConfirmDialog(onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Erase all logs?") },
        text = {
            Text("This permanently deletes every log on the vehicle. It cannot be undone.")
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(); onDismiss() },
                colors = ButtonDefaults.textButtonColors(
                    contentColor = MaterialTheme.colorScheme.error,
                ),
            ) { Text("Erase all") }
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
        supportingContent = { Text(entry.time) },
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
    val json by qgcPath(LOGS_VIEW)
    val logs = remember(json) { logsView(json) }
    val savePath by qgcString("settings.appSettings.logSavePath")
    var confirmErase by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val entries = logs?.entries.orEmpty()
    val selectedCount = entries.count { it.selected }
    val selectable = entries.filter { it.received }
    val busy = logs?.busy == true

    if (logs?.connected != true) {
        Message(logs?.emptyText?.ifBlank { null } ?: "Connect a vehicle to download its flight logs.", modifier)
        return
    }

    if (confirmErase) {
        EraseConfirmDialog(
            onConfirm = { scope.offMain { Qgc.invoke("$LOG_ROOT.eraseAll") } },
            onDismiss = { confirmErase = false },
        )
    }

    LaunchedEffect(logs.connected) {
        if (shouldAutoRefreshLogs(logs.connected, entries.isNotEmpty(), busy)) {
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
                enabled = logs.canRefresh,
            ) { Text("Refresh") }

            Button(
                onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.download") } },
                enabled = logs.canDownload && selectedCount > 0,
            ) { Text(if (selectedCount > 0) "Download ($selectedCount)" else "Download") }

            if (logs.canCancel) {
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
            entries.isEmpty() -> Message(
                logs.emptyText.ifBlank { "This vehicle reports no flight logs." },
            )
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

        if (logs.anyDownloaded && savePath.isNotBlank()) {
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
