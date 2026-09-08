package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val PLAN_ROOT = "plan"
const val PLAN_ITEMS = "$PLAN_ROOT.missionController.visualItems"

// Not every mission item is drawable. A multirotor Land inserts an RTL, which
// has no coordinate of its own, and a multirotor Takeoff goes straight up from
// the launch point. Counting what is on the map reported "1 item" over a plan
// holding a takeoff and a return to launch. Element 0 is the settings item,
// which is a planned home rather than something the pilot added.
fun planItemCount(json: JSONObject?): Int =
    ((json?.optJSONArray("elements")?.length() ?: 0) - 1).coerceAtLeast(0)

// A takeoff and a return to launch are both in the plan and neither is on the
// map: both report coordinate 0,0, and the only position either has is the
// launch point, which the settings item already draws. Stacking markers there
// would be clutter rather than information, so the panel names them instead.
//
// isTakeoffItem and the numeric command, never commandName - that is a tr()
// string and matching it works until the app is localised.
const val MAV_CMD_NAV_RETURN_TO_LAUNCH = 20

// QGC links the planned home to the first item only when that item is a takeoff:
// its linkStartToHome, "Link back to home if first item is takeoff". Both the
// drawn path and the distance have to ask this the same way, or the map shows a
// leg the panel does not count.
fun linksStartToHome(json: JSONObject?): Boolean =
    json?.optJSONArray("elements")?.optJSONObject(1)?.optBoolean("isTakeoffItem") == true

fun planShape(json: JSONObject?): List<String> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()

    return (1 until elements.length())
        .mapNotNull { elements.optJSONObject(it) }
        .mapNotNull { element ->
            when {
                element.optBoolean("isTakeoffItem") -> "takeoff"
                element.optInt("command") == MAV_CMD_NAV_RETURN_TO_LAUNCH -> "RTL"
                else -> null
            }
        }
        .distinct()
}

// An insert that returns a null pointer still answers ok:true, because the
// method was found and did run. The item it hands back is the only evidence
// that anything was created, and a null one serialises as kind "null".
fun insertedItem(raw: String): Boolean =
    runCatching {
        val answer = JSONObject(raw)
        answer.optBoolean("ok") &&
            answer.optJSONObject("result")?.optString("kind") == "object"
    }.getOrDefault(false)

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
    // Where the aircraft leaves this item, which for a survey is the far corner
    // rather than the one it arrived at. Null when the item is a single point.
    val exit: TrackPoint? = null,
    // QGC's isStandaloneCoordinate, whose documentation is the whole rule:
    // "true: Waypoint line does not go through item". A region of interest, a
    // set-home or a land-start has a place on the map and is not somewhere the
    // aircraft flies to, so it is drawn and not routed through.
    val standalone: Boolean = false,
    // Everything past the landing. QGC stops both the line and the distance
    // there and so does this.
    val afterLanding: Boolean = false,
)

// "Don't draw segments immediately after a landing item", and "No need to add
// waypoint segments after an RTL". The aircraft is down; a leg onward is one
// nobody flies, and QGC leaves it out of missionTotalDistance too.
//
// The cut comes from the plan rather than from the items that reach the map,
// because the ending usually is not one of them: a multirotor Land inserts an
// RTL, which has no coordinate of its own and is never drawn.
//
// Two different tests, because QGC uses two. isLandCommand is a command-tree
// question the bridge answers for us. An RTL is not one of those - QGC finds it
// with `mavCommand() == MAV_CMD_NAV_RETURN_TO_LAUNCH` - which is why testing
// only the first left the route running straight past a Land.
fun routeEndsAfter(elements: JSONArray?): Int =
    (0 until (elements?.length() ?: 0))
        .firstOrNull { index ->
            elements?.optJSONObject(index)?.let { element ->
                element.optBoolean("isLandCommand") ||
                    element.optInt("command") == MAV_CMD_NAV_RETURN_TO_LAUNCH
            } == true
        }
        ?: Int.MAX_VALUE

fun missionItems(json: JSONObject?): List<MissionItem> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    val endsAfter = routeEndsAfter(elements)
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
            standalone = element.optBoolean("isStandaloneCoordinate"),
            afterLanding = index > endsAfter,
            exit = element.optJSONObject("exitCoordinate")?.let { at ->
                val exitLatitude = at.optDouble("latitude", Double.NaN)
                val exitLongitude = at.optDouble("longitude", Double.NaN)
                TrackPoint(exitLatitude, exitLongitude)
                    .takeIf {
                        isPlottable(exitLatitude, exitLongitude) &&
                            (exitLatitude != latitude || exitLongitude != longitude)
                    }
            },
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

    fun appendWaypoint(latitude: Double, longitude: Double): Boolean =
        insertAt("insertSimpleMissionItem", latitude, longitude) != null

    private fun insertAt(method: String, latitude: Double, longitude: Double): Int? {
        val count = rawItemCount()?.takeIf { it > 0 } ?: return null
        val raw = runCatching {
            QGCBridge.invoke(
                "$PLAN_ROOT.missionController.$method",
                "[{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}, $count]",
            )
        }.getOrDefault("")
        return if (insertedItem(raw)) count else null
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
    // The item is already in the plan by the time its location is written, so a
    // refused write would leave a takeoff behind while reporting that nothing
    // was added - and the panel counts items whether or not they can be drawn.
    fun appendTakeoff(latitude: Double, longitude: Double): Boolean {
        val index = insertAt("insertTakeoffItem", latitude, longitude) ?: return false
        if (writeCoordinate("$PLAN_ITEMS.$index.launchCoordinate", latitude, longitude)) {
            return true
        }
        removeItem(index)
        return false
    }

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
