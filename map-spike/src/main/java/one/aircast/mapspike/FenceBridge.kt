package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val FENCE_ROOT = "$PLAN_ROOT.geoFenceController"
const val RALLY_ROOT = "$PLAN_ROOT.rallyPointController"

const val FENCE_POLYGONS = "$FENCE_ROOT.polygons"
const val FENCE_CIRCLES = "$FENCE_ROOT.circles"
const val RALLY_POINTS = "$RALLY_ROOT.points"

data class FencePolygon(val index: Int, val inclusion: Boolean, val vertices: List<TrackPoint>)
data class FenceCircle(val index: Int, val inclusion: Boolean, val centre: TrackPoint, val radius: Double)
data class RallyPoint(val index: Int, val latitude: Double, val longitude: Double)

private fun coordinate(json: JSONObject?): TrackPoint? {
    val latitude = json?.optDouble("latitude", Double.NaN) ?: return null
    val longitude = json.optDouble("longitude", Double.NaN)
    if (!isPlottable(latitude, longitude)) return null
    return TrackPoint(latitude, longitude)
}

private fun elements(json: JSONObject?): JSONArray? = json?.optJSONArray("elements")

// A polygon is only worth drawing once it closes, which needs three vertices.
fun fencePolygons(json: JSONObject?): List<FencePolygon> {
    val list = elements(json) ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val path = element.optJSONArray("path") ?: return@mapNotNull null
        val vertices = (0 until path.length()).mapNotNull { coordinate(path.optJSONObject(it)) }
        if (vertices.size < 3) return@mapNotNull null
        FencePolygon(index, element.optBoolean("inclusion", true), vertices)
    }
}

fun fenceCircles(json: JSONObject?): List<FenceCircle> {
    val list = elements(json) ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val centre = coordinate(element.optJSONObject("center")) ?: return@mapNotNull null
        val radius = element.optJSONObject("radius")?.optDouble("value", Double.NaN)
            ?: element.optDouble("radius", Double.NaN)
        if (radius.isNaN() || radius <= 0.0) return@mapNotNull null
        FenceCircle(index, element.optBoolean("inclusion", true), centre, radius)
    }
}

fun rallyPoints(json: JSONObject?): List<RallyPoint> {
    val list = elements(json) ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val point = coordinate(element.optJSONObject("coordinate")) ?: return@mapNotNull null
        RallyPoint(index, point.latitude, point.longitude)
    }
}

object FenceBridge {
    fun polygons(): List<FencePolygon> = fencePolygons(read(FENCE_POLYGONS))

    fun circles(): List<FenceCircle> = fenceCircles(read(FENCE_CIRCLES))

    fun rally(): List<RallyPoint> = rallyPoints(read(RALLY_POINTS))

    fun addInclusionPolygon(topLeft: TrackPoint, bottomRight: TrackPoint): Boolean =
        invoke(
            "$FENCE_ROOT.addInclusionPolygon",
            "[${point(topLeft)}, ${point(bottomRight)}]",
        )

    fun addRallyPoint(latitude: Double, longitude: Double): Boolean =
        invoke("$RALLY_ROOT.addPoint", "[{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}]")

    fun clearFences(): Boolean = invoke("$FENCE_ROOT.clearAllInteractive")

    fun adjustVertex(polygon: Int, vertex: Int, latitude: Double, longitude: Double): Boolean =
        invoke(
            "$FENCE_POLYGONS.$polygon.adjustVertex",
            "[$vertex, {\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}]",
        )

    private fun point(value: TrackPoint) =
        "{\"latitude\":${value.latitude},\"longitude\":${value.longitude},\"altitude\":0}"

    private fun read(path: String): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(path)) }.getOrNull()

    private fun invoke(path: String, args: String = "[]"): Boolean =
        runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
}
