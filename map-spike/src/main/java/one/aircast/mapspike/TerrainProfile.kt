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

    val home = elements.optJSONObject(0)
        ?.takeIf { linksStartToHome(json) }
        ?.let { point(it, "coordinate") }

    val endsAfter = routeEndsAfter(elements)

    return TerrainProfile(
        (1 until elements.length())
            .filter { it <= endsAfter }
            .mapNotNull { index -> elements.optJSONObject(index)?.let { index to it } }
            .filter { (_, element) -> isFlownLeg(element) }
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

                Walk(
                    at = point(element, "exitCoordinate") ?: entry,
                    travelled = flown?.last()?.distance ?: (reached + (span ?: 0.0)),
                    points = walk.points + (flown ?: (arrival + departure)),
                )
            }
            .points,
    )
}

const val SEGMENT_READS_PER_POLL = 4

fun stillTheSameItems(was: Map<Int, String>, now: Map<Int, String>): Set<Int> =
    now.filterKeys { was[it] == now[it] }.keys

fun readThisPoll(indices: List<Int>, from: Int): Set<Int> =
    if (indices.isEmpty()) {
        emptySet()
    } else {
        (0 until minOf(SEGMENT_READS_PER_POLL, indices.size))
            .map { indices[(from % indices.size + it) % indices.size] }
            .toSet()
    }

object SegmentBridge {
    private var held = emptyMap<Int, JSONObject?>()
    private var keys = emptyMap<Int, String>()
    private var next = 0
    private var refreshing = emptySet<Int>()

    fun beginPoll(json: JSONObject?) {
        val indices = complexIndices(json)
        val fresh = indices.associateWith { complexKey(json, it) }

        val kept = stillTheSameItems(keys, fresh)

        held = held.filterKeys { it in kept }
        keys = fresh
        refreshing = readThisPoll(indices, next)
        next += refreshing.size
    }

    fun forItem(index: Int): JSONObject? {
        if (held.containsKey(index) && index !in refreshing) {
            return held[index]
        }
        val read = runCatching {
            JSONObject(
                QGCBridge.get("plan.missionController.visualItems.$index.flightPathSegments"),
            )
        }.getOrNull()
        held = held + (index to read)
        return read
    }
}
