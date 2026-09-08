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
import org.json.JSONArray
import java.io.File

private const val PLAN_ROOT = "plan"
private const val OPEN_CACHE = "opened.plan"
private const val SAVE_CACHE = "saving.plan"
private const val KML_CACHE = "export.kml"
private const val MISSION_ROOT = "plan.missionController"

internal const val PLAN_MIME = "*/*"

internal val PLAN_OPEN_TYPES = arrayOf(PLAN_MIME)

class PatternChoice(
    val options: () -> List<String>,
    val pick: (String) -> Unit,
    val cancel: () -> Unit,
)

class PlanFileActions(
    val open: () -> Unit,
    val saveAs: () -> Unit,
    val save: () -> Unit,
    val exportKml: () -> Unit,
    val importBoundary: () -> Unit,
    val patternChoice: PatternChoice,
    val newPlan: () -> Unit,
    val clearMission: () -> Unit,
    val documentName: () -> String?,
)

private fun planReadyForSave(): Int? =
    (Qgc.invokeResult("$PLAN_ROOT.readyForSaveState") as? Number)?.toInt()

internal fun planLoad(path: String): Boolean? =
    Qgc.invokeResult("$PLAN_ROOT.loadFromFile", path) as? Boolean

internal fun loadFailureMessage(loaded: Boolean?): String? = when (loaded) {
    true -> null
    false -> "That is not a plan file. The current plan is unchanged."
    null -> "The plan could not be loaded. The current plan is unchanged."
}

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

private fun patternNames(): List<String> {
    val value = Qgc.get("$MISSION_ROOT.complexMissionItemNames").opt("value")
    val array = value as? JSONArray ?: return emptyList()
    return (0 until array.length()).map { array.optString(it) }.filter { it.isNotBlank() }
}

private fun visualItems(): JSONArray =
    Qgc.get("$MISSION_ROOT.visualItems").opt("elements") as? JSONArray ?: JSONArray()

private fun lastDistance(items: JSONArray): Double? {
    val last = items.optJSONObject(items.length() - 1) ?: return null
    return (last.opt("complexDistance") as? Number)?.toDouble()
}

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

    fun forget() {
        document.value = null
        name.value = null
    }

    fun discard(method: String, success: String, failure: String) {
        scope.launch {
            val ok = withContext(Dispatchers.Default) { Qgc.invoke("$PLAN_ROOT.$method") }
            if (ok) {
                forget()
                onResult(success)
            } else {
                onResult(failure)
            }
        }
    }

    fun importFrom(uri: Uri, pattern: String) {
        scope.launch {
            val message = withContext(Dispatchers.Default) {
                val label = displayName(context, uri)
                val staged = File(context.cacheDir, boundaryCacheName(label))
                staged.delete()
                if (!copyIn(context, uri, staged)) {
                    return@withContext "That file could not be read."
                }
                val before = visualItems().length()
                Qgc.invoke(
                    "$MISSION_ROOT.insertComplexMissionItemFromKMLOrSHP",
                    pattern, staged.absolutePath, before, true,
                )
                val after = visualItems()
                if (after.length() <= before) {
                    return@withContext "${label ?: "That file"} added nothing to the plan."
                }
                if (importedNothing(lastDistance(after))) {
                    Qgc.invoke("$MISSION_ROOT.removeVisualItem", after.length() - 1)
                    return@withContext "${label ?: "That file"} holds no area for a $pattern."
                }
                null
            }
            onResult(message ?: "Boundary imported.")
        }
    }

    fun guarded(action: () -> Unit) {
        scope.launch {
            val blocked = withContext(Dispatchers.Default) { saveBlockedReason(planReadyForSave()) }
            if (blocked == null) action() else onResult(blocked)
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
            val failure = withContext(Dispatchers.Default) {
                val staged = File(context.cacheDir, OPEN_CACHE)
                staged.delete()
                if (!copyIn(context, chosen, staged)) {
                    return@withContext "That file could not be read."
                }
                loadFailureMessage(planLoad(staged.absolutePath))
            }
            if (failure != null) {
                onResult(failure)
                return@launch
            }
            adopt(chosen)
            val warning = withContext(Dispatchers.Default) {
                undrawnItemsWarning(undrawnItemNames(visualItems()))
            }
            onResult(warning ?: "Plan opened.")
        }
    }

    fun exportKmlTo(target: Uri) {
        scope.launch {
            val message = withContext(Dispatchers.Default) {
                val staged = File(context.cacheDir, KML_CACHE)
                staged.delete()
                Qgc.invoke("$PLAN_ROOT.saveToKml", staged.absolutePath)
                if (staged.length() == 0L) {
                    return@withContext "The plan could not be exported."
                }
                if (!copyOut(context, staged, target)) {
                    return@withContext "The KML was written but could not be copied out."
                }
                null
            }
            onResult(message ?: "KML exported.")
        }
    }

    val kmlCreator = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(PLAN_MIME),
    ) { uri -> uri?.let { exportKmlTo(it) } }

    val pendingImport = remember { mutableStateOf<Uri?>(null) }
    val patterns = remember { mutableStateOf<List<String>>(emptyList()) }

    val importer = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val names = withContext(Dispatchers.Default) { patternNames() }
            when {
                names.isEmpty() -> onResult("This vehicle offers no pattern to import into.")
                names.size == 1 -> importFrom(chosen, names.first())
                else -> {
                    patterns.value = names
                    pendingImport.value = chosen
                }
            }
        }
    }

    val creator = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(PLAN_MIME),
    ) { uri -> uri?.let { writeTo(it) } }

    val choosePattern = PatternChoice(
        options = { patterns.value.takeIf { pendingImport.value != null } ?: emptyList() },
        pick = { name ->
            val uri = pendingImport.value
            pendingImport.value = null
            if (uri != null) importFrom(uri, name)
        },
        cancel = { pendingImport.value = null },
    )

    return remember(opener, creator, kmlCreator, importer) {
        PlanFileActions(
            open = { opener.launch(PLAN_OPEN_TYPES) },
            saveAs = { guarded { creator.launch(name.value ?: DEFAULT_PLAN_NAME) } },
            save = {
                guarded {
                    val target = document.value
                    if (target == null) creator.launch(DEFAULT_PLAN_NAME) else writeTo(target)
                }
            },
            exportKml = { guarded { kmlCreator.launch(DEFAULT_KML_NAME) } },
            importBoundary = { importer.launch(PLAN_OPEN_TYPES) },
            patternChoice = choosePattern,
            newPlan = { discard("removeAll", "New plan.", "The plan could not be cleared.") },
            clearMission = {
                discard(
                    "removeAllFromVehicle",
                    "Clear sent to the vehicle.",
                    "The clear could not be sent to the vehicle.",
                )
            },
            documentName = { name.value },
        )
    }
}
