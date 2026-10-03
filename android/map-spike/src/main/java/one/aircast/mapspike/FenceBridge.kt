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
const val FLY_FENCES_VIEW = "view.flyFences"

data class FencePolygon(
    val index: Int,
    val inclusion: Boolean,
    val vertices: List<TrackPoint>,
    val editable: EditableShape? = null,
    val kindText: String = "",
    val detailText: String = "",
)
data class FenceRow(val index: Int, val circle: Boolean, val title: String, val detail: String, val inclusion: Boolean? = null)

fun fenceRows(polygons: List<FencePolygon>, circles: List<FenceCircle>): List<FenceRow> =
    polygons.map { FenceRow(it.index, false, it.kindText.ifBlank { if (it.inclusion) "Keep-in polygon" else "Keep-out polygon" }, it.detailText, it.inclusion) } +
        circles.map { FenceRow(it.index, true, it.kindText.ifBlank { if (it.inclusion) "Keep-in circle" else "Keep-out circle" }, it.detailText) }

internal fun fenceRowHit(row: FenceRow): MapHit = if (row.circle) MapHit.Circle(row.index) else MapHit.FenceVertex(row.index, 0)

internal fun rowSelected(row: FenceRow, selected: MapHit?): Boolean = when (selected) {
    is MapHit.Circle -> row.circle && selected.index == row.index
    is MapHit.CircleCentre -> row.circle && selected.index == row.index
    is MapHit.FenceVertex -> !row.circle && selected.polygon == row.index
    else -> false
}

internal fun fenceSelectionAfterRemove(row: FenceRow, selected: MapHit?): MapHit? {
    fun moved(index: Int, rebuild: (Int) -> MapHit): MapHit? = when {
        index == row.index -> null
        index > row.index -> rebuild(index - 1)
        else -> selected
    }
    return when (selected) {
        is MapHit.FenceVertex -> if (row.circle) selected else moved(selected.polygon) { MapHit.FenceVertex(it, selected.vertex) }
        is MapHit.Circle -> if (!row.circle) selected else moved(selected.index) { MapHit.Circle(it) }
        is MapHit.CircleCentre -> if (!row.circle) selected else moved(selected.index) { MapHit.CircleCentre(it) }
        else -> selected
    }
}

internal fun rallyAfterRemove(removed: Int, countBefore: Int): MapHit? =
    (countBefore - 2).takeIf { it >= 0 }?.let { last -> MapHit.Rally(minOf(removed, last)) }

internal const val NO_GEOFENCE = "No geofence \u2013 keep the vehicle inside a boundary, or out of an area."
internal const val RALLY_HELP = "Rally Points provide alternate landing points when performing a Return to Launch (RTL)."

fun rallyRows(points: List<RallyPoint>): List<FenceRow> = points.map { point ->
    val height = point.altitude.takeIf { it.isFinite() }?.let { "${plainAltitude(it)} ${point.altitudeUnits.ifBlank { "m" }}" }
    val where = String.format(java.util.Locale.US, "%.6f, %.6f", point.latitude, point.longitude)
    FenceRow(point.index, false, "Rally point ${point.index + 1}", listOfNotNull(height, where).joinToString(" \u00b7 "))
}

private fun plainAltitude(value: Double): String =
    if (value == kotlin.math.floor(value)) value.toLong().toString() else String.format(java.util.Locale.US, "%.1f", value)

data class FenceCircle(
    val index: Int,
    val inclusion: Boolean,
    val centre: TrackPoint,
    val radius: Double,
    val detailText: String = "",
    val kindText: String = "",
    val radiusMinimum: Double? = null,
    val radiusMaximum: Double? = null,
    val radiusMetres: Double = radius,
    val radiusUnits: String = "",
)
data class RallyPoint(
    val index: Int,
    val latitude: Double,
    val longitude: Double,
    val altitudeMetres: Double = 0.0,
    val altitude: Double = Double.NaN,
    val altitudeUnits: String = "",
    val altitudePath: String = "",
)

internal fun coordinate(json: JSONObject?): TrackPoint? {
    val latitude = json?.optDouble("latitude", Double.NaN) ?: return null
    val longitude = json.optDouble("longitude", Double.NaN)
    if (!isPlottable(latitude, longitude)) return null
    return TrackPoint(latitude, longitude)
}

private fun listed(json: JSONObject?, key: String): JSONArray? = json?.optJSONArray(key)

const val FENCE_POLYGON_MINIMUM = 3

internal fun cornerRemovable(polygon: FencePolygon?): Boolean =
    polygon?.editable?.canRemoveVertex == true

