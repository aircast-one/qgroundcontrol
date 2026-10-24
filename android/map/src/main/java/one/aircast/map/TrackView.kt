package one.aircast.map

import org.json.JSONObject

const val TRACK_VIEW = "view.track"
const val TRACK_TAIL_VIEW = "view.track(tail)"
const val RECORDED_ROUTE_VIEW = "view.routeRecorder"

data class TrackReading(
    val points: List<TrackPoint>,
    val count: Int,
    val available: Boolean,
    val vehicleId: Long? = null,
    val generation: Long = 0,
    val from: Int = 0,
)

fun trackReading(view: JSONObject?): TrackReading {
    val listed = view?.optJSONArray("points")
    val points = (0 until (listed?.length() ?: 0)).map { index ->
        val point = listed?.optJSONObject(index)
        TrackPoint(point?.optDouble("latitude", Double.NaN) ?: Double.NaN, point?.optDouble("longitude", Double.NaN) ?: Double.NaN)
    }
    return TrackReading(
        points = points,
        count = view?.optInt("count") ?: 0,
        available = view?.optBoolean("available") == true,
        vehicleId = view?.takeIf { it.has("vehicleId") && !it.isNull("vehicleId") }?.optLong("vehicleId"),
        generation = view?.optLong("generation") ?: 0,
        from = view?.optInt("from") ?: 0,
    )
}

fun mergedTrack(held: TrackReading?, tail: TrackReading): TrackReading? = when {
    tail.from == 0 -> tail
    held == null || held.from != 0 || held.vehicleId != tail.vehicleId || held.generation != tail.generation || held.points.size < tail.from -> null
    else -> tail.copy(points = held.points.take(tail.from) + tail.points, from = 0)
}

fun plottedTrack(reading: TrackReading): List<TrackPoint> = reading.points.filter { isPlottable(it.latitude, it.longitude) }

fun trackDraws(reading: TrackReading): Boolean = reading.available && plottedTrack(reading).size > 1

fun recordedRoute(view: JSONObject?): List<TrackPoint> =
    view?.takeIf { it.optBoolean("recording") }?.optJSONArray("path")?.let { path ->
        (0 until path.length()).mapNotNull { path.optJSONObject(it) }.map { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
    }.orEmpty().filter { isPlottable(it.latitude, it.longitude) }
