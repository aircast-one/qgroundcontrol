package one.aircast.mapspike

import org.json.JSONObject
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon

data class LandingPattern(
    val index: Int,
    val landing: TrackPoint?,
    val slopeStart: TrackPoint?,
    val finalApproach: TrackPoint?,
    val loiterRadiusMetres: Double?,
    val loiterClockwise: Boolean,
    val loiterRadiusText: String = "",
    val loiterToAltitude: Boolean = false,
    val heights: GlideSlopeHeights? = null,
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

data class GlideSlopeHeights(val transition: String, val midSlope: String, val approach: String)

data class LandingLabel(val at: TrackPoint, val text: String)

fun landingLabels(pattern: LandingPattern): List<LandingLabel> {
    val landing = pattern.landing ?: return emptyList()
    val slopeStart = pattern.slopeStart ?: return emptyList()
    val bearing = azimuthBetween(landing, slopeStart)
    val side = bearing + if (bearing > 180) -90 else 90
    val transition = pointAt(landing, LANDING_LENGTH_M / 2, bearing)
    val top = (if (pattern.loiterToAltitude) slopeStart else pattern.finalApproach) ?: slopeStart
    val mid = pointAt(transition, metresBetween(transition, top) / 2, bearing)
    val heights = pattern.heights
    return listOfNotNull(
        LandingLabel(landing, "Landing Area"),
        LandingLabel(pointAt(landing, LANDING_LENGTH_M / 2 + 2, bearing), "Glide Slope"),
        heights?.let { LandingLabel(pointAt(transition, LANDING_WIDTH_M, side), it.transition) },
        heights?.let { LandingLabel(pointAt(mid, LANDING_WIDTH_M / 2, side), it.midSlope) },
        heights?.let { LandingLabel(slopeStart, it.approach) },
    )
}

fun structureScanLabels(items: List<MissionItem>): List<LandingLabel> =
    items.filter { it.kind == KIND_STRUCTURE }.flatMap { item ->
        item.exit?.let { exit -> listOf(LandingLabel(TrackPoint(item.latitude, item.longitude), "Entry"), LandingLabel(exit, "Exit")) }.orEmpty()
    }

fun landingLabelFeatures(patterns: List<LandingPattern>, selected: Int?, items: List<MissionItem> = emptyList()): FeatureCollection =
    FeatureCollection.fromFeatures(
        (patterns.filter { it.index == selected }.flatMap(::landingLabels) + structureScanLabels(items)).map { label ->
            Feature.fromGeometry(Point.fromLngLat(label.at.longitude, label.at.latitude)).apply { addStringProperty(LANDING_LABEL_TEXT, label.text) }
        },
    )

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
        loiterRadiusText = view.optText("loiterRadiusText"),
        loiterToAltitude = view.optBoolean("loiterToAltitude"),
        heights = view.optJSONObject("heights")?.let { GlideSlopeHeights(it.optText("transition"), it.optText("midSlope"), it.optText("approach")) },
    )
    return pattern.takeIf { it.landing != null || it.slopeStart != null || it.finalApproach != null }
}

internal fun landingText(pattern: LandingPattern?): String? {
    val radius = pattern?.loiterRadiusText?.ifBlank { null } ?: return null
    val turn = if (pattern.loiterClockwise) "clockwise" else "anticlockwise"
    return "circles $radius $turn"
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

private const val LANDING_WIDTH_M = 15.0
private const val LANDING_LENGTH_M = 100.0
const val LANDING_AREA_KIND = "area"
const val GLIDE_SLOPE_KIND = "slope"

private fun landingCorners(landing: TrackPoint, bearing: Double): List<TrackPoint> {
    val angle = Math.toDegrees(kotlin.math.atan((LANDING_WIDTH_M / 2) / (LANDING_LENGTH_M / 2)))
    val hypotenuse = (LANDING_WIDTH_M / 2) / kotlin.math.sin(Math.toRadians(angle))
    return listOf(bearing - angle, bearing + angle, bearing + (180 - angle), bearing - (180 - angle)).map { pointAt(landing, hypotenuse, it) }
}

fun landingArea(pattern: LandingPattern): List<TrackPoint>? {
    val landing = pattern.landing ?: return null
    val slopeStart = pattern.slopeStart ?: return null
    return landingCorners(landing, azimuthBetween(landing, slopeStart))
}

fun glideSlope(pattern: LandingPattern): List<TrackPoint>? {
    val landing = pattern.landing ?: return null
    val slopeStart = pattern.slopeStart ?: return null
    val top = (if (pattern.loiterToAltitude) slopeStart else pattern.finalApproach) ?: return null
    return landingCorners(landing, azimuthBetween(landing, slopeStart)).take(2) + top
}

fun landingAreaFeatures(patterns: List<LandingPattern>): FeatureCollection =
    FeatureCollection.fromFeatures(
        patterns.flatMap { pattern ->
            listOfNotNull(landingArea(pattern)?.let { LANDING_AREA_KIND to it }, glideSlope(pattern)?.let { GLIDE_SLOPE_KIND to it })
        }.map { (kind, ring) ->
            Feature.fromGeometry(Polygon.fromLngLats(listOf((ring + ring.first()).map { Point.fromLngLat(it.longitude, it.latitude) }))).apply { addStringProperty(LANDING_SHAPE_KIND, kind) }
        },
    )

fun loiterRings(patterns: List<LandingPattern>, items: List<MissionItem>): List<Pair<TrackPoint, Double>> =
    patterns.filter { it.loiterToAltitude }.mapNotNull { pattern -> pattern.finalApproach?.let { centre -> pattern.loiterRadiusMetres?.let { centre to it } } } +
        items.filter { it.loiterRadius.isFinite() && it.loiterRadius != 0.0 }.map { TrackPoint(it.latitude, it.longitude) to kotlin.math.abs(it.loiterRadius) }

fun landingLoiterFeatures(patterns: List<LandingPattern>, items: List<MissionItem> = emptyList()): FeatureCollection =
    FeatureCollection.fromFeatures(
        loiterRings(patterns, items).mapNotNull { (centre, radius) ->
            circleRing(centre, radius).takeIf { it.size >= 3 }?.let { ring ->
                Feature.fromGeometry(
                    LineString.fromLngLats(
                        (ring + ring.first()).map { Point.fromLngLat(it.longitude, it.latitude) },
                    ),
                )
            }
        },
    )
