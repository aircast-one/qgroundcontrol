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

internal fun holderLine(station: ControlStation?): String? =
    station?.takeIf { it.known && it.inControl == false }?.let { held ->
        held.holderSystemId?.let { "System in control: $it" } ?: held.reason.ifBlank { null }
    }

internal fun acquireLabel(station: ControlStation?): String? = when {
    station == null || !station.known || station.inControl != false -> null
    station.takeoverAllowed == true -> "Acquire control"
    else -> "Send request"
}

internal fun acquireEnabled(station: ControlStation, counting: Boolean): Boolean = station.requestAllowed && !counting

internal fun controlWaitLine(station: ControlStation?): String? = when {
    station == null || !station.known -> null
    station.inControl != false -> null
    station.requestAllowed -> null
    else -> "Waiting for the other station to answer"
}

internal fun inControlLine(station: ControlStation?): String? =
    station?.takeIf { it.known && it.inControl == true }?.let { held ->
        held.holderSystemId?.let { "System in control: This GCS ($it)" } ?: "System in control: This GCS"
    }

internal fun takeoverLine(station: ControlStation?): String? =
    station?.takeIf { it.known }?.takeoverAllowed?.let { if (it) "Takeover allowed" else "Takeover NOT allowed" }

internal fun takeoverChangeable(station: ControlStation?, allowTakeover: Boolean?): Boolean =
    station?.inControl == true && allowTakeover != null && station.takeoverAllowed != allowTakeover

internal fun requestSentLabel(remainingMs: Long): String =
    "Request sent: ${String.format(java.util.Locale.US, "%.1f", remainingMs.coerceAtLeast(0) / 1000.0)}"

internal fun allowTakeoverSetting(): Boolean? =
    Qgc.get(ALLOW_TAKEOVER_SETTING).let { read -> if (read.isNull("value")) null else read.optBoolean("value") }

internal fun saveAllowTakeover(allow: Boolean): String? =
    if (Qgc.set("$ALLOW_TAKEOVER_PATH.rawValue", allow)) null else "This station could not save whether it allows a takeover."

internal fun changeTakeover(allow: Boolean): String? =
    if (Qgc.invoke(REQUEST_CONTROL, allow, 0)) null else "The vehicle did not take the change."

internal fun allowTakeoverEditable(station: ControlStation?): Boolean =
    station?.inControl == true || station?.takeoverAllowed == true

internal fun controlSectionTitle(station: ControlStation): String? = when (station.inControl) {
    true -> "Change takeover condition"
    false -> "Send control request"
    null -> null
}

internal fun requestTimeoutEditable(station: ControlStation): Boolean =
    station.inControl == false && station.takeoverAllowed != true

internal fun requestTimeoutSeconds(station: ControlStation, setting: Int): Int =
    if (station.takeoverAllowed == true) 0 else setting

internal const val REQUEST_CONTROL = "vehicle.requestOperatorControl"
internal const val ALLOW_TAKEOVER_PATH = "settings.flyViewSettings.requestControlAllowTakeover"
internal val ALLOW_TAKEOVER_SETTING = settingControl(ALLOW_TAKEOVER_PATH)
internal const val REQUEST_TIMEOUT_PATH = "settings.flyViewSettings.requestControlTimeout"
internal val REQUEST_TIMEOUT_SETTING = settingControl(REQUEST_TIMEOUT_PATH)
internal const val GCS_SYSTEM_ID_PATH = "settings.mavlinkSettings.gcsMavlinkSystemID"

internal data class ControlAsk(val refusal: String?, val timeoutSeconds: Int)

internal fun askForControl(station: ControlStation): ControlAsk {
    val allowTakeover = allowTakeoverSetting() ?: return ControlAsk("This station cannot tell whether it would allow a takeover, so it did not ask.", 0)
    val timeout = Qgc.get(REQUEST_TIMEOUT_SETTING).let { read ->
        if (read.isNull("value")) null else read.optInt("value")
    } ?: return ControlAsk("This station cannot tell how long it would wait, so it did not ask.", 0)
    val seconds = requestTimeoutSeconds(station, timeout)
    val asked = Qgc.invoke(REQUEST_CONTROL, allowTakeover, seconds)
    return if (asked) ControlAsk(null, seconds) else ControlAsk("The vehicle did not take the request.", 0)
}
