package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val PLAN_ROOT = "plan"
const val PLAN_ITEMS = "$PLAN_ROOT.missionController.visualItems"
const val PLAN_VIEW = "view.missionItems(geometry)"

const val KIND_TAKEOFF = "takeoff"
const val KIND_SURVEY = "survey"

const val MAV_CMD_NAV_RETURN_TO_LAUNCH = 20

internal fun planItems(json: JSONObject?): JSONArray? = json?.optJSONArray("items")

fun planItemCount(json: JSONObject?): Int =
    ((planItems(json)?.length() ?: 0) - 1).coerceAtLeast(0)

fun linksStartToHome(json: JSONObject?): Boolean =
    planItems(json)?.optJSONObject(1)?.optString("kind") == KIND_TAKEOFF

fun planShape(json: JSONObject?): List<String> {
    val items = planItems(json) ?: return emptyList()
    val endsAfter = routeEndsAfter(items)

    val named = (1 until items.length())
        .mapNotNull { index -> items.optJSONObject(index) }
        .mapNotNull { element ->
            when {
                element.optString("kind") == KIND_TAKEOFF -> "takeoff"
                element.optInt("command") == MAV_CMD_NAV_RETURN_TO_LAUNCH -> "RTL"
                else -> null
            }
        }
        .distinct()

    val stranded = (1 until items.length()).count { it > endsAfter }

    return named + listOfNotNull(
        stranded.takeIf { it > 0 }?.let { "$it after the landing" },
    )
}

data class MissionItem(
    val index: Int,
    val sequence: Int,
    val latitude: Double,
    val longitude: Double,
    val command: String,
    val selected: Boolean,
    val altitude: Double = Double.NaN,
    val exit: TrackPoint? = null,
    val routed: Boolean = true,
    val kind: String = "",
    val commandId: Int = 0,
    val placed: Boolean = true,
    val afterRouteEnds: Boolean = false,
    val altitudeText: String = "",
    val specifiesCoordinate: Boolean = false,
)

fun routeEndsAfter(items: JSONArray?): Int =
    (0 until (items?.length() ?: 0))
        .firstOrNull { index -> items?.optJSONObject(index)?.optBoolean("endsRoute") == true }
        ?: Int.MAX_VALUE

internal fun placed(element: JSONObject, key: String): TrackPoint? {
    val at = element.optJSONObject(key) ?: return null
    val latitude = at.optDouble("latitude", Double.NaN)
    val longitude = at.optDouble("longitude", Double.NaN)
    return TrackPoint(latitude, longitude).takeIf { isPlottable(latitude, longitude) }
}

fun allMissionItems(json: JSONObject?): List<MissionItem> {
    val items = planItems(json) ?: return emptyList()
    val endsAfter = routeEndsAfter(items)
    return (0 until items.length()).mapNotNull { index ->
        val element = items.optJSONObject(index) ?: return@mapNotNull null
        val at = placed(element, "coordinate")

        MissionItem(
            index = index,
            sequence = element.optInt("sequence", index),
            latitude = at?.latitude ?: Double.NaN,
            longitude = at?.longitude ?: Double.NaN,
            command = element.optString("name"),
            kind = element.optString("kind"),
            commandId = element.optInt("command"),
            selected = element.optBoolean("selected"),
            altitude = element.optDouble("altitude", Double.NaN),
            altitudeText = element.optString("altitudeText"),
            specifiesCoordinate = element.optBoolean("specifiesCoordinate"),
            routed = element.optBoolean("flownLeg") && index <= endsAfter,
            afterRouteEnds = index > endsAfter,
            placed = at != null,
            exit = placed(element, "exitCoordinate")
                ?.takeIf { at == null || it.latitude != at.latitude || it.longitude != at.longitude },
        )
    }
}

fun missionItems(json: JSONObject?): List<MissionItem> = allMissionItems(json).filter { it.placed }

object PlanBridge {
    fun rawItems(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(PLAN_VIEW)) }.getOrNull()

    fun loadFromVehicle() = invokeOk("$PLAN_ROOT.loadFromVehicle")

    fun clearPlan() = invokeOk("$PLAN_ROOT.removeAll")

    fun sendToVehicle() = invokeOk("$PLAN_ROOT.sendToVehicle")

    fun setAltitude(index: Int, metres: Double): Boolean =
        setOk("$PLAN_ITEMS.$index.altitude", settingJson("$metres"))

    fun removeItem(index: Int): Boolean = removeMissionItem(index).ok

    fun selectSequence(sequence: Int): Boolean =
        invokeOk("$MISSION_CONTROLLER.setCurrentPlanViewSeqNum", "[$sequence, true]")

    fun moveItem(index: Int, latitude: Double, longitude: Double): Boolean =
        setOk("$PLAN_ITEMS.$index.coordinate", settingJson(coordinateJson(latitude, longitude)))
}

internal fun takeoffMissing(items: List<MissionItem>): Boolean =
    items.any { it.latitude != 0.0 || it.longitude != 0.0 } &&
        items.none { it.kind == "takeoff" }
