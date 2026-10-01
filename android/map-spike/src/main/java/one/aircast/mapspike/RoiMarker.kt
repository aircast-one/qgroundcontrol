package one.aircast.mapspike

import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.Point
import org.mavlink.qgroundcontrol.QGCBridge

const val ROI_GUIDED_VIEW = "view.guidedActions"
private const val ROI_SOURCE = "aircast-roi"
const val ROI_LAYER = "aircast-roi-layer"
private const val ROI_LABEL_LAYER = "aircast-roi-label"
private const val ROI_COLOUR = "#FF9500"

fun roiPoint(view: JSONObject?): TrackPoint? =
    view?.optJSONObject("roi")
        ?.let { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
        ?.takeIf { isPlottable(it.latitude, it.longitude) }

object RoiBridge {
    fun read(): TrackPoint? = roiPoint(runCatching { JSONObject(QGCBridge.get(ROI_GUIDED_VIEW)) }.getOrNull())
}

fun installRoiLayer(style: Style) {
    if (style.getSource(ROI_SOURCE) != null) return
    style.addSource(GeoJsonSource(ROI_SOURCE))
    style.addLayer(
        CircleLayer(ROI_LAYER, ROI_SOURCE).withProperties(
            PropertyFactory.circleColor(ROI_COLOUR),
            PropertyFactory.circleRadius(10f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
        ),
    )
    style.addLayer(
        SymbolLayer(ROI_LABEL_LAYER, ROI_SOURCE).withProperties(
            PropertyFactory.textField("ROI here"),
            PropertyFactory.textSize(12f),
            PropertyFactory.textColor("#FFFFFF"),
            PropertyFactory.textOffset(arrayOf(0f, 1.6f)),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
        ),
    )
}

fun renderRoi(style: Style, roi: TrackPoint?) {
    (style.getSource(ROI_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(listOfNotNull(roi).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) }),
    )
}
