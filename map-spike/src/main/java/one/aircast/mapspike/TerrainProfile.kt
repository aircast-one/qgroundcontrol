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

// One terrain height per item is one sample of the ground, and stretching it
// across a survey drew flat ground under a flight that climbs 100 m. The
// segments each carry their own run of heights, which is the shape QGC's own
// profile draws, so the ground comes from them.
//
// Only the ground. Every segment of a 5.9 km survey reported coord1AMSLAlt as
// 50, the height above launch, while the item's amslEntryAlt read 1099 against
// terrain of 1049 - the item is the one resolved to AMSL. Taking the planned
// line from the segments put a relative 50 on an AMSL chart and pinned the
// floor 900 m under the flight, so it is interpolated across the item's own
// entry and exit instead.
private fun segmentTerrain(
    segments: JSONObject?,
    from: Double,
    entryAlt: Double,
    exitAlt: Double,
): List<ProfilePoint>? {
    val elements = segments?.optJSONArray("elements")?.takeIf { it.length() > 0 } ?: return null

    val sampled = (0 until elements.length())
        .mapNotNull { elements.optJSONObject(it) }
        .fold(from to emptyList<Pair<Double, Double>>()) { (at, samples), segment ->
            val length = segment.optDouble("totalDistance", 0.0).takeIf { !it.isNaN() } ?: 0.0
            val heights = segment.optJSONArray("amslTerrainHeights")
            val count = heights?.length() ?: 0

            if (count < 2) {
                at + length to samples
            } else {
                at + length to samples + (0 until count).mapNotNull { index ->
                    heights.optDouble(index, Double.NaN).takeIf { !it.isNaN() }?.let {
                        at + length * (index.toDouble() / (count - 1)) to it
                    }
                }
            }
        }
        .second
        .takeIf { it.size >= 2 } ?: return null

    val span = (sampled.last().first - from).takeIf { it > 0.0 } ?: return null

    return sampled.map { (distance, terrain) ->
        ProfilePoint(distance, terrain, entryAlt + (exitAlt - entryAlt) * ((distance - from) / span))
    }
}

fun terrainProfile(
    json: JSONObject?,
    segments: (Int) -> JSONObject? = { null },
): TerrainProfile {
    val elements = json?.optJSONArray("elements") ?: return TerrainProfile(emptyList())

    // QGC counts the leg out of the launch point only when the first item is a
    // takeoff and home is valid - "Link back to home if first item is takeoff",
    // its linkStartToHome. Without that the settings item is a planned home
    // rather than a leg, which is why it is otherwise left out.
    //
    // isTakeoffItem, not the command name: commandName is a tr() string and
    // matching "Takeoff" would work until the app is localised.
    //
    // It seeds where the walk starts from rather than adding a point, so the
    // leg is measured without charting the settings item's 0 m entry altitude
    // as a dive to sea level.
    val home = elements.optJSONObject(0)
        ?.takeIf { elements.optJSONObject(1)?.optBoolean("isTakeoffItem") == true }
        ?.let { point(it, "coordinate") }

    return TerrainProfile(
        (1 until elements.length())
            .mapNotNull { index -> elements.optJSONObject(index)?.let { index to it } }
            .filter { (_, element) -> element.optBoolean("specifiesCoordinate") }
            .fold(Walk(home, 0.0, emptyList())) { walk, (index, element) ->
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
                val flown = if (span == null || entryAlt.isNaN()) {
                    null
                } else {
                    segmentTerrain(segments(index), reached, entryAlt, exitAlt ?: entryAlt)
                }
                val departure = if (span == null || exitAlt == null) {
                    emptyList()
                } else {
                    listOf(ProfilePoint(reached + span, terrain, exitAlt))
                }

                // The segments carry their own entry altitude, and it is the one
                // resolved against terrain. The item's amslEntryAlt reads back as
                // the height above ground until that resolves, and mixing a
                // relative 50 into an AMSL chart pinned the floor 900 m low.
                Walk(
                    at = point(element, "exitCoordinate") ?: entry,
                    travelled = flown?.last()?.distance ?: (reached + (span ?: 0.0)),
                    points = walk.points + (flown ?: (arrival + departure)),
                )
            }
            .points,
    )
}

object SegmentBridge {
    fun forItem(index: Int): JSONObject? =
        runCatching {
            JSONObject(
                QGCBridge.get("plan.missionController.visualItems.$index.flightPathSegments"),
            )
        }.getOrNull()
}
