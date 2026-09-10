package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val AT_END = -1
const val BEFORE_THE_REST = 1

data class InsertOutcome(val ok: Boolean, val reason: String)

private const val NO_ANSWER = "The plan did not answer."

internal fun insertOutcome(view: JSONObject?): InsertOutcome {
    if (view == null) return InsertOutcome(false, NO_ANSWER)
    if (view.optBoolean("ok")) return InsertOutcome(true, "")
    return InsertOutcome(false, view.optString("reason").ifBlank { NO_ANSWER })
}

fun insertMissionItem(kind: String, latitude: Double, longitude: Double, index: Int): InsertOutcome =
    runCatching {
        insertOutcome(
            JSONObject(
                QGCBridge.invoke("mission.insert", "[\"$kind\", $latitude, $longitude, $index]"),
            ),
        )
    }.getOrElse { InsertOutcome(false, NO_ANSWER) }

fun removeMissionItem(index: Int): InsertOutcome =
    runCatching {
        insertOutcome(JSONObject(QGCBridge.invoke("mission.remove", "[$index]")))
    }.getOrElse { InsertOutcome(false, NO_ANSWER) }
