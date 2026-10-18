package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.map.optText

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
    val reasonCode: String = "",
    val confirm: Boolean = true,
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
                    reasonCode = action.optText("reasonCode"),
                    option = action.optText("option"),
                    confirm = action.optBoolean("confirm", true),
                )
            }
        }
    }.toMap()
}

internal data class PayloadProblem(val code: String, val text: String, val blocking: Boolean, val fix: String)

internal data class Payload(val bays: Int, val empty: List<Int>, val next: Int?, val problems: List<PayloadProblem> = emptyList())

internal val SINGLE_GRIPPER = Payload(bays = 1, empty = emptyList(), next = null)

internal fun payloadOf(view: JSONObject?): Payload {
    val payload = view?.optJSONObject("payload") ?: return SINGLE_GRIPPER
    val empty = payload.optJSONArray("empty")?.let { array -> (0 until array.length()).map { array.optInt(it) } }.orEmpty()
    val problems = payload.optJSONArray("problems")?.let { array ->
        (0 until array.length()).mapNotNull { array.optJSONObject(it) }.map { PayloadProblem(it.optText("code"), it.optText("text"), it.optBoolean("blocking"), it.optText("fix")) }
    }.orEmpty()
    return Payload(bays = payload.optInt("bays", 1).coerceAtLeast(1), empty = empty, next = payload.takeUnless { it.isNull("next") }?.optInt("next"), problems = problems)
}

internal val Payload.multiBay: Boolean get() = bays > 1

internal val Payload.bayNumbers: List<Int> get() = (1..bays).toList()

internal fun resumeFromSequence(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("resumeFromSequence") }?.optInt("resumeFromSequence")?.takeIf { it > 0 }

