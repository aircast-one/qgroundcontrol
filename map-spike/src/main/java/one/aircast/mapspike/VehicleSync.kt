package one.aircast.mapspike

enum class VehicleSync { Offline, Busy, Ready }

fun vehicleSyncState(offline: Boolean, syncing: Boolean): VehicleSync = when {
    offline -> VehicleSync.Offline
    syncing -> VehicleSync.Busy
    else -> VehicleSync.Ready
}

fun syncRefusal(state: VehicleSync, action: String): String? = when (state) {
    VehicleSync.Offline -> "No vehicle to $action"
    VehicleSync.Busy -> "Already syncing, wait for it to finish"
    VehicleSync.Ready -> null
}

data class UploadGate(
    val canSend: Boolean,
    val canProceed: Boolean,
    val pausesFirst: Boolean,
    val heading: String,
    val refusal: String,
    val proceedTitle: String,
)

fun notReadyToSend(view: org.json.JSONObject?): String? =
    view?.optJSONObject("readiness")
        ?.takeIf { !it.optBoolean("ready") }
        ?.let { it.optText("reason").ifBlank { "The plan is not ready to send." } }

fun uploadGate(view: org.json.JSONObject?): UploadGate? =
    view?.optJSONObject("upload")?.let {
        UploadGate(
            canSend = it.optBoolean("canSend"),
            canProceed = it.optBoolean("canProceed"),
            pausesFirst = it.optBoolean("pausesFirst"),
            heading = it.optText("heading"),
            refusal = it.optText("refusal"),
            proceedTitle = it.optText("proceedTitle"),
        )
    }

sealed interface UploadStep {
    data object Send : UploadStep
    data class Confirm(val gate: UploadGate) : UploadStep
    data class Refuse(val reason: String) : UploadStep
}

fun uploadStep(gate: UploadGate?, notReady: String? = null): UploadStep = when {
    notReady != null -> UploadStep.Refuse(notReady)
    gate == null -> UploadStep.Refuse("The plan could not be checked against the vehicle.")
    gate.canSend -> UploadStep.Send
    gate.canProceed -> UploadStep.Confirm(gate)
    else -> UploadStep.Refuse(gate.refusal.ifBlank { "This plan cannot be uploaded." })
}

// Read at the moment of the attempt, not from a watched copy. The precheck answers "should
// this plan go to this vehicle right now", and a vehicle can begin flying the mission between
// one poll and the operator's tap - which is exactly the case that must pause first.
fun freshPlanView(): org.json.JSONObject? =
    runCatching { org.json.JSONObject(org.mavlink.qgroundcontrol.QGCBridge.get("view.plan")) }.getOrNull()