fun fencePolygons(json: JSONObject?): List<FencePolygon> {
    val list = listed(json, "polygons") ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val corners = element.optJSONArray("vertices") ?: return@mapNotNull null
        val vertices = (0 until corners.length()).mapNotNull { coordinate(corners.optJSONObject(it)) }
        if (vertices.size < FENCE_POLYGON_MINIMUM) return@mapNotNull null
        val at = element.optInt("index", index)
        FencePolygon(
            at,
            element.optBoolean("inclusion", true),
            vertices,
            editableShape("$FENCE_POLYGONS.$at"),
            element.optText("kindText"),
            element.optText("detailText"),
        )
    }
}

data class FirmwareFence(
    val radiusMetres: Double,
    val radiusText: String,
    val centre: TrackPoint?,
)

fun firmwareFence(json: JSONObject?): FirmwareFence? {
    val served = json?.optJSONObject("firmwareFence") ?: return null
    val radius = served.optDouble("radiusMetres", Double.NaN)
    if (radius.isNaN() || radius <= 0.0) return null
    return FirmwareFence(radius, served.optText("radiusText"), coordinate(served.optJSONObject("centre")))
}

private fun JSONObject.bound(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { it.isFinite() && it > 0.0 }

internal const val CIRCLE_STEP = 1.5

internal fun grownRadius(circle: FenceCircle): Double? {
    val wanted = circle.radius * CIRCLE_STEP
    val ceiling = circle.radiusMaximum ?: return wanted
    return if (circle.radius >= ceiling) null else minOf(wanted, ceiling)
}

internal fun trimmedRadius(radius: Double): String =
    String.format(java.util.Locale.US, "%.1f", radius).trimEnd('0').trimEnd('.')

internal fun typedRadius(text: String, circle: FenceCircle): Double? =
    text.trim().toDoubleOrNull()
        ?.takeIf { it.isFinite() && it > 0.0 }
        ?.takeIf { wanted -> circle.radiusMinimum?.let { wanted >= it } ?: true }
        ?.takeIf { wanted -> circle.radiusMaximum?.let { wanted <= it } ?: true }

internal fun shrunkRadius(circle: FenceCircle): Double? {
    val wanted = circle.radius / CIRCLE_STEP
    val floor = circle.radiusMinimum ?: return wanted.takeIf { it > 0.0 }
    return if (circle.radius <= floor) null else maxOf(wanted, floor)
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
            element.optText("kindText"),
            element.bound("radiusMinimum"),
            element.bound("radiusMaximum"),
            element.optDouble("radiusMetres", radius).takeIf { it.isFinite() } ?: radius,
            element.optText("radiusUnits"),
        )
    }
}

fun rallyPoints(json: JSONObject?): List<RallyPoint> {
    val list = listed(json, "rallyPoints") ?: return emptyList()
    return (0 until list.length()).mapNotNull { index ->
        val element = list.optJSONObject(index) ?: return@mapNotNull null
        val point = coordinate(element) ?: return@mapNotNull null
        RallyPoint(
            element.optInt("index", index),
            point.latitude,
            point.longitude,
            element.optDouble("altitudeMetres", 0.0).takeIf { it.isFinite() } ?: 0.0,
            element.optDouble("altitude", Double.NaN),
            element.optText("altitudeUnits"),
            element.optText("altitudePath"),
        )
    }
}

fun rallyMovePayload(latitude: Double, longitude: Double, altitudeMetres: Double): String =
    settingJson(coordinateJson(latitude, longitude, altitudeMetres))

fun rallyAltitudeFor(rally: List<RallyPoint>, index: Int): Double =
    rally.firstOrNull { it.index == index }?.altitudeMetres ?: 0.0

const val BREACH_RETURN_PATH = "$FENCE_ROOT.breachReturnPoint"

data class BreachReturn(val point: TrackPoint, val altitude: Double?, val units: String, val altitudePath: String)

fun breachReturn(json: JSONObject?): BreachReturn? {
    val point = coordinate(json?.optJSONObject("breachReturnPoint")) ?: return null
    val altitude = json?.optJSONObject("breachReturnAltitude")
    return BreachReturn(
        point = point,
        altitude = altitude?.takeIf { !it.isNull("value") }?.optDouble("value")?.takeIf { it.isFinite() },
        units = altitude?.optText("units").orEmpty(),
        altitudePath = altitude?.optText("path").orEmpty(),
    )
}

