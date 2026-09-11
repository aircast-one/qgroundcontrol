package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val PLAN_ROOT = "plan"
const val PLAN_ITEMS = "$PLAN_ROOT.missionController.visualItems"

fun planItemCount(json: JSONObject?): Int =
    ((json?.optJSONArray("elements")?.length() ?: 0) - 1).coerceAtLeast(0)

const val MAV_CMD_NAV_RETURN_TO_LAUNCH = 20

fun linksStartToHome(json: JSONObject?): Boolean =
    json?.optJSONArray("elements")?.optJSONObject(1)?.optBoolean("isTakeoffItem") == true

fun complexIndices(json: JSONObject?): List<Int> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()

    return (1 until elements.length()).filter { index ->
        elements.optJSONObject(index)
            ?.optDouble("complexDistance", 0.0)
            ?.let { it > 0.0 && !it.isNaN() } == true
    }
}

fun complexKey(json: JSONObject?, index: Int): String {
    val element = json?.optJSONArray("elements")?.optJSONObject(index) ?: return ""
    val coordinate = element.optJSONObject("coordinate")

    return listOf(
        coordinate?.optDouble("latitude", Double.NaN),
        coordinate?.optDouble("longitude", Double.NaN),
        element.optDouble("complexDistance", 0.0),
    ).joinToString(",")
}

fun planShape(json: JSONObject?): List<String> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    val endsAfter = routeEndsAfter(elements)

    val named = (1 until elements.length())
        .mapNotNull { index -> elements.optJSONObject(index) }
        .mapNotNull { element ->
            when {
                element.optBoolean("isTakeoffItem") -> "takeoff"
                element.optInt("command") == MAV_CMD_NAV_RETURN_TO_LAUNCH -> "RTL"
                else -> null
            }
        }
        .distinct()

    val stranded = (1 until elements.length()).count { it > endsAfter }

    return named + listOfNotNull(
        stranded.takeIf { it > 0 }?.let { "$it after the landing" },
    )
}

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
    val exit: TrackPoint? = null,
    val routed: Boolean = true,
)

fun isFlownLeg(element: JSONObject?): Boolean =
    element != null &&
        element.optBoolean("specifiesCoordinate") &&
        !element.optBoolean("isStandaloneCoordinate") &&
        !element.optBoolean("isIncomplete")

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
            routed = isFlownLeg(element) && index <= endsAfter,
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
    fun rawItems(): JSONObject? =
        runCatching { JSONObject(QGCBridge.getFields(PLAN_ITEMS, "*")) }.getOrNull()

    fun loadFromVehicle() = invoke("$PLAN_ROOT.loadFromVehicle")

    fun clearPlan() = invoke("$PLAN_ROOT.removeAll")

    fun sendToVehicle() = invoke("$PLAN_ROOT.sendToVehicle")

    fun setAltitude(index: Int, metres: Double): Boolean =
        runCatching {
            JSONObject(QGCBridge.set("$PLAN_ITEMS.$index.altitude", "{\"value\":$metres}"))
                .optBoolean("ok")
        }.getOrDefault(false)

    fun removeItem(index: Int): Boolean = removeMissionItem(index).ok

    fun moveItem(index: Int, latitude: Double, longitude: Double): Boolean =
        runCatching {
            val value = "{\"value\":{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}}"
            JSONObject(QGCBridge.set("$PLAN_ITEMS.$index.coordinate", value)).optBoolean("ok")
        }.getOrDefault(false)

    private fun invoke(path: String, args: String = "[]"): Boolean =
        runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
}

internal fun takeoffMissing(items: List<MissionItem>): Boolean =
    items.any { it.latitude != 0.0 || it.longitude != 0.0 } &&
        items.none { it.command.contains("TAKEOFF", ignoreCase = true) }
