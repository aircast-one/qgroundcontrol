package one.aircast.android.ui

import one.aircast.android.bridge.settingControl
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText

internal const val OPERATOR_CONTROL_VIEW = "view.operatorControl"

internal data class ControlStation(
    val known: Boolean,
    val inControl: Boolean?,
    val holderSystemId: Int?,
    val takeoverAllowed: Boolean?,
    val requestAllowed: Boolean,
    val reason: String,
)

internal fun controlStation(view: JSONObject?): ControlStation? {
    if (view == null || !view.optBoolean("available")) return null
    val known = view.optBoolean("known")
    return ControlStation(
        known = known,
        inControl = if (view.isNull("inControl")) null else view.optBoolean("inControl"),
        holderSystemId = if (view.isNull("holderSystemId")) null else view.optInt("holderSystemId"),
        takeoverAllowed = if (view.isNull("takeoverAllowed")) null else view.optBoolean("takeoverAllowed"),
        requestAllowed = view.optBoolean("requestAllowed"),
        reason = view.optText("reason"),
    )
}

internal fun controlIsElsewhere(station: ControlStation?): Boolean = station?.inControl == false

internal fun controlLine(station: ControlStation?): String? = when {
    station == null || !station.known -> null
    station.inControl == true -> null
    station.reason.isBlank() -> null
    else -> station.holderSystemId
        ?.let { "${station.reason} (system $it)" }
        ?: station.reason
}

internal fun acquireLabel(station: ControlStation?): String? = when {
    station == null || !station.known || station.inControl != false -> null
    !station.requestAllowed -> null
    station.takeoverAllowed == true -> "Acquire control"
    else -> "Ask the other station for control"
}

internal fun controlWaitLine(station: ControlStation?): String? = when {
    station == null || !station.known -> null
    station.inControl != false -> null
    station.requestAllowed -> null
    else -> "Waiting for the other station to answer"
}

internal fun requestTimeoutSeconds(station: ControlStation, setting: Int): Int =
    if (station.takeoverAllowed == true) 0 else setting

internal const val REQUEST_CONTROL = "vehicle.requestOperatorControl"
internal val ALLOW_TAKEOVER_SETTING = settingControl("settings.flyViewSettings.requestControlAllowTakeover")
internal val REQUEST_TIMEOUT_SETTING = settingControl("settings.flyViewSettings.requestControlTimeout")

internal fun askForControl(station: ControlStation): String? {
    val allowTakeover = Qgc.get(ALLOW_TAKEOVER_SETTING).let { read ->
        if (read.isNull("value")) null else read.optBoolean("value")
    } ?: return "This station cannot tell whether it would allow a takeover, so it did not ask."
    val timeout = Qgc.get(REQUEST_TIMEOUT_SETTING).let { read ->
        if (read.isNull("value")) null else read.optInt("value")
    } ?: return "This station cannot tell how long it would wait, so it did not ask."
    val asked = Qgc.invoke(REQUEST_CONTROL, allowTakeover, requestTimeoutSeconds(station, timeout))
    return if (asked) null else "The vehicle did not take the request."
}
