package one.aircast.android.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject
import java.net.URLEncoder

internal const val APP_LOG_VIEW = "view.appLog"
internal const val APP_LOG_CLEAR = "appLog.clear"
internal const val APP_LOG_SAVE = "appLog.save"
internal const val APP_LOG_SAVE_FORMAT = "settings.logManagerSettings.saveFormat.rawValue"
private const val SAVE_FORMAT_CSV = 1
private const val APP_LOG_POLL_MS = 500L
private const val FILTER_DEBOUNCE_MS = 200L
private const val LEVEL_WARNING = 2
private const val LEVEL_CRITICAL = 3

internal data class AppLogFilter(val levelIndex: Int = 0, val category: String = "", val text: String = "", val regex: Boolean = false)

internal data class AppLogEntry(val sequence: Long, val level: Int, val message: String, val category: String, val timestamp: String, val source: String)

internal data class AppLogRead(
    val levels: List<String>,
    val categories: List<String>,
    val regexValid: Boolean,
    val first: Long?,
    val entries: List<AppLogEntry>,
)

private fun encoded(text: String): String = URLEncoder.encode(text, "UTF-8")

internal fun appLogPath(filter: AppLogFilter, after: Long?): String =
    "$APP_LOG_VIEW(${(filter.levelIndex - 1).coerceAtLeast(0)},${encoded(filter.category)},${encoded(filter.text)},${if (filter.regex) 1 else 0},${after ?: ""})"

private fun strings(view: JSONObject, key: String): List<String> =
    view.optJSONArray(key)?.let { array -> (0 until array.length()).map { array.optString(it) } } ?: emptyList()

internal fun appLogRead(view: JSONObject?): AppLogRead? =
    view?.takeIf { it.optString("class") == "AppLog" }?.let {
        val entries = it.optJSONArray("entries")
        AppLogRead(
            levels = strings(it, "levels"),
            categories = strings(it, "categories"),
            regexValid = it.optBoolean("regexValid", true),
            first = if (it.isNull("first")) null else it.optLong("first"),
            entries = (0 until (entries?.length() ?: 0)).mapNotNull { index ->
                entries?.optJSONObject(index)?.let { entry ->
                    AppLogEntry(
                        sequence = entry.optLong("sequence"),
                        level = entry.optInt("level"),
                        message = entry.optText("message"),
                        category = entry.optText("category"),
                        timestamp = entry.optText("timestamp"),
                        source = entry.optText("source"),
                    )
                }
            },
        )
    }

internal fun mergedEntries(held: List<AppLogEntry>, read: AppLogRead): List<AppLogEntry> =
    held.filter { entry -> read.first != null && entry.sequence >= read.first } + read.entries

internal fun appLogFileName(saveFormat: JSONObject?): String =
    if (saveFormat?.optInt("value") == SAVE_FORMAT_CSV) "QGCConsole.csv" else "QGCConsole.txt"

internal fun appLogMime(fileName: String): String = if (fileName.endsWith(".csv", ignoreCase = true)) "text/csv" else "text/plain"

private class CreateAppLog : ActivityResultContracts.CreateDocument("text/plain") {
    override fun createIntent(context: Context, input: String): Intent =
        super.createIntent(context, input).setType(appLogMime(input))
}

private fun displayName(context: Context, uri: Uri, fallback: String): String =
    runCatching {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }
    }.getOrNull() ?: fallback

@Composable
private fun levelColor(level: Int): Color = when {
    level == 0 -> MaterialTheme.colorScheme.onSurfaceVariant
    level == LEVEL_WARNING -> MaterialTheme.aircast.warning
    level >= LEVEL_CRITICAL -> MaterialTheme.aircast.alert
    else -> MaterialTheme.colorScheme.onSurface
}

