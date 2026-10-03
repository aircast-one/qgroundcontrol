package one.aircast.android.ui

import one.aircast.mapspike.PlanFocus
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import one.aircast.mapspike.freshPlanView
import one.aircast.mapspike.optText

private const val PLAN_ROOT = "plan"
private const val OPEN_CACHE = "opened.plan"
private const val SAVE_CACHE = "saving.plan"
private const val KML_CACHE = "export.kml"
private const val MISSION_ROOT = "plan.missionController"
private const val PLAN_VIEW = "view.plan"
private const val MISSION_ITEMS_VIEW = "view.missionItems"

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
    val download: () -> Unit,
    val documentName: () -> String?,
    val opened: () -> Int = { 0 },
)

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
    return currentPlanPath(Qgc.get(PLAN_VIEW))
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

// view.plan serves the patterns by their canonical names, the insert keys. The raw
// complexMissionItemNames was renamed upstream to complexMissionItems and answered only through the
// core's rename shim.
internal fun patternNames(view: JSONObject?): List<String> {
    val patterns = view?.optJSONArray("patterns") ?: return emptyList()
    return (0 until patterns.length()).mapNotNull { patterns.optJSONObject(it)?.optText("name")?.ifBlank { null } }
}

private fun patternNames(): List<String> = patternNames(Qgc.get(PLAN_VIEW))

internal fun currentPlanPath(view: JSONObject?): String? = view?.optText("filePath")?.ifBlank { null }

internal fun visualItems(): JSONArray =
    Qgc.get(MISSION_ITEMS_VIEW).optJSONArray("items") ?: JSONArray()

private fun lastDistance(items: JSONArray): Double? {
    val last = items.optJSONObject(items.length() - 1) ?: return null
    return (last.opt("patternDistance") as? Number)?.toDouble()
}

private const val WAYPOINTS_HEADER = "QGC WPL"

internal fun isWaypointsText(head: String): Boolean = head.trimStart().startsWith(WAYPOINTS_HEADER)

private fun isWaypointsFile(file: File): Boolean =
    runCatching { file.bufferedReader().use { isWaypointsText(it.readLine().orEmpty()) } }.getOrDefault(false)

private fun displayName(context: Context, uri: Uri): String? = runCatching {
    context.contentResolver
        .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
        ?.use { if (it.moveToFirst()) it.getString(0) else null }
}.getOrNull()

private const val FILES_STORE = "plan-files"
private const val LAST_DOCUMENT_KEY = "lastDocument"

private fun lastDocument(context: Context): Uri? =
    context.getSharedPreferences(FILES_STORE, Context.MODE_PRIVATE).getString(LAST_DOCUMENT_KEY, null)?.let(Uri::parse)

private fun rememberDocument(context: Context, uri: Uri) {
    context.getSharedPreferences(FILES_STORE, Context.MODE_PRIVATE).edit().putString(LAST_DOCUMENT_KEY, uri.toString()).apply()
}

private fun Intent.startingAt(folder: Uri?): Intent =
    folder?.let { putExtra(DocumentsContract.EXTRA_INITIAL_URI, it) } ?: this

private class OpenFrom(private val folder: () -> Uri?) : ActivityResultContracts.OpenDocument() {
    override fun createIntent(context: Context, input: Array<String>): Intent =
        super.createIntent(context, input).startingAt(folder())
}

private class CreateIn(private val folder: () -> Uri?) : ActivityResultContracts.CreateDocument(PLAN_MIME) {
    override fun createIntent(context: Context, input: String): Intent =
        super.createIntent(context, input).startingAt(folder())
}

private fun suffixed(context: Context, uri: Uri, extension: String): Uri = runCatching {
    val shown = displayName(context, uri) ?: return uri
    val wanted = withExtension(shown, extension)
    if (wanted == shown) uri else DocumentsContract.renameDocument(context.contentResolver, uri, wanted) ?: uri
}.getOrDefault(uri)

@Composable
fun rememberPlanFileActions(onResult: (String) -> Unit = {}): PlanFileActions {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val document = remember { mutableStateOf<Uri?>(null) }
    val name = remember { mutableStateOf<String?>(null) }
    val opened = remember { mutableIntStateOf(0) }

    fun adopt(uri: Uri) {
        document.value = uri
        rememberDocument(context, uri)
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
            // qtpaths: plan.removeAll, plan.removeAllFromVehicle, plan.loadFromVehicle
            val ok = withContext(Dispatchers.Default) { Qgc.invoke("$PLAN_ROOT.$method") }
            if (ok) {
                forget()
                onResult(success)
            } else {
                onResult(failure)
            }
        }
    }

    fun importFrom(uris: List<Uri>, pattern: String) {
        scope.launch {
            val message = withContext(Dispatchers.Default) {
                val labelled = uris.map { it to displayName(context, it) }
                val main = mainBoundaryName(labelled.map { it.second.orEmpty() })
                val label = labelled.firstOrNull { it.second.orEmpty() == main }?.second
                val staged = File(context.cacheDir, boundaryCacheName(label))
                context.cacheDir.listFiles { file -> file.name.startsWith("boundary.") }?.forEach { it.delete() }
                if (!labelled.all { (uri, name) -> copyIn(context, uri, File(context.cacheDir, boundaryCacheName(name))) }) {
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
            val view = withContext(Dispatchers.Default) { freshPlanView() }
            val blocked = saveBlockedReason(view)
            if (blocked == null) action() else {
                PlanFocus.notReady(view)
                onResult(blocked)
            }
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
                adopt(withContext(Dispatchers.IO) { suffixed(context, target, PLAN_EXTENSION) })
                onResult("Plan saved.")
            } else {
                onResult(message)
            }
        }
    }

    val opener = rememberLauncherForActivityResult(OpenFrom { lastDocument(context) }) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val staged = File(context.cacheDir, OPEN_CACHE)
            val failure = withContext(Dispatchers.Default) {
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
            if (withContext(Dispatchers.IO) { isWaypointsFile(staged) }) forget() else adopt(chosen)
            opened.intValue += 1
            onResult("Plan opened.")
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
                suffixed(context, target, KML_EXTENSION)
                null
            }
            onResult(message ?: "KML exported.")
        }
    }

    val kmlCreator = rememberLauncherForActivityResult(
        CreateIn { lastDocument(context) },
    ) { uri -> uri?.let { exportKmlTo(it) } }

    val pendingImport = remember { mutableStateOf<List<Uri>?>(null) }
    val patterns = remember { mutableStateOf<List<String>>(emptyList()) }

    val importer = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        val chosen = uris.takeIf { it.isNotEmpty() } ?: return@rememberLauncherForActivityResult
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
        CreateIn { lastDocument(context) },
    ) { uri -> uri?.let { writeTo(it) } }

    val choosePattern = PatternChoice(
        options = { patterns.value.takeIf { pendingImport.value != null } ?: emptyList() },
        pick = { name ->
            val uris = pendingImport.value
            pendingImport.value = null
            if (uris != null) importFrom(uris, name)
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
            download = { discard("loadFromVehicle", "Loading the plan from the vehicle.", "The plan could not be loaded from the vehicle.") },
            documentName = { name.value },
            opened = { opened.intValue },
        )
    }
}
