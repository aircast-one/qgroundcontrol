package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val FLY_STATE = "view.flyState"

internal data class FlyState(
    val connected: Boolean,
    val armed: Boolean,
    val contactLost: Boolean?,
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
        armed = view.optBoolean("armed"),
        contactLost = if (view.isNull("contactLost")) null else view.optBoolean("contactLost"),
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
    state.contactLost == true -> state.stateText
    else -> listOfNotNull(state.mode.ifBlank { null }, state.stateText).joinToString(" · ")
}

@Composable
internal fun hasVehicle(): Boolean {
    val view by qgcPath(FLY_STATE)
    return remember(view) { flyState(view)?.connected == true }
}
