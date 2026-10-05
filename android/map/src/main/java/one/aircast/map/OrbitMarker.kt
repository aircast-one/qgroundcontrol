package one.aircast.map

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
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
private const val ORBIT_ARROW_SOURCE = "aircast-orbit-arrows"
private const val ORBIT_ARROW_LAYER = "aircast-orbit-arrow-layer"
private const val ARROW_BEARING = "bearing"
private const val ORBIT_HANDLE_SOURCE = "aircast-orbit-handles"
private const val ORBIT_HANDLE_LAYER = "aircast-orbit-handle-layer"

data class OrbitCircle(val centre: TrackPoint, val radiusMetres: Double, val clockwise: Boolean = true)

fun orbitArrows(orbit: OrbitCircle?): List<Pair<TrackPoint, Double>> =
    orbit?.let { circle ->
        listOf(0.0 to 90.0, 180.0 to 270.0).map { (around, travel) ->
            pointAt(circle.centre, circle.radiusMetres, around) to if (circle.clockwise) travel else (travel + 180.0) % 360.0
        }
    }.orEmpty()

fun orbitCircle(view: JSONObject?): OrbitCircle? {
    val turning = view?.takeIf { it.optBoolean("orbiting") } ?: return null
    val centre = turning.optJSONObject("centre") ?: return null
    val radius = turning.optDouble("radiusMetres").takeIf { !it.isNaN() && it > 0 } ?: return null
    return OrbitCircle(TrackPoint(centre.optDouble("latitude"), centre.optDouble("longitude")), radius, turning.optBoolean("clockwise", true))
}

fun orbitRing(orbit: OrbitCircle?): List<TrackPoint> =
    orbit?.let { circleRing(it.centre, it.radiusMetres) }?.takeIf { it.isNotEmpty() }?.let { it + it.first() }.orEmpty()

fun orbitRadiusHandle(orbit: OrbitCircle?): TrackPoint? = orbit?.let { pointAt(it.centre, it.radiusMetres, 90.0) }

fun orbitHandles(preview: OrbitCircle?): List<TrackPoint> = listOfNotNull(preview?.centre, orbitRadiusHandle(preview))

fun draggedOrbitRadius(orbit: OrbitCircle, to: TrackPoint): Double =
    metresBetween(orbit.centre, to).coerceAtLeast(MINIMUM_CIRCLE_RADIUS_METRES)

object OrbitBridge {
    fun read(): OrbitCircle? = orbitCircle(runCatching { JSONObject(QGCBridge.get(ORBIT_VIEW)) }.getOrNull())
}

fun installOrbitLayer(style: Style) {
    if (style.getSource(ORBIT_SOURCE) != null) return
    style.addSource(GeoJsonSource(ORBIT_SOURCE))
    style.addSource(GeoJsonSource(ORBIT_RING_SOURCE))
    style.addSource(GeoJsonSource(ORBIT_ARROW_SOURCE))
    style.addSource(GeoJsonSource(ORBIT_HANDLE_SOURCE))
    style.addLayer(
        LineLayer(ORBIT_RING_LAYER, ORBIT_RING_SOURCE).withProperties(
            PropertyFactory.lineColor(ORBIT_COLOUR),
            PropertyFactory.lineWidth(2f),
        ),
    )
    style.addLayer(
        SymbolLayer(ORBIT_ARROW_LAYER, ORBIT_ARROW_SOURCE).withProperties(
            PropertyFactory.textField("\u25B2"),
            PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
            PropertyFactory.textSize(14f),
            PropertyFactory.textColor(ORBIT_COLOUR),
            PropertyFactory.textRotate(org.maplibre.android.style.expressions.Expression.get(ARROW_BEARING)),
            PropertyFactory.textRotationAlignment(Property.TEXT_ROTATION_ALIGNMENT_MAP),
            PropertyFactory.textAllowOverlap(true),
            PropertyFactory.textIgnorePlacement(true),
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
    style.addLayer(
        CircleLayer(ORBIT_HANDLE_LAYER, ORBIT_HANDLE_SOURCE).withProperties(
            PropertyFactory.circleColor("#FFFFFF"),
            PropertyFactory.circleRadius(9f),
            PropertyFactory.circleStrokeColor("#000000"),
            PropertyFactory.circleStrokeWidth(2f),
        ),
    )
}

fun renderOrbit(style: Style, active: OrbitCircle?, gotoShown: Boolean, preview: OrbitCircle? = null) {
    val orbit = preview ?: active
    (style.getSource(ORBIT_HANDLE_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(orbitHandles(preview).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) }),
    )
    (style.getSource(ORBIT_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(listOfNotNull(orbit?.takeIf { preview != null || !gotoShown }).map { Feature.fromGeometry(Point.fromLngLat(it.centre.longitude, it.centre.latitude)) }),
    )
    (style.getSource(ORBIT_ARROW_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            orbitArrows(orbit).map { (at, bearing) ->
                Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).also { it.addNumberProperty(ARROW_BEARING, bearing) }
            },
        ),
    )
    (style.getSource(ORBIT_RING_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOf(orbitRing(orbit)).filter { it.isNotEmpty() }.map { points -> Feature.fromGeometry(LineString.fromLngLats(points.map { Point.fromLngLat(it.longitude, it.latitude) })) },
        ),
    )
}
