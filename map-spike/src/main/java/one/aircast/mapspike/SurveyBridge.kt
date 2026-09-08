package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val MISSION_CONTROLLER = "$PLAN_ROOT.missionController"

data class Survey(
    val index: Int,
    val area: List<TrackPoint>,
    val transects: List<TrackPoint>,
    val cameraShots: Int,
    val gridAngle: Double = Double.NaN,
)

private fun points(array: JSONArray?): List<TrackPoint> {
    if (array == null) return emptyList()
    return (0 until array.length()).mapNotNull { index ->
        val point = array.optJSONObject(index) ?: return@mapNotNull null
        val latitude = point.optDouble("latitude", Double.NaN)
        val longitude = point.optDouble("longitude", Double.NaN)
        if (isPlottable(latitude, longitude)) TrackPoint(latitude, longitude) else null
    }
}

fun surveys(json: JSONObject?): List<Survey> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        if (!element.optBoolean("isSurveyItem")) return@mapNotNull null

        val transects = points(element.optJSONArray("visualTransectPoints"))
        val area = points(element.optJSONObject("surveyAreaPolygon")?.optJSONArray("path"))
        if (transects.isEmpty() && area.isEmpty()) return@mapNotNull null

        Survey(
            index = index,
            area = area,
            transects = transects,
            cameraShots = element.optInt("cameraShots"),
            gridAngle = factValue(element, "GridAngle"),
        )
    }
}

const val GRID_STEP_DEGREES = 30.0

fun nextGridAngle(current: Double): Double =
    ((if (current.isNaN()) 0.0 else current) + GRID_STEP_DEGREES) % 360.0

object SurveyBridge {
    fun surveys(): List<Survey> = surveysFrom(PlanBridge.rawItems())

    fun surveysFrom(json: JSONObject?): List<Survey> {
        val elements = json?.optJSONArray("elements") ?: return emptyList()

        return (0 until elements.length()).mapNotNull { index ->
            val element = elements.optJSONObject(index) ?: return@mapNotNull null
            if (!element.optBoolean("isSurveyItem")) return@mapNotNull null

            val transects = points(element.optJSONArray("visualTransectPoints"))
            val area = points(polygonPath(index))
            if (transects.isEmpty() && area.isEmpty()) return@mapNotNull null

            Survey(
                index = index,
                area = area,
                transects = transects,
                cameraShots = element.optInt("cameraShots"),
                gridAngle = factValue(element, "GridAngle"),
            )
        }
    }

    private fun polygonPath(itemIndex: Int): JSONArray? =
        runCatching {
            JSONObject(QGCBridge.get("$PLAN_ITEMS.$itemIndex.surveyAreaPolygon"))
                .optJSONArray("path")
        }.getOrNull()

    private fun altitudePath(itemIndex: Int) =
        "$PLAN_ITEMS.$itemIndex.cameraCalc.distanceToSurface"

    fun altitude(itemIndex: Int): Double =
        runCatching {
            JSONObject(QGCBridge.get(altitudePath(itemIndex))).optDouble("value", Double.NaN)
        }.getOrDefault(Double.NaN)

    fun setAltitude(itemIndex: Int, metres: Double): Boolean =
        runCatching {
            JSONObject(QGCBridge.set(altitudePath(itemIndex), "{\"value\":$metres}"))
                .optBoolean("ok")
        }.getOrDefault(false)

    fun surveyItemName(): String =
        runCatching {
            JSONObject(QGCBridge.get("$MISSION_CONTROLLER.surveyComplexItemName")).optString("value")
        }.getOrDefault("")

    fun insertSurvey(latitude: Double, longitude: Double, halfSize: Double = 0.002): Boolean {
        if (!isPlottable(latitude, longitude)) {
            return false
        }
        val name = surveyItemName()
        if (name.isBlank()) {
            return false
        }
        val count = PlanBridge.rawItemCount()?.takeIf { it > 0 } ?: return false

        val args = "[\"$name\", ${coordinate(latitude, longitude)}, $count]"
        val inserted = insertedAnItem(
            runCatching {
                QGCBridge.invoke("$MISSION_CONTROLLER.insertComplexMissionItem", args)
            }.getOrDefault(""),
        )
        if (!inserted) {
            return false
        }

        val index = (PlanBridge.rawItemCount()?.takeIf { it > 0 } ?: return false) - 1
        return listOf(
            latitude + halfSize to longitude - halfSize,
            latitude + halfSize to longitude + halfSize,
            latitude - halfSize to longitude + halfSize,
            latitude - halfSize to longitude - halfSize,
        ).all { (cornerLat, cornerLon) -> appendAreaVertex(index, cornerLat, cornerLon) }
    }

    fun appendAreaVertex(itemIndex: Int, latitude: Double, longitude: Double): Boolean =
        invoke("$PLAN_ITEMS.$itemIndex.surveyAreaPolygon.appendVertex", "[${coordinate(latitude, longitude)}]")

    fun setGridAngle(itemIndex: Int, degrees: Double): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set("$PLAN_ITEMS.$itemIndex.gridAngle", "{\"value\":$degrees}"),
            ).optBoolean("ok")
        }.getOrDefault(false)

    fun adjustAreaVertex(itemIndex: Int, vertex: Int, latitude: Double, longitude: Double): Boolean =
        invoke(
            "$PLAN_ITEMS.$itemIndex.surveyAreaPolygon.adjustVertex",
            "[$vertex, ${coordinate(latitude, longitude)}]",
        )

    private fun coordinate(latitude: Double, longitude: Double) =
        "{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}"

    private fun invoke(path: String, args: String): Boolean =
        runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
}
