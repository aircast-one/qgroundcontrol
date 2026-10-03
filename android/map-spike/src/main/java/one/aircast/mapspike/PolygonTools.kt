package one.aircast.mapspike

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.MultiPoint
import org.maplibre.geojson.Point
import org.mavlink.qgroundcontrol.QGCBridge
import java.io.File
import kotlin.math.cos
import kotlin.math.min

private const val DEFAULT_SHAPE_FRACTION = 0.75
private const val DEFAULT_SHAPE_MAX_METRES = 3000.0
private const val DEFAULT_CIRCLE_SEGMENTS = 16
private const val METRES_PER_DEGREE = 111_320.0
private const val TRACE_SOURCE = "aircast-polygon-trace"
private const val TRACE_LINE_LAYER = "aircast-polygon-trace-line"
private const val TRACE_DOT_LAYER = "aircast-polygon-trace-dots"
private const val TRACE_COLOUR = "#FFFFFF"

private fun offset(centre: TrackPoint, northMetres: Double, eastMetres: Double) = TrackPoint(
    centre.latitude + northMetres / METRES_PER_DEGREE,
    centre.longitude + eastMetres / (METRES_PER_DEGREE * cos(Math.toRadians(centre.latitude))),
)

private fun halfExtents(view: List<TrackPoint>): Pair<Double, Double>? =
    view.takeIf { it.size == 4 }?.let { (topLeft, topRight, _, bottomLeft) ->
        fun half(metres: Double) = min(metres * DEFAULT_SHAPE_FRACTION, DEFAULT_SHAPE_MAX_METRES) / 2
        half(metresBetween(topLeft, topRight)) to half(metresBetween(topLeft, bottomLeft))
    }

private fun viewCentre(view: List<TrackPoint>) =
    TrackPoint(view.sumOf { it.latitude } / view.size, view.sumOf { it.longitude } / view.size)

fun defaultRectangle(view: List<TrackPoint>): List<TrackPoint> =
    halfExtents(view)?.let { (halfWidth, halfHeight) ->
        val centre = viewCentre(view)
        listOf(halfHeight to -halfWidth, halfHeight to halfWidth, -halfHeight to halfWidth, -halfHeight to -halfWidth)
            .map { (north, east) -> offset(centre, north, east) }
    } ?: emptyList()

fun defaultCircle(view: List<TrackPoint>): List<TrackPoint> =
    halfExtents(view)?.let { (halfWidth, halfHeight) ->
        circleRing(viewCentre(view), min(halfWidth, halfHeight), DEFAULT_CIRCLE_SEGMENTS)
    } ?: emptyList()

data class ShapeTarget(val path: String, val line: Boolean) {
    val minimum: Int get() = if (line) 2 else 3
    val fileShape: String get() = if (line) "polyline" else "polygon"
    val missing: String get() = if (line) "No polylines found in file" else "No polygons found in file"
}

fun shapeTarget(fence: Int?, survey: Survey?): ShapeTarget? = when {
    fence != null -> ShapeTarget("$FENCE_POLYGONS.$fence", line = false)
    survey != null -> ShapeTarget("$PLAN_ITEMS.${survey.index}.${survey.property}", line = survey.property == CORRIDOR_PROPERTY)
    else -> null
}

fun defaultLine(view: List<TrackPoint>): List<TrackPoint> =
    view.takeIf { it.size == 4 }?.let { (topLeft, topRight, bottomRight, bottomLeft) ->
        val top = TrackPoint((topLeft.latitude + topRight.latitude) / 2, (topLeft.longitude + topRight.longitude) / 2)
        val bottom = TrackPoint((bottomLeft.latitude + bottomRight.latitude) / 2, (bottomLeft.longitude + bottomRight.longitude) / 2)
        listOf(0.25, 0.75).map { fraction ->
            TrackPoint(top.latitude + (bottom.latitude - top.latitude) * fraction, top.longitude + (bottom.longitude - top.longitude) * fraction)
        }
    } ?: emptyList()

fun polygonCentre(vertices: List<TrackPoint>): TrackPoint? {
    if (vertices.size < 3) return null
    val origin = vertices.first()
    val local = vertices.map { it.longitude - origin.longitude to it.latitude - origin.latitude }
    val edges = local.indices.map { i -> local[i] to local[(i + 1) % local.size] }
    val cross = edges.map { (a, b) -> a.first * b.second - b.first * a.second }
    val area = cross.sum() / 2
    if (area == 0.0) return null
    val x = edges.zip(cross).sumOf { (edge, c) -> (edge.first.first + edge.second.first) * c } / (6 * area)
    val y = edges.zip(cross).sumOf { (edge, c) -> (edge.first.second + edge.second.second) * c } / (6 * area)
    return TrackPoint(origin.latitude + y, origin.longitude + x)
}

fun shapeMovedTo(vertices: List<TrackPoint>, centre: TrackPoint): List<TrackPoint>? {
    val from = polygonCentre(vertices) ?: return null
    val distance = metresBetween(from, centre)
    val azimuth = azimuthBetween(from, centre)
    return vertices.map { pointAt(it, distance, azimuth) }
}

fun fencePath(index: Int) = "$FENCE_POLYGONS.$index"

fun surveyPath(survey: Survey) = "$PLAN_ITEMS.${survey.index}.${survey.property}"

fun circleRadius(vertices: List<TrackPoint>): Double? =
    polygonCentre(vertices)?.let { centre -> vertices.firstOrNull()?.let { metresBetween(centre, it) } }

fun circleAround(vertices: List<TrackPoint>, radius: Double): List<TrackPoint>? =
    polygonCentre(vertices)?.takeIf { radius > 0 }?.let { circleRing(it, radius, DEFAULT_CIRCLE_SEGMENTS) }

