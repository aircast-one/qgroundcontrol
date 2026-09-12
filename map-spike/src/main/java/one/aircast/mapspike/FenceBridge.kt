package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val FENCE_ROOT = "$PLAN_ROOT.geoFenceController"
const val RALLY_ROOT = "$PLAN_ROOT.rallyPointController"

const val FENCE_POLYGONS = "$FENCE_ROOT.polygons"
const val FENCE_CIRCLES = "$FENCE_ROOT.circles"
const val RALLY_POINTS = "$RALLY_ROOT.points"
const val FENCES_VIEW = "view.fences"

data class FencePolygon(
    val index: Int,
    val inclusion: Boolean,
    val vertices: List<TrackPoint>,
    val midpoints: List<TrackPoint> = emptyList(),
)
data class FenceCircle(
    val index: Int,
    val inclusion: Boolean,
    val centre: TrackPoint,
    val radius: Double,
    val detailText: String = "",
)
data class RallyPoint(val index: Int, val latitude: Double, val longitude: Double)

private fun coordinate(json: JSONObject?): TrackPoint? {
    val latitude = json?.optDouble("latitude", Double.NaN) ?: return null
    val longitude = json.optDouble("longitude", Double.NaN)
    if (!isPlottable(latitude, longitude)) return null
    return TrackPoint(latitude, longitude)
}

private fun listed(json: JSONObject?, key: String): JSONArray? = json?.optJSONArray(key)

const val FENCE_POLYGON_MINIMUM = 3

internal fun cornerRemovable(polygon: FencePolygon?): Boolean =
    (polygon?.vertices?.size ?: 0) > FENCE_POLYGON_MINIMUM

private fun midpointsOf(polygon: Int): List<TrackPoint> {
    val view = runCatching {
        JSONObject(QGCBridge.get("view.polygon($FENCE_POLYGONS.$polygon)"))
    }.getOrNull() ?: return emptyList()
    val between = view.optJSONArray("midpoints") ?: return emptyList()
    return (0 until between.length()).mapNotNull { coordinate(between.optJSONObject(it)) }
}

fun fencePolygons(json: JSONObject?): List<FencePolygon> {
    val list = listed(json, "polygons") ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val corners = element.optJSONArray("vertices") ?: return@mapNotNull null
        val vertices = (0 until corners.length()).mapNotNull { coordinate(corners.optJSONObject(it)) }
        if (vertices.size < FENCE_POLYGON_MINIMUM) return@mapNotNull null
        val at = element.optInt("index", index)
        FencePolygon(at, element.optBoolean("inclusion", true), vertices, midpointsOf(at))
    }
}

fun fenceCircles(json: JSONObject?): List<FenceCircle> {
    val list = listed(json, "circles") ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val centre = coordinate(element.optJSONObject("centre")) ?: return@mapNotNull null
        val radius = element.optDouble("radius", Double.NaN)
        if (radius.isNaN() || radius <= 0.0) return@mapNotNull null
        FenceCircle(
            element.optInt("index", index),
            element.optBoolean("inclusion", true),
            centre,
            radius,
            element.optText("detailText"),
        )
    }
}

fun rallyPoints(json: JSONObject?): List<RallyPoint> {
    val list = listed(json, "rallyPoints") ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val point = coordinate(element) ?: return@mapNotNull null
        RallyPoint(element.optInt("index", index), point.latitude, point.longitude)
    }
}

object FenceBridge {
    fun read(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(FENCES_VIEW)) }.getOrNull()

    fun addInclusionPolygon(topLeft: TrackPoint, bottomRight: TrackPoint): Boolean =
        invokeOk(
            "$FENCE_ROOT.addInclusionPolygon",
            "[${coordinateJson(topLeft)}, ${coordinateJson(bottomRight)}]",
        )

    fun addInclusionCircle(topLeft: TrackPoint, bottomRight: TrackPoint): Boolean =
        invokeOk("$FENCE_ROOT.addInclusionCircle", "[${coordinateJson(topLeft)}, ${coordinateJson(bottomRight)}]")

    fun addRallyPoint(latitude: Double, longitude: Double): Boolean =
        invokeOk("$RALLY_ROOT.addPoint", "[${coordinateJson(latitude, longitude)}]")

    fun moveRallyPoint(index: Int, latitude: Double, longitude: Double): Boolean =
        setOk("$RALLY_POINTS.$index.coordinate", settingJson(coordinateJson(latitude, longitude)))

    fun moveCircle(index: Int, latitude: Double, longitude: Double): Boolean =
        setOk("$FENCE_CIRCLES.$index.center", settingJson(coordinateJson(latitude, longitude)))

    fun setCircleRadius(index: Int, metres: Double): Boolean =
        setOk("$FENCE_CIRCLES.$index.radius", settingJson("$metres"))

    fun deletePolygon(index: Int): Boolean = invokeOk("$FENCE_ROOT.deletePolygon", "[$index]")

    fun deleteCircle(index: Int): Boolean = invokeOk("$FENCE_ROOT.deleteCircle", "[$index]")

    fun removeRallyPoint(index: Int): Boolean =
        invokeOk("$RALLY_ROOT.removePoint", "[\"@$RALLY_POINTS.$index\"]")

    fun splitSegment(polygon: Int, segment: Int): Boolean =
        invokeOk("$FENCE_POLYGONS.$polygon.splitPolygonSegment", "[$segment]")

    fun removeVertex(polygon: Int, vertex: Int): Boolean =
        invokeOk("$FENCE_POLYGONS.$polygon.removeVertex", "[$vertex]")

    fun adjustVertex(polygon: Int, vertex: Int, latitude: Double, longitude: Double): Boolean =
        invokeOk(
            "$FENCE_POLYGONS.$polygon.adjustVertex",
            "[$vertex, ${coordinateJson(latitude, longitude)}]",
        )
}

private const val EARTH_RADIUS_M = 6_371_000.0
private const val CIRCLE_SEGMENTS = 48

fun circleRing(
    centre: TrackPoint,
    radiusMetres: Double,
    segments: Int = CIRCLE_SEGMENTS,
): List<TrackPoint> {
    if (radiusMetres <= 0.0 || segments < 3) {
        return emptyList()
    }

    val angular = radiusMetres / EARTH_RADIUS_M
    val lat = Math.toRadians(centre.latitude)
    val lon = Math.toRadians(centre.longitude)

    return (0 until segments).map { step ->
        val bearing = 2 * Math.PI * step / segments
        val pointLat = kotlin.math.asin(
            kotlin.math.sin(lat) * kotlin.math.cos(angular) +
                kotlin.math.cos(lat) * kotlin.math.sin(angular) * kotlin.math.cos(bearing),
        )
        val pointLon = lon + kotlin.math.atan2(
            kotlin.math.sin(bearing) * kotlin.math.sin(angular) * kotlin.math.cos(lat),
            kotlin.math.cos(angular) - kotlin.math.sin(lat) * kotlin.math.sin(pointLat),
        )
        TrackPoint(Math.toDegrees(pointLat), Math.toDegrees(pointLon))
    }
}

fun circlesAsPolygons(circles: List<FenceCircle>): List<FencePolygon> =
    circles.mapNotNull { circle ->
        val ring = circleRing(circle.centre, circle.radius)
        if (ring.size < 3) null else FencePolygon(circle.index, circle.inclusion, ring)
    }
