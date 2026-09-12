package one.aircast.mapspike

import org.json.JSONObject

const val MISSION_CONTEXT = "mission"
const val ALT_MODE_MIXED = 0

data class AltitudeModeOffer(
    val raw: Int,
    val title: String,
    val help: String,
    val enabled: Boolean,
    val current: Boolean,
    val reason: String,
)

data class AltitudeModesView(
    val context: String,
    val current: Int,
    val offers: List<AltitudeModeOffer>,
    val omitted: List<AltitudeModeOffer>,
)

fun altitudeModesPath(context: String, current: Int): String = "view.altitudeModes($context,$current)"

private fun offers(view: JSONObject?, key: String): List<AltitudeModeOffer> {
    val items = view?.optJSONArray(key) ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let {
            AltitudeModeOffer(
                raw = it.optInt("raw", -1),
                title = it.optText("title"),
                help = it.optText("help"),
                enabled = it.optBoolean("enabled"),
                current = it.optBoolean("current"),
                reason = it.optText("reason"),
            )
        }
    }
}

fun altitudeModesView(view: JSONObject?): AltitudeModesView? {
    if (view == null || view.optText("class") != "AltitudeModes") return null
    return AltitudeModesView(
        context = view.optText("context"),
        current = view.optInt("current", -1),
        offers = offers(view, "modes"),
        omitted = offers(view, "omitted"),
    )
}

fun choosable(view: AltitudeModesView?): List<AltitudeModeOffer> =
    view?.offers.orEmpty().filter { it.raw != ALT_MODE_MIXED }

fun refusalFor(view: AltitudeModesView?, raw: Int): String? =
    view?.offers.orEmpty().firstOrNull { it.raw == raw }?.takeIf { !it.enabled }?.reason?.ifBlank { null }

fun altitudeModePath(index: Int): String = "plan.missionController.visualItems.$index.altitudeMode"
