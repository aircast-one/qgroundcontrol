package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val DEFAULT_PLAN_NAME = "mission.plan"
internal const val DEFAULT_KML_NAME = "mission.kml"
internal const val DEFAULT_BOUNDARY_EXT = "kml"
internal const val PLAN_EXTENSION = "plan"
internal const val KML_EXTENSION = "kml"

internal fun withExtension(name: String, extension: String): String =
    if (name.endsWith(".$extension", ignoreCase = true)) name else "$name.$extension"

internal fun boundaryCacheName(displayName: String?): String {
    val ext = displayName?.substringAfterLast('.', "")?.lowercase()?.takeIf { it.isNotBlank() }
    return "boundary.${ext ?: DEFAULT_BOUNDARY_EXT}"
}

internal fun mainBoundaryName(names: List<String>): String? =
    names.firstOrNull { it.endsWith(".shp", ignoreCase = true) } ?: names.firstOrNull { it.endsWith(".kml", ignoreCase = true) } ?: names.firstOrNull()

internal fun importedNothing(distance: Double?): Boolean = distance == null || distance <= 0.0

data class PlanActions(
    val open: Boolean,
    val save: Boolean,
    val exportKml: Boolean,
    val newPlan: Boolean,
    val clearFromVehicle: Boolean,
    val download: Boolean = false,
)

// Every one of these was computed here from four watched properties, in the same
// shapes the core already publishes. Two implementations of one rule agree until
// the day the producer changes its mind and only one of them follows.
internal fun planActions(view: org.json.JSONObject?): PlanActions {
    val actions = view?.optJSONObject("actions")
    fun allowed(name: String) = actions?.optBoolean(name) == true
    return PlanActions(
        open = allowed("open"),
        save = allowed("save"),
        exportKml = allowed("exportKml"),
        newPlan = allowed("newPlan"),
        clearFromVehicle = allowed("clearMission"),
        download = allowed("download"),
    )
}

internal data class PlanHistory(val canUndo: Boolean, val canRedo: Boolean)

internal fun planHistory(view: org.json.JSONObject?): PlanHistory = PlanHistory(
    canUndo = view?.optBoolean("canUndo") == true,
    canRedo = view?.optBoolean("canRedo") == true,
)

internal fun planIsDirty(view: JSONObject?): Boolean = view?.optBoolean("dirty") == true

// view.plan's sync state is offline, busy or ready - plan.rs sync_json - and this compared against
// "syncing", which it never is, so a sync in flight read as idle.
internal fun planIsSyncing(view: JSONObject?): Boolean =
    view?.optJSONObject("sync")?.optText("state") == "busy"

internal fun planSyncProgress(view: JSONObject?): Float =
    view?.optJSONObject("sync")?.optDouble("progress", 0.0)?.toFloat()?.coerceIn(0f, 1f) ?: 0f

internal fun planContainsItems(view: JSONObject?): Boolean = view?.optBoolean("containsItems") == true

// The core maps readyForSaveState to a sentence in readiness.reason. This head
// mapped the same three states to different words, so which sentence an operator
// saw depended on which path refused.
internal fun saveBlockedReason(view: org.json.JSONObject?): String? {
    val readiness = view?.optJSONObject("readiness")
        ?: return "The plan could not be checked for saving."
    if (readiness.optBoolean("ready")) {
        return null
    }
    return readiness.optText("reason").ifBlank { "The plan could not be checked for saving." }
}

enum class PlanConfirm { Open, NewPlan, ClearMission, Download }

data class ConfirmCopy(
    val title: String,
    val body: String,
    val confirm: String,
    val destructive: Boolean = false,
)

internal fun confirmCopy(kind: PlanConfirm): ConfirmCopy = when (kind) {
    PlanConfirm.Open -> ConfirmCopy(
        "Plan overwrite",
        "You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?",
        "Load from file",
    )
    PlanConfirm.NewPlan -> ConfirmCopy(
        "Create Plan",
        "Are you sure you want to remove current plan and create a new plan?",
        "Create plan",
    )
    PlanConfirm.Download -> ConfirmCopy(
        "Plan overwrite",
        "You have unsaved/unsent changes. Loading from the Vehicle will lose these changes. Are you sure you want to load from the Vehicle?",
        "Load from vehicle",
    )
    PlanConfirm.ClearMission -> ConfirmCopy(
        "Clear the mission from the vehicle?",
        "This removes the mission from the aircraft as well as from this plan. It cannot be undone.",
        "Clear mission",
        destructive = true,
    )
}

internal fun planTitle(documentName: String?): String =
    documentName?.ifBlank { null }?.let { name -> name.lastIndexOf('.').takeIf { it > 0 }?.let { name.substring(0, it) } ?: name } ?: "Untitled Plan"

internal fun planStatusText(view: org.json.JSONObject?): String =
    view?.optText("status").orEmpty()

internal val DRAWN_KINDS = setOf(
    "settings", "takeoff", "land", "waypoint", "command", "altitude", "roi",
    "survey", "corridor", "structure",
)

private fun JSONArray.itemsAfterMissionSettings(): List<JSONObject> =
    (1 until length()).mapNotNull { optJSONObject(it) }

internal fun undrawnItemNames(items: JSONArray): List<String> =
    (0 until items.length())
        .mapNotNull { items.optJSONObject(it) }
        .filterNot { it.optText("kind") in DRAWN_KINDS }
        .map { it.optText("name") }
        .filter { it.isNotBlank() }
        .distinct()

internal fun undrawnItemsWarning(names: List<String>): String? = when {
    names.isEmpty() -> null
    else -> "The map cannot draw ${names.joinToString(", ")}. " +
        "Those items are still in the plan and will still be flown."
}
