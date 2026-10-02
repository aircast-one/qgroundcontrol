package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val GUIDED_ACTIONS = "view.guidedActions"

internal data class GuidedOffer(
    val id: String,
    val title: String,
    val offer: String,
    val reason: String,
    val prompt: String,
    val destructive: Boolean,
    val carriesValue: Boolean,
    val option: String = "",
) {
    val ready: Boolean get() = offer == "ready"
    val shown: Boolean get() = offer != "hidden"
}

internal fun guidedOffers(view: JSONObject?): Map<String, GuidedOffer> {
    val actions = view?.optJSONArray("actions") ?: return emptyMap()
    return (0 until actions.length()).mapNotNull { index ->
        actions.optJSONObject(index)?.let { action ->
            val id = action.optText("id")
            if (id.isBlank()) {
                null
            } else {
                id to GuidedOffer(
                    id = id,
                    title = action.optText("title"),
                    offer = action.optText("offer"),
                    reason = action.optText("reason"),
                    prompt = action.optText("prompt"),
                    destructive = action.optBoolean("destructive"),
                    carriesValue = action.optBoolean("carriesValue"),
                    option = action.optText("option"),
                )
            }
        }
    }.toMap()
}

internal fun resumeFromSequence(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("resumeFromSequence") }?.optInt("resumeFromSequence")?.takeIf { it > 0 }

