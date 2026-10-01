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

const val ORBIT_VIEW = "view.orbit"
private const val ORBIT_SOURCE = "aircast-orbit"
private const val ORBIT_RING_SOURCE = "aircast-orbit-ring"
private const val ORBIT_LAYER = "aircast-orbit-layer"
private const val ORBIT_LABEL_LAYER = "aircast-orbit-label"
private const val ORBIT_RING_LAYER = "aircast-orbit-ring-layer"
private const val ORBIT_COLOUR = "#FFFFFF"

data class OrbitCircle(val centre: TrackPoint, val radiusMetres: Double)

fun orbitCircle(view: JSONObject?): OrbitCircle? {
    val turning = view?.takeIf { it.optBoolean("orbiting") } ?: return null
    val centre = turning.optJSONObject("centre") ?: return null
    val radius = turning.optDouble("radiusMetres").takeIf { !it.isNaN() && it > 0 } ?: return null
    return OrbitCircle(TrackPoint(centre.optDouble("latitude"), centre.optDouble("longitude")), radius)
}

fun orbitRing(orbit: OrbitCircle?): List<TrackPoint> =
    orbit?.let { circleRing(it.centre, it.radiusMetres) }?.takeIf { it.isNotEmpty() }?.let { it + it.first() }.orEmpty()

object OrbitBridge {
    fun read(): OrbitCircle? = orbitCircle(runCatching { JSONObject(QGCBridge.get(ORBIT_VIEW)) }.getOrNull())
}

fun installOrbitLayer(style: Style) {
    if (style.getSource(ORBIT_SOURCE) != null) return
    style.addSource(GeoJsonSource(ORBIT_SOURCE))
    style.addSource(GeoJsonSource(ORBIT_RING_SOURCE))
    style.addLayer(
        LineLayer(ORBIT_RING_LAYER, ORBIT_RING_SOURCE).withProperties(
            PropertyFactory.lineColor(ORBIT_COLOUR),
            PropertyFactory.lineWidth(2f),
        ),
    )
    style.addLayer(
        CircleLayer(ORBIT_LAYER, ORBIT_SOURCE).withProperties(
            PropertyFactory.circleColor(ORBIT_COLOUR),
            PropertyFactory.circleRadius(6f),
        ),
    )
    style.addLayer(
        SymbolLayer(ORBIT_LABEL_LAYER, ORBIT_SOURCE).withProperties(
            PropertyFactory.textField("Orbit"),
            PropertyFactory.textSize(12f),
            PropertyFactory.textColor("#FFFFFF"),
            PropertyFactory.textOffset(arrayOf(0f, 1.4f)),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
        ),
    )
}

fun renderOrbit(style: Style, orbit: OrbitCircle?, gotoShown: Boolean) {
    (style.getSource(ORBIT_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(listOfNotNull(orbit?.takeIf { !gotoShown }).map { Feature.fromGeometry(Point.fromLngLat(it.centre.longitude, it.centre.latitude)) }),
    )
    (style.getSource(ORBIT_RING_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOf(orbitRing(orbit)).filter { it.isNotEmpty() }.map { points -> Feature.fromGeometry(LineString.fromLngLats(points.map { Point.fromLngLat(it.longitude, it.latitude) })) },
        ),
    )
}
