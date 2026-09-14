package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val FLY_STATE = "view.flyState"

internal data class FlyState(
    val connected: Boolean,
    val contactLost: Boolean,
    val state: String,
    val stateText: String,
    val staleNotice: String,
    val mode: String,
    val rcSupported: Boolean,
    val rcSignalText: String,
    val rcSignal: Int?,
)

internal fun flyState(view: JSONObject?): FlyState? {
    if (view == null || view.optText("class") != "FlyState") return null
    return FlyState(
        connected = view.optBoolean("connected"),
        contactLost = view.optBoolean("contactLost"),
        state = view.optText("state"),
        stateText = view.optText("stateText"),
        staleNotice = view.optText("staleNotice"),
        mode = view.optText("mode"),
        rcSupported = view.optBoolean("rcSupported"),
        rcSignalText = view.optText("rcSignalText"),
        rcSignal = if (view.isNull("rcSignal")) null else view.optInt("rcSignal"),
    )
}

internal fun vehicleSubtitle(state: FlyState?): String = when {
    state == null || !state.connected -> "No vehicle"
    state.contactLost -> state.stateText
    else -> listOfNotNull(state.mode.ifBlank { null }, state.stateText).joinToString(" · ")
}
