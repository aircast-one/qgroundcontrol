package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc

private const val OSM_FILE_SETTING = "settings.viewer3DSettings.osmFilePath"
private const val OSM_FOLDER = "osm"
private const val OSM_ENABLED_SETTING = "settings.viewer3DSettings.enabled"

internal fun osmFileAccepted(name: String): Boolean = name.lowercase().let { it.endsWith(".osm") || it.endsWith(".xml") }

private sealed interface Staged {
    data class Copied(val path: String) : Staged
    data object NotOsm : Staged
    data object Unreadable : Staged
}

private fun stagedOsm(context: Context, uri: Uri): Staged = runCatching {
    val shown = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }?.replace(Regex("[/\\\\]"), "_").orEmpty()
    if (!osmFileAccepted(shown)) return@runCatching Staged.NotOsm
    val folder = File(context.filesDir, OSM_FOLDER).apply { mkdirs() }
    val incoming = File(folder, ".incoming")
    val copied = context.contentResolver.openInputStream(uri)?.use { source -> incoming.outputStream().use { source.copyTo(it) } } != null
    val staged = File(folder, shown)
    if (copied && incoming.renameTo(staged)) Staged.Copied(staged.absolutePath) else Staged.Unreadable
}.getOrDefault(Staged.Unreadable)

private fun keepOnly(context: Context, path: String) {
    File(context.filesDir, OSM_FOLDER).listFiles()?.filter { it.absolutePath != path }?.forEach { it.delete() }
}

@Composable
internal fun OsmFilePicker(onWrite: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }
    val enabled by one.aircast.android.bridge.qgcBool(one.aircast.android.bridge.settingControl(OSM_ENABLED_SETTING))
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            refusal = withContext(Dispatchers.IO) {
                when (val staged = stagedOsm(context, chosen)) {
                    is Staged.Copied -> Qgc.writeRefusal(OSM_FILE_SETTING, staged.path).also { if (it == null) keepOnly(context, staged.path) }
                    Staged.NotOsm -> "Choose an OpenStreetMap file (.osm or .xml)."
                    Staged.Unreadable -> "That file could not be read."
                }
            }
            onWrite()
        }
    }
    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
        OutlinedButton(enabled = enabled, onClick = { picker.launch(arrayOf("*/*")) }) { Text("Choose OpenStreetMap file") }
        refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
    }
}
