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

data class Clearance(
    val collides: Boolean,
    val metres: Double?,
    val text: String,
    val complete: Boolean,
)

data class ProfilePoint(
    val distance: Double,
    val terrain: Double?,
    val planned: Double,
)

data class TerrainProfile(
    val points: List<ProfilePoint>,
    val clearance: Clearance? = null,
    val lowestText: String = "",
    val highestText: String = "",
    val distanceText: String = "",
    val bandText: String = "",
) {
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

internal fun clearanceOf(view: JSONObject?): Clearance? {
    if (view == null) return null
    return Clearance(
        collides = view.optBoolean("hasCollision"),
        metres = view.optDouble("minClearanceMetres", Double.NaN).takeIf { !it.isNaN() },
        text = view.optText("clearanceText").takeIf { !view.isNull("clearanceText") }.orEmpty(),
        complete = view.optBoolean("clearanceComplete"),
    )
}

// A mission below the ground is below it whether or not every sample has ground
// under it; a mission that clears is only known to clear by as much as the
// smallest measured gap, which is not the smallest gap when samples are missing.
internal fun terrainWarning(clearance: Clearance?): String? {
    if (clearance == null) return null
    val metres = clearance.metres
    return when {
        clearance.collides || (metres != null && metres < 0.0) ->
            clearance.text.takeIf { it.isNotBlank() }
                ?.let { "The route goes $it below the ground." }
                ?: "The route goes below the ground."
        metres == null || !clearance.complete -> null
        else -> null
    }
}

fun terrainProfile(view: JSONObject?): TerrainProfile {
    val points = view?.optJSONArray("points") ?: return TerrainProfile(emptyList())
    return TerrainProfile(
        points = (0 until points.length()).mapNotNull { index ->
            val point = points.optJSONObject(index) ?: return@mapNotNull null
            val planned = point.optDouble("missionAltitude", Double.NaN)
            if (planned.isNaN()) return@mapNotNull null
            ProfilePoint(
                distance = point.optDouble("distance", 0.0),
                terrain = point.optDouble("terrainAltitude", Double.NaN).takeIf { !it.isNaN() },
                planned = planned,
            )
        },
        clearance = clearanceOf(view),
        lowestText = view.optText("lowestText"),
        highestText = view.optText("highestText"),
        distanceText = view.optText("distanceText"),
        bandText = view.optText("bandText"),
    )
}