object FenceBridge {
    fun read(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(FENCES_VIEW)) }.getOrNull()

    fun readFlown(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(FLY_FENCES_VIEW)) }.getOrNull()

    fun addInclusionPolygon(topLeft: TrackPoint, bottomRight: TrackPoint): Boolean =
        invokeOk(
            "$FENCE_ROOT.addInclusionPolygon",
            "[${coordinateJson(topLeft)}, ${coordinateJson(bottomRight)}]",
        )

    fun addInclusionCircle(topLeft: TrackPoint, bottomRight: TrackPoint): Boolean =
        invokeOk("$FENCE_ROOT.addInclusionCircle", "[${coordinateJson(topLeft)}, ${coordinateJson(bottomRight)}]")

    fun addRallyPoint(latitude: Double, longitude: Double): Boolean =
        invokeOk("$RALLY_ROOT.addPoint", "[${coordinateJson(latitude, longitude)}]")

    // RallyPoint::setCoordinate writes coordinate.altitude() straight into _altitudeFact, unlike
    // SimpleMissionItem::setCoordinate which takes lat/lon only. QGC's own drag never sends a bare
    // coordinate: MissionItemIndicatorDrag.qml:57 copies the existing altitude on first.
    fun moveRallyPoint(index: Int, latitude: Double, longitude: Double, altitudeMetres: Double): Boolean =
        setOk("$RALLY_POINTS.$index.coordinate", rallyMovePayload(latitude, longitude, altitudeMetres))

    fun moveCircle(index: Int, latitude: Double, longitude: Double): Boolean =
        setOk("$FENCE_CIRCLES.$index.center", settingJson(coordinateJson(latitude, longitude)))

    fun setCircleRadius(index: Int, shown: Double): Boolean =
        setOk("$FENCE_CIRCLES.$index.radius", settingJson("$shown"))

    fun deletePolygon(index: Int): Boolean = invokeOk("$FENCE_ROOT.deletePolygon", "[$index]")

    fun setBreachReturn(at: TrackPoint): Boolean = setOk(BREACH_RETURN_PATH, settingJson(coordinateJson(at)))

    fun clearBreachReturn(): Boolean = setOk(BREACH_RETURN_PATH, settingJson("null"))

    fun setBreachAltitude(path: String, shown: Double): Boolean = setOk(path, settingJson("$shown"))

    fun deleteCircle(index: Int): Boolean = invokeOk("$FENCE_ROOT.deleteCircle", "[$index]")

    fun setRallyAltitude(path: String, shown: Double): Boolean =
        setOk(path, settingJson("$shown"))

    fun removeRallyPoint(index: Int): Boolean =
        invokeOk("$RALLY_ROOT.removePoint", "[\"@$RALLY_POINTS.$index\"]")

    // QGC offers addInclusionPolygon and addInclusionCircle and no exclusion
    // counterparts, so a keep-out zone is reached by flipping the property
    // rather than by a different call.
    fun setPolygonInclusion(index: Int, inclusion: Boolean): Boolean =
        setOk("$FENCE_POLYGONS.$index.inclusion", settingJson(inclusion.toString()))

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

    return (0 until segments).map { step -> pointAt(centre, radiusMetres, 360.0 * step / segments) }
}

fun azimuthBetween(from: TrackPoint, to: TrackPoint): Double {
    val fromLat = Math.toRadians(from.latitude)
    val toLat = Math.toRadians(to.latitude)
    val deltaLon = Math.toRadians(to.longitude - from.longitude)
    val y = kotlin.math.sin(deltaLon) * kotlin.math.cos(toLat)
    val x = kotlin.math.cos(fromLat) * kotlin.math.sin(toLat) - kotlin.math.sin(fromLat) * kotlin.math.cos(toLat) * kotlin.math.cos(deltaLon)
    return (Math.toDegrees(kotlin.math.atan2(y, x)) + 360) % 360
}

fun pointAt(centre: TrackPoint, metres: Double, bearingDegrees: Double): TrackPoint {
    val angular = metres / EARTH_RADIUS_M
    val lat = Math.toRadians(centre.latitude)
    val lon = Math.toRadians(centre.longitude)
    val bearing = Math.toRadians(bearingDegrees)
    val pointLat = kotlin.math.asin(
        kotlin.math.sin(lat) * kotlin.math.cos(angular) +
            kotlin.math.cos(lat) * kotlin.math.sin(angular) * kotlin.math.cos(bearing),
    )
    val pointLon = lon + kotlin.math.atan2(
        kotlin.math.sin(bearing) * kotlin.math.sin(angular) * kotlin.math.cos(lat),
        kotlin.math.cos(angular) - kotlin.math.sin(lat) * kotlin.math.sin(pointLat),
    )
    return TrackPoint(Math.toDegrees(pointLat), Math.toDegrees(pointLon))
}

fun circlesAsPolygons(circles: List<FenceCircle>): List<FencePolygon> =
    circles.mapNotNull { circle ->
        val ring = circleRing(circle.centre, circle.radiusMetres)
        if (ring.size < 3) {
            null
        } else {
            FencePolygon(circle.index, circle.inclusion, ring, kindText = circle.kindText)
        }
    }
