package one.aircast.android.ui

import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.FilledTonalButton
import androidx.compose.ui.semantics.semantics

import androidx.compose.ui.semantics.contentDescription

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import one.aircast.android.R

import java.time.LocalDateTime
import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
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
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText

private const val LOG_ROOT = "logDownload"
private const val LOG_MODEL = "logDownload.model"
private const val LOGS_VIEW = "view.logs"

internal const val TIME_UNRECEIVED = "unreceived"
internal const val TIME_UNKNOWN = "unknown"

private val LOCAL_TIME: DateTimeFormatter =
    DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM)

internal fun logLocalTime(raw: String): LocalDateTime? =
    runCatching { OffsetDateTime.parse(raw).atZoneSameInstant(ZoneId.systemDefault()).toLocalDateTime() }
        .recoverCatching { LocalDateTime.parse(raw) }
        .getOrNull()

internal fun logTimeText(raw: String, state: String, format: (LocalDateTime) -> String): String = when (state) {
    TIME_UNRECEIVED -> ""
    TIME_UNKNOWN -> "Date Unknown"
    else -> logLocalTime(raw)?.let(format) ?: "Date Unknown"
}

internal data class LogEntry(
    val index: Int,
    val id: Int,
    val time: String,
    val sizeStr: String,
    val received: Boolean,
    val selected: Boolean,
    val status: String,
    val downloading: Boolean = false,
    val saved: Boolean = false,
)

internal fun logSections(entries: List<LogEntry>): List<Pair<String, List<LogEntry>>> =
    listOf("On the vehicle" to entries.filterNot { it.saved }, "On this phone" to entries.filter { it.saved })
        .filter { it.second.isNotEmpty() }

internal fun downloadCard(entries: List<LogEntry>): Pair<String, String> =
    entries.firstOrNull { it.downloading }
        ?.let { "Downloading log ${it.id}" to it.status }
        ?: ("Downloading" to "")

internal data class LogsView(
    val connected: Boolean,
    val entries: List<LogEntry>,
    val emptyText: String,
    val eraseWarning: String,
    val canRefresh: Boolean,
    val canDownload: Boolean,
    val canCancel: Boolean,
    val canErase: Boolean,
    val eraseSelectedShown: Boolean = false,
    val canEraseSelected: Boolean = false,
    val canSort: Boolean = false,
    val sortText: String = "",
    val busy: Boolean,
    val downloading: Boolean = false,
    val anyDownloaded: Boolean,
    val savePath: String,
    val savePathReason: String,
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
                    time = logTimeText(
                        entry.optText("time"),
                        entry.optText("timeState"),
                    ) { LOCAL_TIME.format(it) },
                    sizeStr = entry.optText("sizeText"),
                    received = entry.optBoolean("received"),
                    selected = entry.optBoolean("selected"),
                    status = entry.optText("status"),
                    downloading = entry.optText("statusId") == "downloading",
                    saved = entry.optText("statusId") == "downloaded",
                )
            }
        },
        emptyText = view.optText("emptyText"),
        eraseWarning = view.optText("eraseWarning"),
        canRefresh = view.optBoolean("canRefresh"),
        canDownload = view.optBoolean("canDownload"),
        canCancel = view.optBoolean("canCancel"),
        canErase = view.optBoolean("canErase"),
        eraseSelectedShown = view.optBoolean("eraseSelectedShown"),
        canEraseSelected = view.optBoolean("canEraseSelected"),
        canSort = view.optBoolean("canSort"),
        sortText = view.optText("sortText"),
        busy = view.optBoolean("busy"),
        downloading = view.optBoolean("downloading"),
        anyDownloaded = view.optBoolean("anyDownloaded"),
        savePath = view.optText("savePath"),
        savePathReason = view.optText("savePathReason"),
    )
}

internal enum class EraseKind(val action: String, val title: String, val text: String, val confirm: String) {
    All("eraseAll", "Delete All Onboard Log Files", "All onboard log files will be erased permanently. Is this really what you want?", "Erase All"),
    Selected("eraseSelected", "Delete Selected Onboard Log Files", "The selected onboard log files will be erased permanently. Is this really what you want?", "Erase Selected"),
}

