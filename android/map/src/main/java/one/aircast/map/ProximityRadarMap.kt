package one.aircast.map

import com.google.gson.JsonObject
import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

private const val RADAR_SOURCE = "aircast-proximity-radar"
private const val RADAR_LAYER = "aircast-proximity-radar-layer"
private const val COLOUR_PROPERTY = "colour"
private const val LIMIT_COLOUR = "#FFFFFF"
private const val SECTOR_COLOUR = "#FF0000"
private const val RADAR_OPACITY = 0.5f
private const val RADAR_LINE_WIDTH = 3f
private const val SECTOR_SWEEP_DEGREES = 45.0
private const val ARC_STEPS = 9
private const val LIMIT_SEGMENTS = 64

data class RadarReading(val maxMeters: Double?, val sectors: List<Pair<Double, Double>>)

fun radarReading(view: JSONObject?): RadarReading? = view?.takeIf { it.optBoolean("shown") }?.let {
    val listed = it.optJSONArray("sectors")
    RadarReading(
        maxMeters = if (it.isNull("maxMeters")) null else it.optDouble("maxMeters").takeIf { m -> m.isFinite() },
        sectors = (0 until (listed?.length() ?: 0)).mapNotNull { index -> listed?.optJSONObject(index) }
            .filter { sector -> !sector.isNull("meters") }
            .map { sector -> sector.optDouble("bearing") to sector.optDouble("meters") },
    )
}

fun radarLines(centre: TrackPoint, heading: Double, reading: RadarReading): List<Pair<String, List<TrackPoint>>> {
    val facing = if (heading.isNaN()) 0.0 else heading
    val limit = reading.maxMeters?.let { metres -> circleRing(centre, metres, LIMIT_SEGMENTS).let { it + it.first() } }
    val arcs = reading.sectors.filter { (_, metres) -> metres > 0 }.map { (bearing, metres) ->
        val start = facing + bearing - SECTOR_SWEEP_DEGREES / 2
        SECTOR_COLOUR to (0..ARC_STEPS).map { step -> pointAt(centre, metres, start + SECTOR_SWEEP_DEGREES * step / ARC_STEPS) }
    }
    return listOfNotNull(limit?.let { LIMIT_COLOUR to it }) + arcs
}

fun installProximityRadarLayer(style: Style) {
    if (style.getSource(RADAR_SOURCE) != null) return
    style.addSource(GeoJsonSource(RADAR_SOURCE))
    style.addLayer(
        LineLayer(RADAR_LAYER, RADAR_SOURCE).withProperties(
            PropertyFactory.lineColor(Expression.get(COLOUR_PROPERTY)),
            PropertyFactory.lineWidth(RADAR_LINE_WIDTH),
            PropertyFactory.lineOpacity(RADAR_OPACITY),
        ),
    )
}

data class PlacedRadar(val at: TrackPoint, val heading: Double, val reading: RadarReading)

fun placedRadars(fleet: List<VehicleChoice>, active: TrackPoint, activeHeading: Double): List<PlacedRadar> =
    fleet.mapNotNull { vehicle ->
        vehicle.radar?.let { reading ->
            when {
                vehicle.active -> PlacedRadar(active, activeHeading, reading)
                else -> PlacedRadar(TrackPoint(vehicle.latitude, vehicle.longitude), vehicle.heading, reading)
            }
        }
    }.filter { isPlottable(it.at.latitude, it.at.longitude) }

fun renderProximityRadars(style: Style, radars: List<PlacedRadar>) {
    val lines = radars.flatMap { radarLines(it.at, it.heading, it.reading) }
    (style.getSource(RADAR_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            lines.map { (colour, points) ->
                Feature.fromGeometry(
                    LineString.fromLngLats(points.map { Point.fromLngLat(it.longitude, it.latitude) }),
                    JsonObject().apply { addProperty(COLOUR_PROPERTY, colour) },
                )
            },
        ),
    )
}
