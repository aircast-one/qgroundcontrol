package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import java.io.File

private const val PLAN_ROOT = "plan"
private const val OPEN_CACHE = "opened.plan"
private const val SAVE_CACHE = "saving.plan"

internal const val PLAN_MIME = "*/*"

internal val PLAN_OPEN_TYPES = arrayOf(PLAN_MIME)

internal const val DEFAULT_PLAN_NAME = "mission.plan"

class PlanFileActions(
    val open: () -> Unit,
    val saveAs: () -> Unit,
    val save: () -> Unit,
    val documentName: () -> String?,
)

internal fun planLoad(path: String): Boolean =
    Qgc.invokeResult("$PLAN_ROOT.loadFromFile", path) == true

internal fun planSave(path: String): String? {
    if (Qgc.invokeResult("$PLAN_ROOT.saveToFile", path) != true) {
        return null
    }
    return (Qgc.get("$PLAN_ROOT.currentPlanFile").opt("value") as? String)?.takeIf { it.isNotBlank() }
}

private fun copyIn(context: Context, uri: Uri, into: File): Boolean = runCatching {
    context.contentResolver.openInputStream(uri)?.use { source ->
        into.outputStream().use { source.copyTo(it) }
    } ?: return false
    into.length() > 0
}.getOrDefault(false)

private fun copyOut(context: Context, from: File, uri: Uri): Boolean = runCatching {
    context.contentResolver.openOutputStream(uri, "wt")?.use { sink ->
        from.inputStream().use { it.copyTo(sink) }
    } ?: return false
    true
}.getOrDefault(false)

private fun displayName(context: Context, uri: Uri): String? = runCatching {
    context.contentResolver
        .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
        ?.use { if (it.moveToFirst()) it.getString(0) else null }
}.getOrNull()

@Composable
fun rememberPlanFileActions(onResult: (String) -> Unit = {}): PlanFileActions {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val document = remember { mutableStateOf<Uri?>(null) }
    val name = remember { mutableStateOf<String?>(null) }

    fun adopt(uri: Uri) {
        document.value = uri
        scope.launch {
            name.value = withContext(Dispatchers.IO) { displayName(context, uri) }
        }
    }

    fun writeTo(target: Uri) {
        scope.launch {
            val message = withContext(Dispatchers.Default) {
                val staged = File(context.cacheDir, SAVE_CACHE)
                staged.delete()
                val written = planSave(staged.absolutePath)
                    ?: return@withContext "The plan could not be saved."
                if (!copyOut(context, File(written), target)) {
                    return@withContext "The plan was written but could not be copied out."
                }
                null
            }
            if (message == null) {
                adopt(target)
                onResult("Plan saved.")
            } else {
                onResult(message)
            }
        }
    }

    val opener = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val message = withContext(Dispatchers.Default) {
                val staged = File(context.cacheDir, OPEN_CACHE)
                staged.delete()
                if (!copyIn(context, chosen, staged)) {
                    return@withContext "That file could not be read."
                }
                if (!planLoad(staged.absolutePath)) {
                    return@withContext "That is not a plan file. The current plan is unchanged."
                }
                null
            }
            if (message == null) {
                adopt(chosen)
                onResult("Plan opened.")
            } else {
                onResult(message)
            }
        }
    }

    val creator = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(PLAN_MIME),
    ) { uri -> uri?.let { writeTo(it) } }

    return remember(opener, creator) {
        PlanFileActions(
            open = { opener.launch(PLAN_OPEN_TYPES) },
            saveAs = { creator.launch(name.value ?: DEFAULT_PLAN_NAME) },
            save = {
                val target = document.value
                if (target == null) creator.launch(DEFAULT_PLAN_NAME) else writeTo(target)
            },
            documentName = { name.value },
        )
    }
}
