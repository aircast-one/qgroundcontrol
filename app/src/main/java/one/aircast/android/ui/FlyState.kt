package one.aircast.android.ui

import org.json.JSONObject

internal const val FLY_STATE = "view.flyState"

internal data class FlyState(
    val connected: Boolean,
    val contactLost: Boolean,
    val state: String,
    val stateText: String,
    val staleNotice: String,
    val mode: String,
)

internal fun flyState(view: JSONObject?): FlyState? {
    if (view == null || view.optString("class") != "FlyState") return null
    return FlyState(
        connected = view.optBoolean("connected"),
        contactLost = view.optBoolean("contactLost"),
        state = view.optString("state"),
        stateText = view.optString("stateText"),
        staleNotice = view.optString("staleNotice"),
        mode = view.optString("mode"),
    )
}

internal fun vehicleSubtitle(state: FlyState?): String = when {
    state == null || !state.connected -> "No vehicle"
    state.contactLost -> state.stateText
    else -> listOfNotNull(state.mode.ifBlank { null }, state.stateText).joinToString(" · ")
}
