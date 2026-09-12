package one.aircast.mapspike

import org.json.JSONObject
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

data class LandingPattern(
    val index: Int,
    val landing: TrackPoint?,
    val slopeStart: TrackPoint?,
    val finalApproach: TrackPoint?,
    val loiterRadiusMetres: Double?,
    val loiterClockwise: Boolean,
)

private fun place(view: JSONObject?, key: String): TrackPoint? =
    view?.optJSONObject(key)?.let {
        val latitude = it.optDouble("latitude", Double.NaN)
        val longitude = it.optDouble("longitude", Double.NaN)
        when {
            latitude.isNaN() || longitude.isNaN() -> null
            else -> TrackPoint(latitude, longitude)
        }
    }

fun isLandingPattern(view: JSONObject?): Boolean =
    view != null && view.optString("kind") != "null" && !view.has("reason")

fun landingPattern(index: Int, view: JSONObject?): LandingPattern? {
    if (view == null || !isLandingPattern(view)) {
        return null
    }
    val pattern = LandingPattern(
        index = index,
        landing = place(view, "landing"),
        slopeStart = place(view, "slopeStart"),
        finalApproach = place(view, "finalApproach"),
        loiterRadiusMetres = view.optDouble("loiterRadiusMetres", Double.NaN).takeIf { !it.isNaN() },
        loiterClockwise = view.optBoolean("loiterClockwise"),
    )
    return pattern.takeIf { it.landing != null || it.slopeStart != null || it.finalApproach != null }
}

internal fun approachPath(pattern: LandingPattern): List<TrackPoint> =
    listOfNotNull(pattern.finalApproach, pattern.slopeStart, pattern.landing)

fun landingPathFeatures(patterns: List<LandingPattern>): FeatureCollection =
    FeatureCollection.fromFeatures(
        patterns.map(::approachPath)
            .filter { it.size >= 2 }
            .map { path ->
                Feature.fromGeometry(
                    LineString.fromLngLats(path.map { Point.fromLngLat(it.longitude, it.latitude) }),
                )
            },
    )

fun landingLoiterFeatures(patterns: List<LandingPattern>): FeatureCollection =
    FeatureCollection.fromFeatures(
        patterns.mapNotNull { pattern ->
            val centre = pattern.finalApproach ?: return@mapNotNull null
            val radius = pattern.loiterRadiusMetres ?: return@mapNotNull null
            circleRing(centre, radius).takeIf { it.size >= 3 }?.let { ring ->
                Feature.fromGeometry(
                    LineString.fromLngLats(
                        (ring + ring.first()).map { Point.fromLngLat(it.longitude, it.latitude) },
                    ),
                )
            }
        },
    )
