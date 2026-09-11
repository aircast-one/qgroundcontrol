package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val MISSION_CONTROLLER = "$PLAN_ROOT.missionController"

const val SHAPE_AREA = "area"
const val SHAPE_LINE = "line"

data class Survey(
    val index: Int,
    val area: List<TrackPoint>,
    val transects: List<TrackPoint>,
    val cameraShots: Int,
    val kind: String,
    val shape: String,
    val property: String,
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

const val GRID_STEP_DEGREES = 30.0

fun nextGridAngle(current: Double): Double =
    ((if (current.isNaN()) 0.0 else current) + GRID_STEP_DEGREES) % 360.0

object SurveyBridge {
    fun surveysFrom(json: JSONObject?): List<Survey> {
        val items = planItems(json) ?: return emptyList()

        return (0 until items.length()).mapNotNull { index ->
            val element = items.optJSONObject(index) ?: return@mapNotNull null
            val geometry = element.optJSONObject("geometry") ?: return@mapNotNull null
            val shape = geometry.optString("shape")
            val property = geometry.optString("property")
            if (shape.isBlank() || property.isBlank()) return@mapNotNull null

            val area = points(geometry.optJSONArray("vertices"))
            val transects = points(geometry.optJSONArray("transects"))
            if (transects.isEmpty() && area.isEmpty()) return@mapNotNull null

            Survey(
                index = index,
                area = area,
                transects = transects,
                cameraShots = element.optInt("cameraShots"),
                kind = element.optString("kind"),
                shape = shape,
                property = property,
            )
        }
    }

    fun rotateGrid(itemIndex: Int): Boolean {
        val current = runCatching {
            JSONObject(QGCBridge.get("$PLAN_ITEMS.$itemIndex.gridAngle")).optDouble("value", Double.NaN)
        }.getOrDefault(Double.NaN)
        return setGridAngle(itemIndex, nextGridAngle(current))
    }

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

    fun setGridAngle(itemIndex: Int, degrees: Double): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set("$PLAN_ITEMS.$itemIndex.gridAngle", "{\"value\":$degrees}"),
            ).optBoolean("ok")
        }.getOrDefault(false)

    fun adjustAreaVertex(survey: Survey, vertex: Int, latitude: Double, longitude: Double): Boolean =
        invoke(
            "$PLAN_ITEMS.${survey.index}.${survey.property}.adjustVertex",
            "[$vertex, ${coordinate(latitude, longitude)}]",
        )

    private fun coordinate(latitude: Double, longitude: Double) =
        "{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}"

    private fun invoke(path: String, args: String): Boolean =
        runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
}
