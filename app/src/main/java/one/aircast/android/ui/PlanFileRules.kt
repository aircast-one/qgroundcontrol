package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject

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

internal fun planStatusText(name: String?, dirty: Boolean, offline: Boolean): String = when {
    name == null && !dirty -> "New plan"
    name == null -> "Unsaved plan"
    !dirty -> name
    offline -> "$name \u00b7 unsaved changes"
    else -> "$name \u00b7 not uploaded"
}

private fun JSONArray.itemsAfterMissionSettings(): List<JSONObject> =
    (1 until length()).mapNotNull { optJSONObject(it) }

internal fun undrawnItemNames(elements: JSONArray): List<String> =
    elements.itemsAfterMissionSettings()
        .filterNot { it.optBoolean("isSimpleItem") || it.optBoolean("isSurveyItem") }
        .map { it.optString("patternName").ifBlank { it.optString("class") } }
        .filter { it.isNotBlank() }
        .distinct()

internal fun undrawnItemsWarning(names: List<String>): String? = when {
    names.isEmpty() -> null
    else -> "The map cannot draw ${names.joinToString(", ")}. " +
        "Those items are still in the plan and will still be flown."
}
