package one.aircast.mapspike

data class PlanBounds(
    val south: Double,
    val west: Double,
    val north: Double,
    val east: Double,
) {
    val centre: TrackPoint get() = TrackPoint((south + north) / 2, normaliseLongitude(west + longitudeSpan / 2))

    val longitudeSpan: Double get() = if (east >= west) east - west else east - west + 360.0

    val spanDegrees: Double get() = maxOf(north - south, longitudeSpan)
}

internal fun normaliseLongitude(degrees: Double): Double {
    val wrapped = (degrees + 180.0).mod(360.0) - 180.0
    return if (wrapped == -180.0) 180.0 else wrapped
}

internal fun longitudeArc(longitudes: List<Double>): Pair<Double, Double> {
    val sorted = longitudes.map(::normaliseLongitude).sorted()
    val gaps = sorted.indices.map { index ->
        val here = sorted[index]
        val next = if (index == sorted.lastIndex) sorted[0] + 360.0 else sorted[index + 1]
        next - here to index
    }
    val widest = gaps.maxByOrNull { it.first } ?: return 0.0 to 0.0
    val after = (widest.second + 1) % sorted.size
    return sorted[after] to sorted[widest.second]
}

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

fun fitPoints(plan: List<TrackPoint>, latitude: Double, longitude: Double): List<TrackPoint> =
    plan.ifEmpty {
        if (isPlottable(latitude, longitude)) listOf(TrackPoint(latitude, longitude)) else emptyList()
    }

fun planBounds(points: List<TrackPoint>): PlanBounds? {
    val usable = points.filter { isPlottable(it.latitude, it.longitude) }
    if (usable.isEmpty()) {
        return null
    }
    val (west, east) = longitudeArc(usable.map { it.longitude })
    return PlanBounds(
        south = usable.minOf { it.latitude },
        west = west,
        north = usable.maxOf { it.latitude },
        east = east,
    )
}
