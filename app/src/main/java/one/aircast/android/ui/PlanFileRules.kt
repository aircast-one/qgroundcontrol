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

internal fun discardNeedsConfirming(dirty: Boolean, containsItems: Boolean): Boolean =
    dirty && containsItems

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

// The core spells this too, and knows whether the changes are unsaved or merely
// not uploaded - which is the offline flag this head used to watch for itself.
internal fun planStatusText(view: org.json.JSONObject?, name: String?, dirty: Boolean): String =
    view?.optString("status").orEmpty().ifBlank {
        when {
            name == null && !dirty -> "New plan"
            name == null -> "Unsaved plan"
            else -> name
        }
    }

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
