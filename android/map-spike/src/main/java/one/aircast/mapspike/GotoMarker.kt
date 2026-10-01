package one.aircast.mapspike

import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point
import org.mavlink.qgroundcontrol.QGCBridge

const val GOTO_MAP_CLICK_VIEW = "view.mapClick"
private const val GOTO_SOURCE = "aircast-goto"
private const val GOTO_RING_SOURCE = "aircast-goto-ring"
private const val GOTO_LAYER = "aircast-goto-layer"
private const val GOTO_LABEL_LAYER = "aircast-goto-label"
private const val GOTO_RING_LAYER = "aircast-goto-ring-layer"
private const val GOTO_COLOUR = "#2E7D32"

data class GotoLocation(val at: TrackPoint, val loiterRadiusMetres: Double?)

fun gotoLocation(view: JSONObject?): GotoLocation? =
    view?.optJSONObject("gotoLocation")?.let { json ->
        GotoLocation(
            TrackPoint(json.optDouble("latitude"), json.optDouble("longitude")),
            json.optDouble("loiterRadiusMetres").takeIf { !it.isNaN() && it > 0 },
        )
    }?.takeIf { isPlottable(it.at.latitude, it.at.longitude) }

fun gotoRing(location: GotoLocation?): List<TrackPoint> =
    location?.loiterRadiusMetres?.let { circleRing(location.at, it) }?.takeIf { it.isNotEmpty() }?.let { it + it.first() }.orEmpty()

object GotoBridge {
    fun read(): GotoLocation? = gotoLocation(runCatching { JSONObject(QGCBridge.get(GOTO_MAP_CLICK_VIEW)) }.getOrNull())
}

fun installGotoLayer(style: Style) {
    if (style.getSource(GOTO_SOURCE) != null) return
    style.addSource(GeoJsonSource(GOTO_SOURCE))
    style.addSource(GeoJsonSource(GOTO_RING_SOURCE))
    style.addLayer(
        LineLayer(GOTO_RING_LAYER, GOTO_RING_SOURCE).withProperties(
            PropertyFactory.lineColor(GOTO_COLOUR),
            PropertyFactory.lineWidth(2f),
        ),
    )
    style.addLayer(
        CircleLayer(GOTO_LAYER, GOTO_SOURCE).withProperties(
            PropertyFactory.circleColor(GOTO_COLOUR),
            PropertyFactory.circleRadius(9f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
        ),
    )
    style.addLayer(
        SymbolLayer(GOTO_LABEL_LAYER, GOTO_SOURCE).withProperties(
            PropertyFactory.textField("Go here"),
            PropertyFactory.textSize(12f),
            PropertyFactory.textColor("#FFFFFF"),
            PropertyFactory.textOffset(arrayOf(0f, 1.6f)),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
        ),
    )
}

fun renderGoto(style: Style, location: GotoLocation?) {
    (style.getSource(GOTO_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(listOfNotNull(location).map { Feature.fromGeometry(Point.fromLngLat(it.at.longitude, it.at.latitude)) }),
    )
    val ring = gotoRing(location)
    (style.getSource(GOTO_RING_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOf(ring).filter { it.isNotEmpty() }.map { points -> Feature.fromGeometry(LineString.fromLngLats(points.map { Point.fromLngLat(it.longitude, it.latitude) })) },
        ),
    )
}