@Composable
private fun EraseConfirmDialog(erase: EraseKind, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(erase.title) },
        text = { Text(erase.text) },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(); onDismiss() },
                colors = ButtonDefaults.textButtonColors(
                    contentColor = MaterialTheme.colorScheme.error,
                ),
            ) { Text(erase.confirm) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun LogRow(entry: LogEntry, enabled: Boolean, onToggle: (Boolean) -> Unit) {
    val selectable = enabled && entry.received
    Row(
        Modifier
            .fillMaxWidth()
            .toggleable(value = entry.selected, enabled = selectable, role = Role.Checkbox, onValueChange = onToggle)
            .background(if (entry.selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent)
            .heightIn(min = 72.dp)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.size(40.dp).background(
                if (entry.selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.secondaryContainer,
                CircleShape,
            ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                painterResource(if (entry.selected) R.drawable.ic_check_circle else R.drawable.ic_description),
                null,
                tint = if (entry.selected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSecondaryContainer,
                modifier = Modifier.size(24.dp),
            )
        }
        Column(Modifier.weight(1f)) {
            Text("Log ${entry.id}", style = MaterialTheme.typography.bodyLarge)
            Text(
                listOf(entry.time, entry.sizeStr).filter { it.isNotBlank() }.joinToString(" · "),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(entry.status, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
    }
}

internal fun savedToText(logs: LogsView?): String? {
    val reading = logs?.takeIf { it.anyDownloaded } ?: return null
    return when {
        reading.savePath.isNotBlank() -> "Saved to ${reading.savePath}"
        reading.savePathReason.isNotBlank() -> reading.savePathReason
        else -> null
    }
}

internal fun shouldAutoRefreshLogs(hasVehicle: Boolean, hasEntries: Boolean, busy: Boolean) =
    hasVehicle && !hasEntries && !busy

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun LogDownloadScreen(modifier: Modifier = Modifier) {
    val json by qgcPath(LOGS_VIEW)
    val logs = remember(json) { logsView(json) }
    var confirmErase by remember { mutableStateOf<EraseKind?>(null) }
    val scope = rememberCoroutineScope()

    val entries = logs?.entries.orEmpty()
    val selectedCount = entries.count { it.selected }
    val selectable = entries.filter { it.received }
    val busy = logs?.busy == true
    BlocksNavigation(logs?.downloading == true, LOG_DOWNLOAD_BLOCK)

    if (logs?.connected != true) {
        EmptyState(R.drawable.ic_description, "No vehicle", logs?.emptyText?.ifBlank { null } ?: "Connect a vehicle to download its flight logs.", modifier)
        return
    }

    confirmErase?.let { erase ->
        EraseConfirmDialog(
            erase = erase,
            onConfirm = { scope.offMain { Qgc.invoke("$LOG_ROOT.${erase.action}") } },
            onDismiss = { confirmErase = null },
        )
    }

    LaunchedEffect(logs.connected) {
        if (shouldAutoRefreshLogs(logs.connected, entries.isNotEmpty(), busy)) {
            offMain { Qgc.invoke("$LOG_ROOT.refresh") }
        }
    }

    Box(modifier.fillMaxSize()) {
    Column(Modifier.fillMaxSize()) {
        FlowRow(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            FilledTonalButton(
                onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.refresh") } },
                enabled = logs.canRefresh,
            ) { Text("Refresh") }

            if (logs.sortText.isNotBlank()) {
                FilledTonalButton(
                    onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.toggleSortByDate") } },
                    enabled = logs.canSort,
                ) { Text(sentenceCase(logs.sortText)) }
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
                    text = if (selectedCount < selectable.size) "Select all" else "Deselect all",
                    style = MaterialTheme.typography.labelLarge,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }

        if (busy) {
            Surface(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
                shape = MaterialTheme.shapes.large,
                color = MaterialTheme.colorScheme.surfaceContainerHigh,
            ) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    val (title, progress) = downloadCard(entries)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(title, style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
                        if (progress.isNotBlank()) {
                            Text(progress, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        if (logs.canCancel) {
                            TextButton(onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.cancel") } }) { Text("Cancel") }
                        }
                    }
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }
            }
        }

        when {
            entries.isEmpty() -> EmptyState(
                R.drawable.ic_description,
                "No logs yet",
                logs.emptyText.ifBlank { "This vehicle reports no flight logs." },
            )
            else -> LazyColumn(Modifier.weight(1f)) {
                logSections(entries).forEach { (title, section) ->
                    item(key = title) { SectionHeader(title) }
                    items(section, key = { it.index }) { entry ->
                        LogRow(entry, enabled = !busy) { checked ->
                            scope.offMain { Qgc.set("$LOG_MODEL.${entry.index}.selected", checked) }
                        }
                    }
                }
                item(key = "erase") {
                    if (!busy) {
                        Row(Modifier.padding(horizontal = 16.dp, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            if (logs.eraseSelectedShown) {
                                OutlinedButton(
                                    enabled = logs.canEraseSelected,
                                    onClick = { confirmErase = EraseKind.Selected },
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                ) { Text("Erase Selected") }
                            }
                            OutlinedButton(
                                enabled = logs.canErase,
                                onClick = { confirmErase = EraseKind.All },
                                colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                            ) { Text("Erase all") }
                        }
                    }
                    Spacer(Modifier.height(88.dp))
                }
            }
        }

        savedToText(logs)?.let { line ->
            Text(
                text = line,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
            )
        }
    }
    if (logs.canDownload && selectedCount > 0) {
        ExtendedFloatingActionButton(
            onClick = { scope.offMain { Qgc.invoke("$LOG_ROOT.download") } },
            icon = { Icon(painterResource(R.drawable.ic_download), null) },
            text = { Text("Download ($selectedCount)") },
            containerColor = MaterialTheme.colorScheme.primaryContainer,
            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp).semantics { contentDescription = "Download ($selectedCount)" },
        )
    }
    }
}
