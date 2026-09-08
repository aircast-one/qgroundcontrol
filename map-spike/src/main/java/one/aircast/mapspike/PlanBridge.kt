package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val PLAN_ROOT = "plan"
const val PLAN_ITEMS = "$PLAN_ROOT.missionController.visualItems"

// Several of QGroundControl's settings arrive as Facts in an object's fact
// list rather than as plain fields: a circle's radius, a survey's grid angle,
// an item's altitude.
fun factValue(element: JSONObject, name: String): Double {
    val facts = element.optJSONArray("facts") ?: return Double.NaN
    for (index in 0 until facts.length()) {
        val fact = facts.optJSONObject(index) ?: continue
        if (fact.optString("name").equals(name, ignoreCase = true)) {
            return fact.optDouble("value", Double.NaN)
        }
    }
    return Double.NaN
}

data class MissionItem(
    val index: Int,
    val sequence: Int,
    val latitude: Double,
    val longitude: Double,
    val command: String,
    val current: Boolean,
    val altitude: Double = Double.NaN,
)

// A mission item only sits on the map when it specifies a coordinate; takeoff
// and land items that inherit position do not, and neither does the settings
// item that always heads the list.
fun missionItems(json: JSONObject?): List<MissionItem> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        if (!element.optBoolean("specifiesCoordinate")) return@mapNotNull null

        val coordinate = element.optJSONObject("coordinate") ?: return@mapNotNull null
        val latitude = coordinate.optDouble("latitude", Double.NaN)
        val longitude = coordinate.optDouble("longitude", Double.NaN)
        if (!isPlottable(latitude, longitude)) return@mapNotNull null

        MissionItem(
            index = index,
            sequence = element.optInt("sequenceNumber", index),
            latitude = latitude,
            longitude = longitude,
            command = element.optString("commandName"),
            current = element.optBoolean("isCurrentItem"),
            altitude = factValue(element, "Altitude"),
        )
    }
}

object PlanBridge {
    // The item list feeds mission items, surveys and the terrain profile. Read
    // once and hand the same JSON to each, rather than three trips over the
    // bridge for the same data.
    fun rawItems(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(PLAN_ITEMS)) }.getOrNull()

    fun items(): List<MissionItem> = missionItems(rawItems())

    fun loadFromVehicle() = invoke("$PLAN_ROOT.loadFromVehicle")

    // The controller only. QGC's Clear Mission calls removeAllFromVehicle and
    // wipes the aircraft as well, behind a modal confirmation. Starting a plan
    // over is the local intent and does not need to touch the vehicle; Upload is
    // there for anyone who does want the empty plan flown.
    fun clearPlan() = invoke("$PLAN_ROOT.removeAll")

    fun sendToVehicle() = invoke("$PLAN_ROOT.sendToVehicle")


    // The insert index addresses the whole visual item list, which starts with a
    // settings item and can hold items that carry no coordinate. Counting only the
    // ones drawn on the map gives an index past the end, and MissionController
    // walks off the list rather than refusing it.
    // Null when the plan could not be read at all. Folding that into 0 made a
    // failed call indistinguishable from an empty plan, and they need opposite
    // responses: an empty plan is a thing to add to, a failed read is a thing to
    // stop on. A started controller always holds at least the settings item, so
    // a genuine 0 does not occur and a 0 was always a failure wearing a count.
    fun rawItemCount(): Int? =
        runCatching { JSONObject(QGCBridge.get(PLAN_ITEMS)).optJSONArray("elements")?.length() }
            .getOrNull()

    fun appendWaypoint(latitude: Double, longitude: Double): Boolean {
        val count = rawItemCount()?.takeIf { it > 0 } ?: return false
        return invoke(
            "$PLAN_ROOT.missionController.insertSimpleMissionItem",
            "[{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}, $count]",
        )
    }

    private fun insertAt(method: String, latitude: Double, longitude: Double): Int? {
        val count = rawItemCount()?.takeIf { it > 0 } ?: return null
        val placed = invoke(
            "$PLAN_ROOT.missionController.$method",
            "[{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}, $count]",
        )
        return if (placed) count else null
    }

    private fun writeCoordinate(path: String, latitude: Double, longitude: Double): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set(
                    path,
                    "{\"value\":{\"latitude\":$latitude,\"longitude\":$longitude}}",
                ),
            ).optBoolean("ok")
        }.getOrDefault(false)

    // insertTakeoffItem throws the coordinate away - the parameter is commented
    // out in MissionController - so the item arrives at 0,0 with
    // specifiesCoordinate false and readyForSaveMessage "Set its location". It
    // is not drawn, it is not counted, and the panel read "Empty plan" over a
    // plan that had a takeoff in it. Writing launchCoordinate is what places it,
    // and it sets the planned home the takeoff is measured from at the same time.
    fun appendTakeoff(latitude: Double, longitude: Double): Boolean =
        insertAt("insertTakeoffItem", latitude, longitude)?.let { index ->
            writeCoordinate("$PLAN_ITEMS.$index.launchCoordinate", latitude, longitude)
        } ?: false

    // insertLandItem passes its coordinate through, so it needs no such help.
    fun appendLanding(latitude: Double, longitude: Double): Boolean =
        insertAt("insertLandItem", latitude, longitude) != null

    fun setAltitude(index: Int, metres: Double): Boolean =
        runCatching {
            JSONObject(QGCBridge.set("$PLAN_ITEMS.$index.altitude", "{\"value\":$metres}"))
                .optBoolean("ok")
        }.getOrDefault(false)

    fun removeItem(index: Int): Boolean =
        invoke("$PLAN_ROOT.missionController.removeVisualItem", "[$index]")

    fun moveItem(index: Int, latitude: Double, longitude: Double): Boolean =
        runCatching {
            val value = "{\"value\":{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}}"
            JSONObject(QGCBridge.set("$PLAN_ITEMS.$index.coordinate", value)).optBoolean("ok")
        }.getOrDefault(false)

    private fun invoke(path: String, args: String = "[]"): Boolean =
        runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
}
