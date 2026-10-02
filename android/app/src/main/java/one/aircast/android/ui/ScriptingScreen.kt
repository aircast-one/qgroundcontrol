package one.aircast.android.ui

import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val SCRIPTING_VIEW = "view.scripting"
internal const val SCRIPTING_SCREEN = "scripting"
internal const val SCRIPTING_REFRESH = "scripting.refresh"
internal const val SCRIPTING_UPLOAD = "scripting.upload"
internal const val SCRIPTING_DOWNLOAD = "scripting.download"
internal const val SCRIPTING_DELETE = "scripting.delete"
internal const val SCRIPTING_CANCEL = "scripting.cancel"
private const val BUSY_POLL_MS = 300L
private const val IDLE_POLL_MS = 2000L

internal data class Scripting(
    val available: Boolean,
    val enabled: Boolean,
    val enable: Fact?,
    val scripts: List<String>,
    val busy: Boolean,
    val progress: Float,
    val status: String,
    val unsupportedText: String,
)

internal fun scripting(view: JSONObject?): Scripting? = view?.let {
    val scripts = it.optJSONArray("scripts")
    Scripting(
        available = it.optBoolean("available"),
        enabled = it.optBoolean("enabled"),
        enable = it.optJSONObject("enable")?.let(::factFromControl),
        scripts = (0 until (scripts?.length() ?: 0)).map { at -> scripts!!.optString(at) },
        busy = it.optBoolean("busy"),
        progress = it.optDouble("progress", 0.0).toFloat(),
        status = it.optText("status"),
        unsupportedText = it.optText("unsupportedText"),
    )
}

private fun displayName(context: android.content.Context, uri: Uri): String =
    context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    } ?: uri.lastPathSegment.orEmpty().substringAfterLast('/')

@Composable
fun ScriptingScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<Scripting?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var confirmDelete by remember { mutableStateOf<String?>(null) }
    var pendingDownload by remember { mutableStateOf<String?>(null) }
    var refreshedOnOpen by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { scripting(Qgc.get(SCRIPTING_VIEW)) }
        if (!refreshedOnOpen && read?.enabled == true) {
            refreshedOnOpen = true
            withContext(Dispatchers.Default) { Qgc.invoke(SCRIPTING_REFRESH) }
        }
        delay(if (read?.busy == true) BUSY_POLL_MS else IDLE_POLL_MS)
        revision++
    }

    fun act(path: String, vararg args: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            revision++
        }
    }

    val uploader = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            refusal = withContext(Dispatchers.IO) {
                val name = displayName(context, chosen).replace('/', '_')
                val staged = File(context.cacheDir, "upload-$name")
                val copied = runCatching { context.contentResolver.openInputStream(chosen)?.use { source -> staged.outputStream().use { source.copyTo(it) } } != null }.getOrDefault(false)
                if (copied) Qgc.refusalOf(SCRIPTING_UPLOAD, staged.absolutePath, name) else "That file could not be read."
            }
            revision++
        }
    }
    val downloader = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/x-lua")) { uri ->
        val name = pendingDownload ?: return@rememberLauncherForActivityResult
        val target = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = File(context.cacheDir, "download-$name")
            withContext(Dispatchers.IO) { staged.delete() }
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(SCRIPTING_DOWNLOAD, name, staged.absolutePath) }
            while (refusal == null && withContext(Dispatchers.Default) { scripting(Qgc.get(SCRIPTING_VIEW))?.busy } == true) delay(BUSY_POLL_MS)
            withContext(Dispatchers.IO) {
                when (refusal == null && staged.exists()) {
                    true -> context.contentResolver.openOutputStream(target)?.use { out -> staged.inputStream().use { it.copyTo(out) } }
                    false -> runCatching { android.provider.DocumentsContract.deleteDocument(context.contentResolver, target) }
                }
                staged.delete()
            }
            revision++
        }
    }

    val page = read ?: return
    if (!page.available) {
        Text(page.unsupportedText, modifier.padding(16.dp))
        return
    }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        page.enable?.let { FactRow(it, title = "Enable Scripting") { act(SCRIPTING_REFRESH) } }
        Column(Modifier.padding(horizontal = 20.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            if (page.status.isNotBlank()) Text(page.status, style = MaterialTheme.typography.bodyMedium)
            OutlinedButton(enabled = page.enabled && !page.busy, onClick = { uploader.launch(arrayOf("*/*")) }) { Text("Upload") }
            page.scripts.forEach { name ->
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(name, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                    TextButton(enabled = page.enabled && !page.busy, onClick = {
                        pendingDownload = name
                        downloader.launch(name)
                    }) { Text("Download") }
                    TextButton(enabled = page.enabled && !page.busy, onClick = { confirmDelete = name }) { Text("Delete") }
                }
            }
            if (page.busy) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    TextButton(onClick = { act(SCRIPTING_CANCEL) }) { Text("Cancel Operation") }
                    Text("Transferring... ${Math.round(page.progress * 100)}%")
                }
                LinearProgressIndicator(progress = { page.progress }, modifier = Modifier.fillMaxWidth())
            }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }
    confirmDelete?.let { name ->
        AlertDialog(
            onDismissRequest = { confirmDelete = null },
            title = { Text("Delete Lua Script") },
            text = { Text("Are you sure you want to delete the script \"$name\"? This action cannot be undone.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = null
                    act(SCRIPTING_DELETE, name)
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = null }) { Text("Cancel") } },
        )
    }
}
