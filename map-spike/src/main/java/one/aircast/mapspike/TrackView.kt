package one.aircast.mapspike

import org.json.JSONObject

const val TRACK_VIEW = "view.track"

data class TrackReading(
    val points: List<TrackPoint>,
    val dropped: Int,
    val count: Int,
    val available: Boolean,
)

fun trackReading(view: JSONObject?): TrackReading {
    val listed = view?.optJSONArray("points")
    val points = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
        listed?.optJSONObject(index)?.let { point ->
            val latitude = point.optDouble("latitude", Double.NaN)
            val longitude = point.optDouble("longitude", Double.NaN)
            if (isPlottable(latitude, longitude)) TrackPoint(latitude, longitude) else null
        }
    }
    return TrackReading(
        points = points,
        dropped = view?.optInt("dropped") ?: 0,
        count = view?.optInt("count") ?: 0,
        available = view?.optBoolean("available") == true,
    )
}

fun trackDraws(reading: TrackReading): Boolean = reading.available && reading.points.size > 1

fun trimmedNotice(reading: TrackReading): String? =
    reading.dropped.takeIf { it > 0 && reading.available }
        ?.let { "Trail trimmed — showing the last ${reading.count} positions of this flight." }
