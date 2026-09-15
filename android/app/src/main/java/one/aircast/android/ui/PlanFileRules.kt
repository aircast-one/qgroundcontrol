package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val DEFAULT_PLAN_NAME = "mission.plan"
internal const val DEFAULT_KML_NAME = "mission.kml"
internal const val DEFAULT_BOUNDARY_EXT = "kml"

internal fun boundaryCacheName(displayName: String?): String {
    val ext = displayName?.substringAfterLast('.', "")?.lowercase()?.takeIf { it.isNotBlank() }
    return "boundary.${ext ?: DEFAULT_BOUNDARY_EXT}"
}

internal fun importedNothing(distance: Double?): Boolean = distance == null || distance <= 0.0

data class PlanActions(
    val open: Boolean,
    val save: Boolean,
    val exportKml: Boolean,
    val newPlan: Boolean,
    val clearFromVehicle: Boolean,
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
    )
}

internal data class PlanHistory(val canUndo: Boolean, val canRedo: Boolean)

internal fun planHistory(view: org.json.JSONObject?): PlanHistory = PlanHistory(
    canUndo = view?.optBoolean("canUndo") == true,
    canRedo = view?.optBoolean("canRedo") == true,
)

internal fun discardNeedsConfirming(dirty: Boolean, containsItems: Boolean): Boolean =
    dirty && containsItems

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
