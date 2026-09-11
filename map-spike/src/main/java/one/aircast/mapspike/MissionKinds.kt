package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val AT_END = -1
const val HOME_ITEM = 0
const val BEFORE_THE_REST = 1

data class InsertOutcome(val ok: Boolean, val reason: String, val index: Int? = null)

private const val NO_ANSWER = "The plan did not answer."

internal fun insertOutcome(view: JSONObject?): InsertOutcome {
    if (view == null) return InsertOutcome(false, NO_ANSWER)
    if (view.optBoolean("ok")) {
        return InsertOutcome(true, "", view.opt("index").let { it as? Number }?.toInt())
    }
    return InsertOutcome(false, view.optText("reason").ifBlank { NO_ANSWER })
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
