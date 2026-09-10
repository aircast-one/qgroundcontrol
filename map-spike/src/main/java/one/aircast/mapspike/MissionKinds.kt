package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

data class MissionKind(val id: String, val enabled: Boolean?, val disabledReason: String)

internal fun missionKind(view: JSONObject?): MissionKind? {
    if (view == null) return null
    val id = view.optString("id")
    if (id.isBlank()) return null
    return MissionKind(
        id = id,
        enabled = if (view.isNull("enabled")) null else view.optBoolean("enabled"),
        disabledReason = view.optString("disabledReason"),
    )
}

internal fun kindRefusal(kind: MissionKind?): String? = when {
    kind?.enabled == false -> kind.disabledReason.ifBlank { "That item does not belong here in the mission." }
    else -> null
}

private fun askAboutTheEnd() {
    val last = (PlanBridge.rawItemCount() ?: 0) - 1
    if (last >= 0) {
        QGCBridge.invoke("plan.missionController.setCurrentPlanViewSeqNum", "[$last,true]")
    }
}

fun freshMissionKind(id: String): MissionKind? {
    askAboutTheEnd()
    return runCatching { missionKind(JSONObject(QGCBridge.get("view.missionKinds($id)"))) }.getOrNull()
}
