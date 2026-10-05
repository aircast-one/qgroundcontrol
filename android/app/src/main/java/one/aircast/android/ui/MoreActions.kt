package one.aircast.android.ui

import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.PlanCommands
import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainDetached

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

private const val GRIPPER_RELEASE = 0
private const val GRIPPER_GRAB = 1
private const val GRIPPER_HOLD = 2
private const val LAND_ABORT_CLIMB_METERS = 50.0

internal fun moreActions(offers: Map<String, GuidedOffer>): List<GuidedOffer> =
    offers.values.filter {
        it.shown && it.id !in BAR_ACTIONS && it.id != EMERGENCY_STOP && it.id in SHEET_ACTIONS
    }

private const val LAND_ABORT = "landAbort"

internal val AUTO_POPUP_ACTIONS = listOf(LAND_ABORT, "startMission", "continueMission")

internal fun popupReplacesOpenConfirm(id: String): Boolean = id == LAND_ABORT

internal fun autoMissionPopup(wasReady: Set<String>, offers: Map<String, GuidedOffer>, enabled: Boolean): GuidedOffer? =
    AUTO_POPUP_ACTIONS.mapNotNull { offers[it] }.firstOrNull { (enabled || it.id == LAND_ABORT) && it.ready && it.id !in wasReady }

internal fun guidedCommand(id: String, resumeFrom: Int?): (() -> Unit)? = when (id) {
    "startMission", "continueMission" -> ({ offMainDetached { VehicleCommands.startMission() } })
    "landAbort" -> ({ offMainDetached { VehicleCommands.abortLanding(LAND_ABORT_CLIMB_METERS) } })
    "grab" -> ({ offMainDetached { VehicleCommands.gripper(GRIPPER_GRAB) } })
    "release" -> ({ offMainDetached { VehicleCommands.gripper(GRIPPER_RELEASE) } })
    "hold" -> ({ offMainDetached { VehicleCommands.gripper(GRIPPER_HOLD) } })
    "cancelRoi" -> ({ offMainDetached { VehicleCommands.stopRoi() } })
    "vtolTransitionToFixedWing" -> ({ offMainDetached { VehicleCommands.setForwardFlight(true) } })
    "vtolTransitionToMultiRotor" -> ({ offMainDetached { VehicleCommands.setForwardFlight(false) } })
    "forceArm" -> ({ offMainDetached { VehicleCommands.forceArm() } })
    "resumeMission" -> resumeFrom?.let { at ->
        ({ offMainDetached { PlanCommands.resumeMission(at) } })
    }
    else -> null
}
