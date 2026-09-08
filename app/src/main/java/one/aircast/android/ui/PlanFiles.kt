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
private const val KML_CACHE = "export.kml"

internal const val PLAN_MIME = "*/*"

internal val PLAN_OPEN_TYPES = arrayOf(PLAN_MIME)

internal const val DEFAULT_PLAN_NAME = "mission.plan"
internal const val DEFAULT_KML_NAME = "mission.kml"

data class PlanActions(
    val open: Boolean,
    val save: Boolean,
    val exportKml: Boolean,
    val newPlan: Boolean,
    val clearMission: Boolean,
)

internal fun planActions(
    syncing: Boolean,
    containsItems: Boolean,
    hasMissionItems: Boolean,
    offline: Boolean,
) = PlanActions(
    open = !syncing,
    save = !syncing && containsItems,
    exportKml = !syncing && hasMissionItems,
    newPlan = !syncing,
    clearMission = !offline && !syncing,
)

internal const val READY_FOR_SAVE = 0
internal const val NOT_READY_TERRAIN = 1
internal const val NOT_READY_DATA = 2

internal fun saveBlockedReason(state: Int?): String? = when (state) {
    READY_FOR_SAVE -> null
    NOT_READY_TERRAIN -> "Waiting on terrain data. Saving now would store wrong altitudes."
    NOT_READY_DATA -> "Some items still need a position or a value."
    else -> "The plan could not be checked for saving."
}

enum class PlanConfirm { Open, NewPlan, ClearMission }

data class ConfirmCopy(
    val title: String,
    val body: String,
    val confirm: String,
    val destructive: Boolean = false,
)

internal fun confirmCopy(kind: PlanConfirm): ConfirmCopy = when (kind) {
    PlanConfirm.Open -> ConfirmCopy(
        "Discard unsaved changes?",
        "Opening a plan replaces the one you have. Your unsaved changes cannot be recovered.",
        "Discard and open",
    )
    PlanConfirm.NewPlan -> ConfirmCopy(
        "Discard unsaved changes?",
        "Starting a new plan clears the one you have. Your unsaved changes cannot be recovered.",
        "Discard and start new",
    )
    PlanConfirm.ClearMission -> ConfirmCopy(
        "Clear the mission from the vehicle?",
        "This removes the mission from the aircraft as well as from this plan. It cannot be undone.",
        "Clear mission",
        destructive = true,
    )
}

internal fun planStatusText(name: String?, dirty: Boolean): String = when {
    name == null && !dirty -> "New plan"
    name == null -> "Unsaved plan"
    dirty -> "$name \u00b7 unsaved changes"
    else -> name
}

class PlanFileActions(
    val open: () -> Unit,
    val saveAs: () -> Unit,
    val save: () -> Unit,
    val exportKml: () -> Unit,
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
            val message = withContext(Dispatchers.Default) {
                val staged = File(context.cacheDir, OPEN_CACHE)
                staged.delete()
                if (!copyIn(context, chosen, staged)) {
                    return@withContext "That file could not be read."
                }
                loadFailureMessage(planLoad(staged.absolutePath))
            }
            if (message == null) {
                adopt(chosen)
                onResult("Plan opened.")
            } else {
                onResult(message)
            }
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

    val creator = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(PLAN_MIME),
    ) { uri -> uri?.let { writeTo(it) } }

    return remember(opener, creator, kmlCreator) {
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
