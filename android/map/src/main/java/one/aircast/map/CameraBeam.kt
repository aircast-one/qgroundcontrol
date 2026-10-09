package one.aircast.map

import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.FillLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon

const val SYNTHETIC_VIEW_PATH = "view.syntheticView"
private const val BEAM_SOURCE = "aircast-camera-beam"
private const val BEAM_FILL_LAYER = "aircast-camera-beam-fill"
private const val BEAM_LINE_LAYER = "aircast-camera-beam-line"
private const val BEAM_COLOUR = "#FFE9A6"
private const val BEAM_FILL_OPACITY = 0.28f
private const val BEAM_LINE_OPACITY = 0.6f
private const val BEAM_LINE_WIDTH = 1.5f
private const val POLYGON_POINTS = 4

fun cameraBeam(view: JSONObject?): List<TrackPoint> {
    val points = view?.takeIf { it.optBoolean("available") }?.optJSONArray("beam") ?: return emptyList()
    return (0 until points.length()).mapNotNull { points.optJSONObject(it) }.map { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
}

fun beamFeatures(beam: List<TrackPoint>): FeatureCollection =
    FeatureCollection.fromFeatures(
        listOfNotNull(
            beam.takeIf { it.size >= POLYGON_POINTS && it.all { point -> isPlottable(point.latitude, point.longitude) } }
                ?.let { ring -> Feature.fromGeometry(Polygon.fromLngLats(listOf(ring.map { Point.fromLngLat(it.longitude, it.latitude) }))) },
        ),
    )

fun installCameraBeamLayer(style: Style) {
    if (style.getSource(BEAM_SOURCE) != null) return
    style.addSource(GeoJsonSource(BEAM_SOURCE))
    style.addLayer(
        FillLayer(BEAM_FILL_LAYER, BEAM_SOURCE).withProperties(
            PropertyFactory.fillColor(BEAM_COLOUR),
            PropertyFactory.fillOpacity(BEAM_FILL_OPACITY),
        ),
    )
    style.addLayer(
        LineLayer(BEAM_LINE_LAYER, BEAM_SOURCE).withProperties(
            PropertyFactory.lineColor(BEAM_COLOUR),
            PropertyFactory.lineOpacity(BEAM_LINE_OPACITY),
            PropertyFactory.lineWidth(BEAM_LINE_WIDTH),
        ),
    )
}

fun renderCameraBeam(style: Style, beam: List<TrackPoint>) {
    (style.getSource(BEAM_SOURCE) as? GeoJsonSource)?.setGeoJson(beamFeatures(beam))
}
