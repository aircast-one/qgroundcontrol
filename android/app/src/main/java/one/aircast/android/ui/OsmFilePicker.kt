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

internal fun osmFileAccepted(name: String): Boolean = name.lowercase().let { it.endsWith(".osm") || it.endsWith(".xml") }

private fun stagedOsm(context: Context, uri: Uri): String? = runCatching {
    val shown = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }?.replace(Regex("[/\\\\]"), "_").orEmpty()
    if (!osmFileAccepted(shown)) return@runCatching null
    val folder = File(context.filesDir, OSM_FOLDER).apply { deleteRecursively(); mkdirs() }
    val staged = File(folder, shown)
    context.contentResolver.openInputStream(uri)?.use { source -> staged.outputStream().use { source.copyTo(it) } }?.let { staged.absolutePath }
}.getOrNull()

@Composable
internal fun OsmFilePicker(onWrite: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            refusal = withContext(Dispatchers.IO) {
                stagedOsm(context, chosen)?.let { path -> Qgc.writeRefusal(OSM_FILE_SETTING, path) } ?: "Choose an OpenStreetMap file (.osm or .xml)."
            }
            onWrite()
        }
    }
    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
        OutlinedButton(onClick = { picker.launch(arrayOf("*/*")) }) { Text("Choose OpenStreetMap file") }
        refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
    }
}
