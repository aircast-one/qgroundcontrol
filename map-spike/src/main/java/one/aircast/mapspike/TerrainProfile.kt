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

private data class Walk(
    val at: TrackPoint?,
    val travelled: Double,
    val points: List<ProfilePoint>,
)

private fun point(json: JSONObject, name: String): TrackPoint? {
    val coordinate = json.optJSONObject(name) ?: return null
    val latitude = coordinate.optDouble("latitude", Double.NaN)
    val longitude = coordinate.optDouble("longitude", Double.NaN)
    return if (isPlottable(latitude, longitude)) TrackPoint(latitude, longitude) else null
}

// An item with no planned altitude is dropped rather than drawn at zero, which
// would read as a dive to sea level. Unknown ground height is carried as null.
//
// A survey is one item holding a whole flight. Reading only its entry
// coordinate charted the hop out to it and called that the mission: 0.25 km
// against the 6.43 km the plan actually flies. complexDistance is how far the
// item itself covers, so it is added to the distance travelled and closed off
// at the exit altitude.
//
// Item 0 is the mission settings item, which QGC identifies by that position
// too. It is the planned home position, not a leg that gets flown, and
// MissionController leaves it out of missionTotalDistance for that reason.
// Counting it put a different distance in the profile than in the status line
// above it, and drew the plan diving from a sea-level launch it never has.
fun terrainProfile(json: JSONObject?): TerrainProfile {
    val elements = json?.optJSONArray("elements") ?: return TerrainProfile(emptyList())

    return TerrainProfile(
        (1 until elements.length())
            .mapNotNull { elements.optJSONObject(it) }
            .filter { it.optBoolean("specifiesCoordinate") }
            .fold(Walk(null, 0.0, emptyList())) { walk, element ->
                val entry = point(element, "coordinate") ?: return@fold walk
                val terrain = element.optDouble("terrainAltitude", Double.NaN).takeIf { !it.isNaN() }
                val entryAlt = element.optDouble("amslEntryAlt", Double.NaN)
                val span = element.optDouble("complexDistance", 0.0).takeIf { it > 0.0 && !it.isNaN() }
                val exitAlt = element.optDouble("amslExitAlt", Double.NaN).takeIf { !it.isNaN() }

                val reached = walk.travelled + (walk.at?.let { metresBetween(it, entry) } ?: 0.0)
                val arrival = if (entryAlt.isNaN()) {
                    emptyList()
                } else {
                    listOf(ProfilePoint(reached, terrain, entryAlt))
                }
                val departure = if (span == null || exitAlt == null) {
                    emptyList()
                } else {
                    listOf(ProfilePoint(reached + span, terrain, exitAlt))
                }

                Walk(
                    at = point(element, "exitCoordinate") ?: entry,
                    travelled = reached + (span ?: 0.0),
                    points = walk.points + arrival + departure,
                )
            }
            .points,
    )
}
