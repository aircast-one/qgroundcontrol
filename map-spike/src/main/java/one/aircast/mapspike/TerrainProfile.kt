package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge
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

    // Ground height comes from a terrain server and is often unknown. The
    // planned altitude alone is still worth showing, so only that is required.
    val hasTerrain: Boolean get() = points.count { it.terrain != null } >= 2

    // A survey flies at one altitude, so with no terrain under it the range is a
    // single value. That is a flat profile, not an absent one, and refusing to
    // draw it told the pilot to add altitudes they had already set.
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

// An item with no planned altitude is dropped rather than drawn at zero, which
// would read as a dive to sea level. Unknown ground height is carried as null.
fun terrainProfile(json: JSONObject?): TerrainProfile {
    val elements = json?.optJSONArray("elements") ?: return TerrainProfile(emptyList())

    var previous: TrackPoint? = null
    var travelled = 0.0
    val points = mutableListOf<ProfilePoint>()

    for (index in 0 until elements.length()) {
        val element = elements.optJSONObject(index) ?: continue
        if (!element.optBoolean("specifiesCoordinate")) continue

        val coordinate = element.optJSONObject("coordinate") ?: continue
        val latitude = coordinate.optDouble("latitude", Double.NaN)
        val longitude = coordinate.optDouble("longitude", Double.NaN)
        if (!isPlottable(latitude, longitude)) continue

        val terrain = element.optDouble("terrainAltitude", Double.NaN)
        val planned = element.optDouble("amslEntryAlt", Double.NaN)
        val here = TrackPoint(latitude, longitude)

        previous?.let { travelled += metresBetween(it, here) }
        previous = here

        if (planned.isNaN()) continue
        points.add(ProfilePoint(travelled, terrain.takeIf { !it.isNaN() }, planned))
    }

    return TerrainProfile(points)
}

object TerrainBridge {
    fun profile(): TerrainProfile = terrainProfile(PlanBridge.rawItems())
}
