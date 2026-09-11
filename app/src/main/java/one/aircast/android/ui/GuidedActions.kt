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
) {
    val ready: Boolean get() = offer == "ready"
    val blocked: Boolean get() = offer == "blocked"
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
                )
            }
        }
    }.toMap()
}

internal val PRIMARY_ACTIONS = listOf("arm", "disarm", "takeoff", "land", "rtl")

internal fun primaryBlockedReason(offers: Map<String, GuidedOffer>): String? =
    PRIMARY_ACTIONS.firstNotNullOfOrNull { offers[it]?.let(::blockedReasonFor) }

internal fun blockedReasonFor(offer: GuidedOffer?): String? =
    offer?.takeIf { it.blocked }?.reason?.ifBlank { "The vehicle will not accept this yet." }
