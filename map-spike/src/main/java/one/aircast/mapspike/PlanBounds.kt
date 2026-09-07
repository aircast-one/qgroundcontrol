package one.aircast.mapspike

data class PlanBounds(
    val south: Double,
    val west: Double,
    val north: Double,
    val east: Double,
) {
    val centre: TrackPoint get() = TrackPoint((south + north) / 2, (west + east) / 2)

    val spanDegrees: Double get() = maxOf(north - south, east - west)
}

// Everything the plan draws, so a fit frames the whole thing rather than the
// mission alone. A circle contributes its ring, not its centre, or a fit would
// cut the circle in half.
fun planPoints(
    items: List<MissionItem> = emptyList(),
    polygons: List<FencePolygon> = emptyList(),
    circles: List<FenceCircle> = emptyList(),
    rally: List<RallyPoint> = emptyList(),
    surveys: List<Survey> = emptyList(),
): List<TrackPoint> =
    items.map { TrackPoint(it.latitude, it.longitude) } +
        polygons.flatMap { it.vertices } +
        circlesAsPolygons(circles).flatMap { it.vertices } +
        rally.map { TrackPoint(it.latitude, it.longitude) } +
        surveys.flatMap { it.area + it.transects }

fun planBounds(points: List<TrackPoint>): PlanBounds? {
    val usable = points.filter { isPlottable(it.latitude, it.longitude) }
    if (usable.isEmpty()) {
        return null
    }
    return PlanBounds(
        south = usable.minOf { it.latitude },
        west = usable.minOf { it.longitude },
        north = usable.maxOf { it.latitude },
        east = usable.maxOf { it.longitude },
    )
}
