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
private const val GOTO_RADIUS_SOURCE = "aircast-goto-radius"
private const val GOTO_RADIUS_LAYER = "aircast-goto-radius-layer"
private const val RADIUS_TEXT = "text"
private const val GOTO_ARROW_SOURCE = "aircast-goto-arrows"
private const val GOTO_ARROW_LAYER = "aircast-goto-arrow-layer"
private const val ARROW_BEARING = "bearing"
private const val GOTO_COLOUR = "#2E7D32"

data class GotoLocation(val at: TrackPoint, val loiterRadiusMetres: Double?, val loiterRadiusText: String = "", val loiterClockwise: Boolean = true)

fun gotoLocation(view: JSONObject?): GotoLocation? =
    view?.optJSONObject("gotoLocation")?.let { json ->
        GotoLocation(
            TrackPoint(json.optDouble("latitude"), json.optDouble("longitude")),
            json.optDouble("loiterRadiusMetres").takeIf { !it.isNaN() && it > 0 },
            json.optText("loiterRadiusText"),
            json.optBoolean("loiterClockwise", true),
        )
    }?.takeIf { isPlottable(it.at.latitude, it.at.longitude) }

fun gotoRing(location: GotoLocation?): List<TrackPoint> =
    location?.loiterRadiusMetres?.let { circleRing(location.at, it) }?.takeIf { it.isNotEmpty() }?.let { it + it.first() }.orEmpty()

fun gotoArrows(location: GotoLocation?): List<Pair<TrackPoint, Double>> =
    orbitArrows(location?.loiterRadiusMetres?.let { OrbitCircle(location.at, it, location.loiterClockwise) })

object GotoBridge {
    fun read(): GotoLocation? = gotoLocation(runCatching { JSONObject(QGCBridge.get(GOTO_MAP_CLICK_VIEW)) }.getOrNull())
}

fun installGotoLayer(style: Style) {
    if (style.getSource(GOTO_SOURCE) != null) return
    style.addSource(GeoJsonSource(GOTO_SOURCE))
    style.addSource(GeoJsonSource(GOTO_RING_SOURCE))
    style.addSource(GeoJsonSource(GOTO_RADIUS_SOURCE))
    style.addSource(GeoJsonSource(GOTO_ARROW_SOURCE))
    style.addLayer(
        LineLayer(GOTO_RING_LAYER, GOTO_RING_SOURCE).withProperties(
            PropertyFactory.lineColor(GOTO_COLOUR),
            PropertyFactory.lineWidth(2f),
        ),
    )
    style.addLayer(
        SymbolLayer(GOTO_ARROW_LAYER, GOTO_ARROW_SOURCE).withProperties(
            PropertyFactory.textField("\u25B2"),
            PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
            PropertyFactory.textSize(14f),
            PropertyFactory.textColor(GOTO_COLOUR),
            PropertyFactory.textRotate(org.maplibre.android.style.expressions.Expression.get(ARROW_BEARING)),
            PropertyFactory.textRotationAlignment(Property.TEXT_ROTATION_ALIGNMENT_MAP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
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
        SymbolLayer(GOTO_RADIUS_LAYER, GOTO_RADIUS_SOURCE).withProperties(
            PropertyFactory.textField(org.maplibre.android.style.expressions.Expression.get(RADIUS_TEXT)),
            PropertyFactory.textSize(12f),
            PropertyFactory.textColor("#000000"),
            PropertyFactory.textHaloColor("#80FFFFFF"),
            PropertyFactory.textHaloWidth(4f),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_BOTTOM),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
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
    (style.getSource(GOTO_RADIUS_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOfNotNull(location?.takeIf { it.loiterRadiusMetres != null && it.loiterRadiusText.isNotBlank() }).map { shown ->
                val edge = pointAt(shown.at, shown.loiterRadiusMetres ?: 0.0, 0.0)
                Feature.fromGeometry(Point.fromLngLat(edge.longitude, edge.latitude)).also { it.addStringProperty(RADIUS_TEXT, shown.loiterRadiusText) }
            },
        ),
    )
    (style.getSource(GOTO_ARROW_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            gotoArrows(location).map { (at, bearing) ->
                Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).also { it.addNumberProperty(ARROW_BEARING, bearing) }
            },
        ),
    )
    val ring = gotoRing(location)
    (style.getSource(GOTO_RING_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOf(ring).filter { it.isNotEmpty() }.map { points -> Feature.fromGeometry(LineString.fromLngLats(points.map { Point.fromLngLat(it.longitude, it.latitude) })) },
        ),
    )
}

private const val CLICK_MARKER_SOURCE = "aircast-click-marker"
private const val CLICK_MARKER_SHADOW_LAYER = "aircast-click-marker-shadow"
private const val CLICK_MARKER_RING_LAYER = "aircast-click-marker-ring"
private const val CLICK_MARKER_DOT_LAYER = "aircast-click-marker-dot"
private const val CLICK_MARKER_RADIUS = 13f

fun installClickMarker(style: Style) {
    if (style.getSource(CLICK_MARKER_SOURCE) != null) return
    style.addSource(GeoJsonSource(CLICK_MARKER_SOURCE))
    style.addLayer(
        CircleLayer(CLICK_MARKER_SHADOW_LAYER, CLICK_MARKER_SOURCE).withProperties(
            PropertyFactory.circleRadius(CLICK_MARKER_RADIUS),
            PropertyFactory.circleOpacity(0f),
            PropertyFactory.circleStrokeColor("rgba(0, 0, 0, 0.6)"),
            PropertyFactory.circleStrokeWidth(4f),
        ),
    )
    style.addLayer(
        CircleLayer(CLICK_MARKER_RING_LAYER, CLICK_MARKER_SOURCE).withProperties(
            PropertyFactory.circleRadius(CLICK_MARKER_RADIUS - 1f),
            PropertyFactory.circleOpacity(0f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
        ),
    )
    style.addLayer(
        CircleLayer(CLICK_MARKER_DOT_LAYER, CLICK_MARKER_SOURCE).withProperties(
            PropertyFactory.circleRadius(2f),
            PropertyFactory.circleColor("#FFFFFF"),
        ),
    )
}

fun clickMarkerFeatures(at: TrackPoint?): FeatureCollection =
    FeatureCollection.fromFeatures(
        listOfNotNull(at?.takeIf { isPlottable(it.latitude, it.longitude) }).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) },
    )

fun renderClickMarker(style: Style, at: TrackPoint?) {
    (style.getSource(CLICK_MARKER_SOURCE) as? GeoJsonSource)?.setGeoJson(clickMarkerFeatures(at))
}
