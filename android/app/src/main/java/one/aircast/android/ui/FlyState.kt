package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import one.aircast.map.optText

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
    val readyToFly: Boolean = true,
)

internal enum class ChipTone { Error, Neutral, Warning, Success }

internal fun notReadyToFly(state: FlyState?): Boolean =
    state != null && state.connected && !state.armed && state.contactLost != true && !state.readyToFly

internal fun chipTone(state: FlyState?, lost: Boolean): ChipTone = when {
    lost || state?.fault == true -> ChipTone.Error
    state?.connected != true -> ChipTone.Neutral
    !state.nominal -> ChipTone.Warning
    notReadyToFly(state) -> ChipTone.Neutral
    else -> ChipTone.Success
}

internal const val ALL_CHECKS_PASSED = "All checks passed."
internal const val SETUP_NOT_COMPLETE = "Aircraft setup is not complete."

internal fun readinessWarning(state: FlyState?): String? =
    state?.takeIf { it.connected && !it.armed && (notReadyToFly(it) || !it.nominal) }
        ?.let { listOfNotNull(it.stateText.ifBlank { null }, (if (notReadyToFly(it) && it.summaryDetail == ALL_CHECKS_PASSED) SETUP_NOT_COMPLETE else it.summaryDetail).ifBlank { null }).joinToString(". ") }
        ?.ifBlank { null }

internal data class Readiness(val text: String, val blocks: Boolean)

internal const val READINESS_BLOCKED = "The vehicle will refuse to arm until this is fixed."

internal fun guidedReadiness(state: FlyState?): Readiness? =
    readinessWarning(state)?.let { text ->
        val blocks = state?.canArm == false
        Readiness(if (blocks) "${text.removeSuffix(".")}. $READINESS_BLOCKED" else text, blocks)
    }

internal fun disarmNotice(wasArmed: Boolean, flewWhileArmed: Boolean, armedNow: Boolean): String? =
    if (wasArmed && !armedNow) (if (flewWhileArmed) "Landed and disarmed" else "Disarmed") else null

internal const val VEHICLE_FLIGHT_DISTANCE = "vehicle.flightDistance"

internal fun flownDistanceText(view: JSONObject?): String? =
    view?.takeIf { it.optDouble("value", 0.0) > 0.0 }
        ?.let { fact -> fact.optText("valueString").ifBlank { null }?.let { "$it ${fact.optText("units")}".trim() } }

internal fun landedSummary(seconds: Double?, distance: String?, batteryUsed: Int?): String =
    listOfNotNull(
        seconds?.takeIf { it >= 1.0 }?.let(::flightTimeText),
        distance?.ifBlank { null },
        batteryUsed?.takeIf { it > 0 }?.let { "$it% battery used" },
    ).joinToString(" \u00b7 ")

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
        stateText = view.optText("stateText").split(" · ").joinToString(" · ", transform = ::sentenceCase),
        summaryDetail = view.optText("summaryDetail"),
        nominal = view.optBoolean("nominal", true),
        fault = view.optBoolean("fault"),
        canArm = view.optBoolean("canArm", true),
        readyToFly = view.optBoolean("readyToFly", true),
        staleNotice = view.optText("staleNotice"),
        mode = view.optText("mode"),
        rcSupported = view.optBoolean("rcSupported"),
        rcSignalText = view.optText("rcSignalText"),
        rcSignal = if (view.isNull("rcSignal")) null else view.optInt("rcSignal"),
        rcOverride = if (view.isNull("rcOverride")) null else view.optBoolean("rcOverride"),
        telemetry = telemetryLink(view),
    )
}

internal fun offlineMainStatus(view: JSONObject?): String? =
    view?.optText("mainStatus")?.ifBlank { null }?.let(::sentenceCase)

internal fun vehicleSubtitle(state: FlyState?, offline: String? = null): String = when {
    state == null || !state.connected -> offline ?: "No vehicle"
    state.contactLost == true -> state.stateText
    else -> listOfNotNull(state.mode.ifBlank { null }, state.stateText).joinToString(" · ")
}

@Composable
internal fun hasVehicle(): Boolean {
    val view by qgcPath(FLY_STATE)
    return remember(view) { flyState(view)?.connected == true }
}
