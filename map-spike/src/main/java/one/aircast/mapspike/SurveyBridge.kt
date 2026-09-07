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

// A survey is a visual item that owns a survey area polygon. Reading the whole
// item list and picking those out avoids having to track which index we created.
fun surveys(json: JSONObject?): List<Survey> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        if (!element.optBoolean("isSurveyItem")) return@mapNotNull null

        val transects = points(element.optJSONArray("visualTransectPoints"))
        val area = points(element.optJSONObject("surveyAreaPolygon")?.optJSONArray("path"))
        if (transects.isEmpty() && area.isEmpty()) return@mapNotNull null

        Survey(index, area, transects, element.optInt("cameraShots"))
    }
}

object SurveyBridge {
    // The bridge lists a child object by name rather than nesting it, so the
    // survey's polygon needs its own read.
    fun surveys(): List<Survey> {
        val json = runCatching { JSONObject(QGCBridge.get(PLAN_ITEMS)) }.getOrNull()
        val elements = json?.optJSONArray("elements") ?: return emptyList()

        return (0 until elements.length()).mapNotNull { index ->
            val element = elements.optJSONObject(index) ?: return@mapNotNull null
            if (!element.optBoolean("isSurveyItem")) return@mapNotNull null

            val transects = points(element.optJSONArray("visualTransectPoints"))
            val area = points(polygonPath(index))
            if (transects.isEmpty() && area.isEmpty()) return@mapNotNull null

            Survey(index, area, transects, element.optInt("cameraShots"))
        }
    }

    private fun polygonPath(itemIndex: Int): JSONArray? =
        runCatching {
            JSONObject(QGCBridge.get("$PLAN_ITEMS.$itemIndex.surveyAreaPolygon"))
                .optJSONArray("path")
        }.getOrNull()

    fun surveyItemName(): String =
        runCatching {
            JSONObject(QGCBridge.get("$MISSION_CONTROLLER.surveyComplexItemName")).optString("value")
        }.getOrDefault("")

    // A survey arrives with an empty polygon and so draws nothing. Seeding a
    // square around the centre is what makes it generate transects at all.
    fun insertSurvey(latitude: Double, longitude: Double, halfSize: Double = 0.002): Boolean {
        if (!isPlottable(latitude, longitude)) {
            return false
        }
        val name = surveyItemName()
        if (name.isBlank()) {
            return false
        }
        val count = PlanBridge.rawItemCount()
        if (count <= 0) {
            return false
        }

        val args = "[\"$name\", ${coordinate(latitude, longitude)}, $count]"
        val inserted = runCatching {
            JSONObject(QGCBridge.invoke("$MISSION_CONTROLLER.insertComplexMissionItem", args))
                .optBoolean("ok")
        }.getOrDefault(false)
        if (!inserted) {
            return false
        }

        val index = PlanBridge.rawItemCount() - 1
        listOf(
            latitude + halfSize to longitude - halfSize,
            latitude + halfSize to longitude + halfSize,
            latitude - halfSize to longitude + halfSize,
            latitude - halfSize to longitude - halfSize,
        ).forEach { (cornerLat, cornerLon) -> appendAreaVertex(index, cornerLat, cornerLon) }
        return true
    }

    fun appendAreaVertex(itemIndex: Int, latitude: Double, longitude: Double): Boolean =
        invoke("$PLAN_ITEMS.$itemIndex.surveyAreaPolygon.appendVertex", "[${coordinate(latitude, longitude)}]")

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
