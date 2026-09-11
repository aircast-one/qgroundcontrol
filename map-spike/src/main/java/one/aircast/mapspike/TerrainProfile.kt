package one.aircast.mapspike

import org.json.JSONObject
import kotlin.math.asin
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sin
import kotlin.math.sqrt

private const val EARTH_RADIUS_METRES = 6_371_000.0
private const val MIN_SPAN_METRES = 1.0

data class ProfilePoint(
    val distance: Double,
    val terrain: Double?,
    val planned: Double,
)

data class TerrainProfile(val points: List<ProfilePoint>) {
    val distance: Double get() = points.lastOrNull()?.distance ?: 0.0

    val lowest: Double
        get() = points.minOfOrNull { min(it.terrain ?: it.planned, it.planned) } ?: 0.0

    val highest: Double
        get() = points.maxOfOrNull { max(it.terrain ?: it.planned, it.planned) } ?: 0.0

    val hasTerrain: Boolean get() = points.count { it.terrain != null } >= 2

    val terrainCoverage: Double
        get() = distance.takeIf { it > 0.0 }?.let { total ->
            points.zipWithNext()
                .filter { (from, to) -> from.terrain != null && to.terrain != null }
                .sumOf { (from, to) -> to.distance - from.distance } / total
        } ?: 0.0

    val span: Double get() = (highest - lowest).coerceAtLeast(MIN_SPAN_METRES)

    val flat: Boolean get() = highest - lowest < MIN_SPAN_METRES

    val drawable: Boolean get() = points.size >= 2
}

fun metresBetween(from: TrackPoint, to: TrackPoint): Double {
    val fromLat = Math.toRadians(from.latitude)
    val toLat = Math.toRadians(to.latitude)
    val deltaLat = toLat - fromLat
    val deltaLon = Math.toRadians(to.longitude - from.longitude)

    val a = sin(deltaLat / 2).let { it * it } +
        cos(fromLat) * cos(toLat) * sin(deltaLon / 2).let { it * it }
    return 2 * EARTH_RADIUS_METRES * asin(min(1.0, sqrt(a)))
}

const val TERRAIN_VIEW = "view.terrainProfile"

fun terrainProfile(view: JSONObject?): TerrainProfile {
    val points = view?.optJSONArray("points") ?: return TerrainProfile(emptyList())
    return TerrainProfile(
        (0 until points.length()).mapNotNull { index ->
            val point = points.optJSONObject(index) ?: return@mapNotNull null
            val planned = point.optDouble("missionAltitude", Double.NaN)
            if (planned.isNaN()) return@mapNotNull null
            ProfilePoint(
                distance = point.optDouble("distance", 0.0),
                terrain = point.optDouble("terrainAltitude", Double.NaN).takeIf { !it.isNaN() },
                planned = planned,
            )
        },
    )
}