@Composable
fun AppLogPage(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var filter by remember { mutableStateOf(AppLogFilter()) }
    var entries by remember { mutableStateOf<List<AppLogEntry>>(emptyList()) }
    var read by remember { mutableStateOf<AppLogRead?>(null) }
    var cleared by remember { mutableStateOf(0) }
    var notice by remember { mutableStateOf<String?>(null) }
    var following by remember { mutableStateOf(true) }
    var showCategories by remember { mutableStateOf(false) }
    val list = rememberLazyListState()

    LaunchedEffect(filter, cleared) {
        delay(FILTER_DEBOUNCE_MS)
        entries = emptyList()
        while (isActive) {
            val after = entries.lastOrNull()?.sequence
            val next = withContext(Dispatchers.IO) { appLogRead(Qgc.get(appLogPath(filter, after))) }
            next?.let {
                read = it
                entries = mergedEntries(entries, it)
            }
            delay(APP_LOG_POLL_MS)
        }
    }

    val atBottom = !list.canScrollForward
    LaunchedEffect(atBottom) { if (!atBottom) following = false }
    LaunchedEffect(entries.size, following) {
        if (following && entries.isNotEmpty()) list.scrollToItem(entries.lastIndex)
    }

    var saveName by remember { mutableStateOf(appLogFileName(null)) }
    val saver = rememberLauncherForActivityResult(CreateAppLog()) { uri ->
        val target = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            notice = withContext(Dispatchers.IO) {
                val saved = Qgc.invokeResult(APP_LOG_SAVE, displayName(context, target, saveName)) as? String
                    ?: return@withContext "The log could not be read."
                runCatching { context.contentResolver.openOutputStream(target, "wt")?.use { it.write(saved.toByteArray()) } }
                    .fold({ null }, { "The file could not be written." })
            }
        }
    }

    Column(modifier.fillMaxSize()) {
        Box(Modifier.weight(1f).fillMaxWidth()) {
            if (entries.isEmpty()) {
                Text(
                    "No log entries",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.align(Alignment.Center),
                )
            }
            LazyColumn(state = list, modifier = Modifier.fillMaxSize()) {
                itemsIndexed(entries, key = { _, entry -> entry.sequence }) { index, entry ->
                    AppLogRow(entry, index)
                }
            }
            if (!atBottom && entries.isNotEmpty()) {
                FilledTonalButton(
                    onClick = { following = true },
                    modifier = Modifier.align(Alignment.TopCenter).padding(top = 8.dp),
                ) { Text("Show latest") }
            }
        }
        HorizontalDivider()
        AppLogFilterBar(
            filter = filter,
            read = read,
            onFilter = { filter = it },
            onCategories = { showCategories = true },
            onSave = {
                scope.launch {
                    saveName = withContext(Dispatchers.Default) { appLogFileName(Qgc.get(APP_LOG_SAVE_FORMAT)) }
                    saver.launch(saveName)
                }
            },
            onClear = {
                scope.launch {
                    withContext(Dispatchers.IO) { Qgc.invoke(APP_LOG_CLEAR) }
                    cleared++
                }
            },
        )
        notice?.let {
            Text(it, color = MaterialTheme.aircast.alert, modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp))
        }
    }
    if (showCategories) LoggingCategoriesDialog(onDismiss = { showCategories = false })
}

@Composable
private fun AppLogRow(entry: AppLogEntry, index: Int) {
    val colour = levelColor(entry.level)
    val shade = if (index % 2 == 0) MaterialTheme.colorScheme.surface else MaterialTheme.colorScheme.surfaceContainer
    Row(
        Modifier.fillMaxWidth().background(shade).padding(end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            Modifier.width(3.dp).fillMaxHeight()
                .background(if (entry.level >= LEVEL_WARNING) colour else Color.Transparent),
        )
        Column(Modifier.weight(1f).padding(vertical = 4.dp)) {
            Text(
                entry.message,
                color = colour,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodySmall,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                listOf(entry.timestamp, entry.category, entry.source).filter { it.isNotEmpty() }.joinToString("  "),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.labelSmall,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun AppLogFilterBar(
    filter: AppLogFilter,
    read: AppLogRead?,
    onFilter: (AppLogFilter) -> Unit,
    onCategories: () -> Unit,
    onSave: () -> Unit,
    onClear: () -> Unit,
) {
    Column(
        Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainer).padding(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(
            Modifier.horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val levels = read?.levels.orEmpty()
            Picker(sentenceCase(levels.getOrElse(filter.levelIndex) { "All Levels" }), levels.map(::sentenceCase)) { index ->
                onFilter(filter.copy(levelIndex = index))
            }
            val categories = read?.categories.orEmpty()
            Picker(sentenceCase(filter.category.ifEmpty { "All Categories" }), categories.map(::sentenceCase)) { index ->
                onFilter(filter.copy(category = if (index == 0) "" else categories[index]))
            }
            OutlinedButton(onClick = onCategories) { Text("Categories") }
            OutlinedButton(onClick = onSave) { Text("Save") }
            OutlinedButton(onClick = onClear) { Text("Clear") }
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = filter.text,
                onValueChange = { onFilter(filter.copy(text = it)) },
                placeholder = { Text("Search…") },
                singleLine = true,
                isError = read?.regexValid == false,
                modifier = Modifier.weight(1f),
            )
            FilterChip(
                selected = filter.regex,
                onClick = { onFilter(filter.copy(regex = !filter.regex)) },
                label = { Text(".*", fontFamily = FontFamily.Monospace) },
            )
        }
    }
}

@Composable
private fun Picker(label: String, options: List<String>, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        TextButton(onClick = { open = true }, enabled = options.isNotEmpty()) { Text(label) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(text = { Text(option) }, onClick = {
                    open = false
                    onPick(index)
                })
            }
        }
    }
}
