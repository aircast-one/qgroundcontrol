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
    val rcOverride: Boolean?,
    val telemetry: TelemetryLink?,
    val summaryDetail: String = "",
    val nominal: Boolean = true,
    val fault: Boolean = false,
    val canArm: Boolean = true,
)

internal data class TelemetryLink(
    val localRssiDbm: Int,
    val remoteRssiDbm: Int?,
    val localNoise: Int?,
    val remoteNoise: Int?,
    val receiveErrors: Int?,
    val errorsFixed: Int? = null,
    val txBuffer: Int? = null,
)

internal fun telemetryLink(view: JSONObject?): TelemetryLink? {
    val radio = view?.optJSONObject("telemetry") ?: return null
    if (radio.isNull("localRssiDbm")) return null
    return TelemetryLink(
        localRssiDbm = radio.optInt("localRssiDbm"),
        remoteRssiDbm = if (radio.isNull("remoteRssiDbm")) null else radio.optInt("remoteRssiDbm"),
        localNoise = if (radio.isNull("localNoise")) null else radio.optInt("localNoise"),
        remoteNoise = if (radio.isNull("remoteNoise")) null else radio.optInt("remoteNoise"),
        receiveErrors = if (radio.isNull("receiveErrors")) null else radio.optInt("receiveErrors"),
        errorsFixed = if (radio.isNull("errorsFixed")) null else radio.optInt("errorsFixed"),
        txBuffer = if (radio.isNull("txBuffer")) null else radio.optInt("txBuffer"),
    )
}

internal fun flyState(view: JSONObject?): FlyState? {
    if (view == null || view.optText("class") != "FlyState") return null
    return FlyState(
        connected = view.optBoolean("connected"),
        armed = view.optBoolean("armed"),
        contactLost = if (view.isNull("contactLost")) null else view.optBoolean("contactLost"),
        state = view.optText("state"),
        stateText = view.optText("stateText"),
        summaryDetail = view.optText("summaryDetail"),
        nominal = view.optBoolean("nominal", true),
        fault = view.optBoolean("fault"),
        canArm = view.optBoolean("canArm", true),
        staleNotice = view.optText("staleNotice"),
        mode = view.optText("mode"),
        rcSupported = view.optBoolean("rcSupported"),
        rcSignalText = view.optText("rcSignalText"),
        rcSignal = if (view.isNull("rcSignal")) null else view.optInt("rcSignal"),
        rcOverride = if (view.isNull("rcOverride")) null else view.optBoolean("rcOverride"),
        telemetry = telemetryLink(view),
    )
}

internal fun vehicleSubtitle(state: FlyState?): String = when {
    state == null || !state.connected -> "No Vehicle"
    state.contactLost == true -> state.stateText
    else -> listOfNotNull(state.mode.ifBlank { null }, state.stateText).joinToString(" · ")
}

@Composable
internal fun hasVehicle(): Boolean {
    val view by qgcPath(FLY_STATE)
    return remember(view) { flyState(view)?.connected == true }
}
