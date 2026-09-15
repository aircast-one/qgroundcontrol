package one.aircast.android.ui

import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached

internal const val PAUSE = "pause"

private val SHEET_ACTIONS =
    setOf(
        "startMission", "continueMission", "resumeMission", "cancelRoi", PAUSE,
        "landAbort", "grab", "release", "emergencyStop",
    )

private const val GRIPPER_RELEASE = 0
private const val GRIPPER_GRAB = 1
private const val LAND_ABORT_CLIMB_METERS = 50.0

internal fun moreActions(offers: Map<String, GuidedOffer>): List<GuidedOffer> =
    offers.values.filter { it.shown && it.id in SHEET_ACTIONS }

internal fun guidedCommand(id: String, resumeFrom: Int? = null): (() -> Unit)? = when (id) {
    "startMission", "continueMission" -> ({ offMainDetached { Qgc.invoke("vehicle.startMission") } })
    "landAbort" -> ({ offMainDetached { Qgc.invoke("vehicle.abortLanding", LAND_ABORT_CLIMB_METERS) } })
    "grab" -> ({ offMainDetached { Qgc.invoke("vehicle.sendGripperAction", GRIPPER_GRAB) } })
    "release" -> ({ offMainDetached { Qgc.invoke("vehicle.sendGripperAction", GRIPPER_RELEASE) } })
    "emergencyStop" -> ({ offMainDetached { Qgc.invoke("vehicle.emergencyStop") } })
    "cancelRoi" -> ({ offMainDetached { Qgc.invoke("vehicle.stopGuidedModeROI") } })
    "resumeMission" -> resumeFrom?.let { at ->
        ({ offMainDetached { Qgc.invoke("planFly.missionController.resumeMission", at) } })
    }
    else -> null
}
