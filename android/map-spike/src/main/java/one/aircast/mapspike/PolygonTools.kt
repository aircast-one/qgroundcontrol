package one.aircast.mapspike

import kotlin.math.cos
import kotlin.math.min

private const val DEFAULT_SHAPE_FRACTION = 0.75
private const val DEFAULT_SHAPE_MAX_METRES = 3000.0
private const val DEFAULT_CIRCLE_SEGMENTS = 16
private const val METRES_PER_DEGREE = 111_320.0

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

fun shapePath(fence: Int?, survey: Survey?): String? = when {
    fence != null -> "$FENCE_POLYGONS.$fence"
    survey != null -> "$PLAN_ITEMS.${survey.index}.${survey.property}"
    else -> null
}

fun replaceShape(path: String, vertices: List<TrackPoint>): Boolean =
    vertices.size >= 3 &&
        // qtpaths: plan.geoFenceController.polygons.0.clear, plan.missionController.visualItems.0.surveyAreaPolygon.clear
        invokeOk("$path.clear") &&
        // qtpaths: plan.geoFenceController.polygons.0.appendVertices, plan.missionController.visualItems.0.surveyAreaPolygon.appendVertices
        invokeOk("$path.appendVertices", vertices.joinToString(",", "[[", "]]") { coordinateJson(it) })
const val CORRIDOR_PROPERTY = "corridorPolyline"
