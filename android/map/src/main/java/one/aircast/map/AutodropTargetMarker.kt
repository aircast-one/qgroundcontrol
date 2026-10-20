package one.aircast.map

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

const val AUTODROP_TARGET_VIEW = "vehicle.autodropTarget"
private const val AUTODROP_TARGET_SOURCE = "aircast-autodrop-target"
const val AUTODROP_TARGET_LAYER = "aircast-autodrop-target-layer"
private const val AUTODROP_TARGET_LABEL_LAYER = "aircast-autodrop-target-label"
private const val AUTODROP_TARGET_COLOUR = "#FF3B30"

fun autodropTargetPoint(reading: JSONObject?): TrackPoint? =
    reading?.optJSONObject("value")
        ?.let { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }
        ?.takeIf { isPlottable(it.latitude, it.longitude) }

object AutodropTargetBridge {
    fun read(): TrackPoint? = autodropTargetPoint(runCatching { JSONObject(QGCBridge.get(AUTODROP_TARGET_VIEW)) }.getOrNull())
}

fun installAutodropTargetLayer(style: Style) {
    if (style.getSource(AUTODROP_TARGET_SOURCE) != null) return
    style.addSource(GeoJsonSource(AUTODROP_TARGET_SOURCE))
    style.addLayer(
        CircleLayer(AUTODROP_TARGET_LAYER, AUTODROP_TARGET_SOURCE).withProperties(
            PropertyFactory.circleColor(AUTODROP_TARGET_COLOUR),
            PropertyFactory.circleRadius(10f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
        ),
    )
    style.addLayer(
        SymbolLayer(AUTODROP_TARGET_LABEL_LAYER, AUTODROP_TARGET_SOURCE).withProperties(
            PropertyFactory.textField("Target"),
            PropertyFactory.textSize(12f),
            PropertyFactory.textColor("#FFFFFF"),
            PropertyFactory.textOffset(arrayOf(0f, 1.6f)),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
        ),
    )
}

fun renderAutodropTarget(style: Style, target: TrackPoint?) {
    (style.getSource(AUTODROP_TARGET_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(listOfNotNull(target).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) }),
    )
}