fun shapeVertices(target: ShapeTarget, fences: List<FencePolygon>, surveys: List<Survey>): List<TrackPoint> =
    fences.firstOrNull { fencePath(it.index) == target.path }?.vertices
        ?: surveys.firstOrNull { surveyPath(it) == target.path }?.area
        ?: emptyList()

private const val CIRCLE_TOLERANCE = 0.01

fun isCircleShape(vertices: List<TrackPoint>): Boolean {
    val centre = polygonCentre(vertices) ?: return false
    val radius = circleRadius(vertices) ?: return false
    return vertices.size == DEFAULT_CIRCLE_SEGMENTS && radius > 0 &&
        vertices.all { kotlin.math.abs(metresBetween(centre, it) - radius) <= radius * CIRCLE_TOLERANCE }
}

fun liveCircles(circled: Set<String>, fences: List<FencePolygon>, surveys: List<Survey>): Set<String> =
    circled.filter { path -> isCircleShape(shapeVertices(ShapeTarget(path, line = false), fences, surveys)) }.toSet()

fun replaceShape(target: ShapeTarget, vertices: List<TrackPoint>): Boolean =
    vertices.size >= target.minimum &&
        // qtpaths: plan.geoFenceController.polygons.0.clear, plan.missionController.visualItems.0.surveyAreaPolygon.clear, plan.missionController.visualItems.0.corridorPolyline.clear
        invokeOk("${target.path}.clear") &&
        // qtpaths: plan.geoFenceController.polygons.0.appendVertices, plan.missionController.visualItems.0.surveyAreaPolygon.appendVertices, plan.missionController.visualItems.0.corridorPolyline.appendVertices
        invokeOk("${target.path}.appendVertices", vertices.joinToString(",", "[[", "]]") { coordinateJson(it) })

const val CORRIDOR_PROPERTY = "corridorPolyline"

fun fileShape(view: JSONObject?, target: ShapeTarget): Pair<List<TrackPoint>, String> = when {
    view == null -> emptyList<TrackPoint>() to target.missing
    view.optText("error").isNotBlank() -> emptyList<TrackPoint>() to view.optText("error")
    view.optText("shape") != target.fileShape -> emptyList<TrackPoint>() to target.missing
    else -> view.optJSONArray("points").let { points ->
        (0 until (points?.length() ?: 0)).mapNotNull { index ->
            points?.optJSONObject(index)?.let { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
        } to ""
    }
}

private fun fileName(context: Context, uri: Uri): String = runCatching {
    context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
        ?.use { if (it.moveToFirst()) it.getString(0) else null }
}.getOrNull().orEmpty()

internal fun extensionOf(name: String): String = name.substringAfterLast('.', "").lowercase()

internal fun mainShapeExtension(names: List<String>): String? {
    val extensions = names.map(::extensionOf)
    return listOf("shp", "kml").firstOrNull { it in extensions } ?: extensions.firstOrNull()
}

fun importShapeFiles(context: Context, uris: List<Uri>, target: ShapeTarget): String? {
    val named = uris.map { it to extensionOf(fileName(context, it)) }
    val extension = mainShapeExtension(named.map { "x.${it.second}" }) ?: return "That file could not be read."
    context.cacheDir.listFiles { file -> file.name.startsWith("shape.") }?.forEach { it.delete() }
    val copied = named.all { (uri, ext) ->
        runCatching {
            context.contentResolver.openInputStream(uri)?.use { source -> File(context.cacheDir, "shape.$ext").outputStream().use { source.copyTo(it) } } != null
        }.getOrDefault(false)
    }
    val staged = File(context.cacheDir, "shape.$extension")
    val view = "view.${if (target.line) "lineFile" else "areaFile"}(${staged.absolutePath})"
    val (vertices, error) = when {
        !copied -> emptyList<TrackPoint>() to "That file could not be read."
        else -> fileShape(runCatching { JSONObject(QGCBridge.get(view)) }.getOrNull(), target)
    }
    return when {
        error.isNotBlank() -> error
        replaceShape(target, vertices) -> null
        else -> target.missing
    }
}

fun traceOutline(points: List<TrackPoint>, line: Boolean = false): List<TrackPoint> =
    if (!line && points.size >= 3) points + points.first() else points

fun installTraceLayer(style: Style) {
    if (style.getSource(TRACE_SOURCE) != null) return
    style.addSource(GeoJsonSource(TRACE_SOURCE))
    style.addLayer(
        LineLayer(TRACE_LINE_LAYER, TRACE_SOURCE).withProperties(
            PropertyFactory.lineColor(TRACE_COLOUR),
            PropertyFactory.lineWidth(2f),
        ),
    )
    style.addLayer(
        CircleLayer(TRACE_DOT_LAYER, TRACE_SOURCE).withProperties(
            PropertyFactory.circleColor(TRACE_COLOUR),
            PropertyFactory.circleRadius(4f),
        ),
    )
}

fun renderTrace(style: Style, points: List<TrackPoint>, line: Boolean) {
    val lngLats = traceOutline(points, line).map { Point.fromLngLat(it.longitude, it.latitude) }
    val features = listOfNotNull(
        lngLats.takeIf { it.size >= 2 }?.let { Feature.fromGeometry(LineString.fromLngLats(it)) },
        lngLats.takeIf { it.isNotEmpty() }?.let { Feature.fromGeometry(MultiPoint.fromLngLats(it)) },
    )
    (style.getSource(TRACE_SOURCE) as? GeoJsonSource)?.setGeoJson(FeatureCollection.fromFeatures(features))
}
