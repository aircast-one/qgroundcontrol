package one.aircast.android.ui

import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.PlanCommands
import one.aircast.android.bridge.VehicleCommands
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

internal const val PAUSE = "pause"

internal val BAR_ACTIONS =
    setOf(
        "arm", "disarm", "takeoff", "land", "rtl", "changeSpeed", "changeAltitude",
    )

internal val SHEET_ACTIONS =
    setOf(
        "startMission", "continueMission", "resumeMission", "cancelRoi", PAUSE,
        "landAbort", "release", "grab", "hold", "vtolTransitionToFixedWing",
        "vtolTransitionToMultiRotor", "forceArm",
    )

internal const val GRIPPER_RELEASE = 0
internal const val GRIPPER_GRAB = 1
internal const val GRIPPER_HOLD = 2
internal const val RELEASE = "release"
private const val LAND_ABORT_CLIMB_METERS = 50.0

internal fun moreActions(offers: Map<String, GuidedOffer>): List<GuidedOffer> =
    offers.values.filter {
        it.shown && it.id !in BAR_ACTIONS && it.id != EMERGENCY_STOP && it.id in SHEET_ACTIONS
    }

private const val LAND_ABORT = "landAbort"

internal val AUTO_POPUP_ACTIONS = listOf(LAND_ABORT, "startMission", "continueMission")

internal fun popupReplacesOpenConfirm(id: String): Boolean = id == LAND_ABORT

internal const val DISMISS_MISSION_PROMPT = "dismissMissionPrompt"

internal fun dismissesMissionPrompt(offerId: String?): Boolean = offerId != null && offerId != LAND_ABORT && offerId in AUTO_POPUP_ACTIONS

internal const val OTHER_DECISION = "decision"

internal fun openDecision(confirmingOffer: String?, confirming: Boolean, panelOpen: Boolean): String? = when {
    confirming -> confirmingOffer ?: OTHER_DECISION
    panelOpen -> OTHER_DECISION
    else -> null
}

internal fun clickMenuAllowed(decision: String?): Boolean = decision == null || dismissesMissionPrompt(decision)

internal fun autoMissionPopup(wasReady: Set<String>, offers: Map<String, GuidedOffer>, enabled: Boolean): GuidedOffer? =
    AUTO_POPUP_ACTIONS.mapNotNull { offers[it] }.firstOrNull { (enabled || it.id == LAND_ABORT) && it.ready && it.id !in wasReady }

internal fun guidedCommand(id: String, resumeFrom: Int?): (suspend () -> String?)? = when (id) {
    "startMission", "continueMission" -> offMainRefusal { VehicleCommands.startMissionRefusal() }
    "landAbort" -> offMainRefusal { VehicleCommands.abortLandingRefusal(LAND_ABORT_CLIMB_METERS) }
    "grab" -> offMainRefusal { VehicleCommands.gripperRefusal(GRIPPER_GRAB) }
    "release" -> offMainRefusal { VehicleCommands.gripperRefusal(GRIPPER_RELEASE) }
    "hold" -> offMainRefusal { VehicleCommands.gripperRefusal(GRIPPER_HOLD) }
    "cancelRoi" -> offMainRefusal { VehicleCommands.stopRoiRefusal() }
    "vtolTransitionToFixedWing" -> offMainRefusal { VehicleCommands.forwardFlightRefusal(true) }
    "vtolTransitionToMultiRotor" -> offMainRefusal { VehicleCommands.forwardFlightRefusal(false) }
    "forceArm" -> offMainRefusal { VehicleCommands.forceArmRefusal() }
    "resumeMission" -> resumeFrom?.let { at -> offMainRefusal { PlanCommands.resumeMissionRefusal(at) } }
    else -> null
}

private fun offMainRefusal(command: () -> String?): suspend () -> String? = { withContext(Dispatchers.Default) { command() } }

internal const val AIRCRAFT_SECTION = "Aircraft"
internal const val SCREEN_SECTION = "Screen"
internal const val CALIBRATE = "Calibrate"
internal const val AUX_SWITCHES_VIEW = "view.auxSwitches"
internal const val AUX_SWITCH = "vehicle.auxSwitch"

internal data class AuxSwitch(val id: String, val on: Boolean, val action: String, val confirm: PursuitConfirm?)

internal fun auxSwitchReadings(view: org.json.JSONObject?): List<AuxSwitch> =
    view?.optJSONArray("switches")?.let { array ->
        (0 until array.length()).mapNotNull { array.optJSONObject(it) }.map {
            AuxSwitch(
                id = it.optString("id"),
                on = it.optBoolean("on"),
                action = it.optString("action"),
                confirm = it.optJSONObject("confirm")?.let { confirm -> PursuitConfirm(confirm.optString("title"), confirm.optString("text")) },
            )
        }
    }.orEmpty()

internal fun auxSwitchIcon(id: String): Int = if (id == "avoidance") R.drawable.ic_shield else R.drawable.ic_straighten
