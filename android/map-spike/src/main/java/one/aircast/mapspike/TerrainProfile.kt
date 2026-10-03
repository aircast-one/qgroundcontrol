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
    val collision: Boolean = false,
)

data class ProfileMarker(
    val sequence: Int,
    val distance: Double,
    val label: String,
    val endDistance: Double? = null,
    val lastSequence: Int? = null,
    val pattern: String = "",
)

data class TerrainProfile(
    val points: List<ProfilePoint>,
    val clearance: Clearance? = null,
    val lowestText: String = "",
    val highestText: String = "",
    val distanceText: String = "",
    val bandText: String = "",
    val markers: List<ProfileMarker> = emptyList(),
    val band: Pair<Double, Double>? = null,
    val heightHeader: String = "",
    val distanceTicks: List<String> = emptyList(),
    val heightTicks: List<String> = emptyList(),
) {
    val distance: Double get() = points.lastOrNull()?.distance ?: 0.0

    val lowest: Double
        get() = band?.first ?: points.minOfOrNull { min(it.terrain ?: it.planned, it.planned) } ?: 0.0

    val highest: Double
        get() = band?.second ?: points.maxOfOrNull { max(it.terrain ?: it.planned, it.planned) } ?: 0.0

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
                collision = point.optBoolean("collision"),
            )
        },
        markers = view.optJSONArray("markers")?.let { list ->
            (0 until list.length()).mapNotNull { list.optJSONObject(it) }.map { marker ->
                val complex = marker.optJSONObject("complex")
                ProfileMarker(
                    sequence = marker.optInt("sequence"),
                    distance = marker.optDouble("distance", 0.0),
                    label = marker.optText("label"),
                    endDistance = complex?.optDouble("endDistance"),
                    lastSequence = complex?.optInt("lastSequence"),
                    pattern = complex?.optText("pattern").orEmpty(),
                )
            }
        }.orEmpty(),
        clearance = clearanceOf(view),
        lowestText = view.optText("lowestText"),
        highestText = view.optText("highestText"),
        distanceText = view.optText("distanceText"),
        bandText = view.optText("bandText"),
        band = view.optDouble("minAltitudeMeters", Double.NaN).takeIf { it.isFinite() }
            ?.let { low -> view.optDouble("maxAltitudeMeters", Double.NaN).takeIf { it.isFinite() }?.let { low to it } },
        heightHeader = view.optText("heightHeader"),
        distanceTicks = view.optJSONArray("distanceTicks")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
        heightTicks = view.optJSONArray("heightTicks")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
    )
}

fun collisionLegs(view: JSONObject?): List<Pair<TrackPoint, TrackPoint>> {
    val legs = view?.optJSONArray("collisionLegs") ?: return emptyList()
    fun spot(json: JSONObject?) = json?.let { TrackPoint(it.optDouble("latitude", Double.NaN), it.optDouble("longitude", Double.NaN)) }?.takeIf { isPlottable(it.latitude, it.longitude) }
    return (0 until legs.length()).mapNotNull { index ->
        val leg = legs.optJSONObject(index)
        spot(leg?.optJSONObject("from"))?.let { from -> spot(leg?.optJSONObject("to"))?.let { to -> from to to } }
    }
}
