package one.aircast.map

import android.annotation.SuppressLint
import android.view.MotionEvent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONObject
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView
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
private const val GOTO_HANDLE_SOURCE = "aircast-goto-handle"
private const val GOTO_HANDLE_LAYER = "aircast-goto-handle-layer"
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

data class LoiterEdit(val radiusMetres: Double, val clockwise: Boolean, val unit: String, val metresPerUnit: Double)

object GotoLoiterEdit {
    var edit by mutableStateOf<LoiterEdit?>(null)
}

fun loiterEditNumber(edit: LoiterEdit): String =
    String.format(java.util.Locale.US, "%.1f", edit.radiusMetres / edit.metresPerUnit).removeSuffix(".0")

fun editedGoto(location: GotoLocation?, edit: LoiterEdit?): GotoLocation? =
    edit?.let { changing ->
        location?.takeIf { it.loiterRadiusMetres != null }?.copy(
            loiterRadiusMetres = changing.radiusMetres,
            loiterClockwise = changing.clockwise,
            loiterRadiusText = listOf(loiterEditNumber(changing), changing.unit).filter { it.isNotBlank() }.joinToString(" "),
        )
    } ?: location

fun gotoRadiusHandle(location: GotoLocation?): TrackPoint? =
    location?.loiterRadiusMetres?.let { pointAt(location.at, it, 90.0) }

fun draggedGotoRadius(location: GotoLocation, to: TrackPoint): Double =
    metresBetween(location.at, to).coerceAtLeast(MINIMUM_CIRCLE_RADIUS_METRES)

private enum class CircleGrab { None, GotoRadius, GotoFlip, OrbitRadius, OrbitCentre, OrbitFlip }

@SuppressLint("ClickableViewAccessibility")
fun attachGotoRadiusDrag(mapView: MapView, map: MapLibreMap, shown: () -> GotoLocation?) {
    var grab = CircleGrab.None
    var downX = 0f
    var downY = 0f
    fun near(at: TrackPoint?, x: Float, y: Float): Boolean =
        at != null && map.projection.toScreenLocation(LatLng(at.latitude, at.longitude)).let { withinHit(it.x - x, it.y - y) }
    fun nearArrow(arrows: List<Pair<TrackPoint, Double>>, x: Float, y: Float): Boolean = arrows.any { (at, _) -> near(at, x, y) }
    fun grabbed(x: Float, y: Float): CircleGrab {
        val orbit = OrbitPreview.circle
        val goto = shown().takeIf { GotoLoiterEdit.edit != null }
        return when {
            near(orbitRadiusHandle(orbit), x, y) -> CircleGrab.OrbitRadius
            near(orbit?.centre, x, y) -> CircleGrab.OrbitCentre
            nearArrow(orbitArrows(orbit), x, y) -> CircleGrab.OrbitFlip
            near(gotoRadiusHandle(goto), x, y) -> CircleGrab.GotoRadius
            nearArrow(gotoArrows(goto), x, y) -> CircleGrab.GotoFlip
            else -> CircleGrab.None
        }
    }
    fun dragTo(x: Float, y: Float) {
        val to = map.projection.fromScreenLocation(android.graphics.PointF(x, y)).let { TrackPoint(it.latitude, it.longitude) }
        val orbit = OrbitPreview.circle
        val edit = GotoLoiterEdit.edit
        val goto = shown()
        when {
            grab == CircleGrab.OrbitRadius && orbit != null -> OrbitPreview.circle = orbit.copy(radiusMetres = draggedOrbitRadius(orbit, to))
            grab == CircleGrab.OrbitCentre && orbit != null -> OrbitPreview.circle = orbit.copy(centre = to)
            grab == CircleGrab.GotoRadius && edit != null && goto != null -> GotoLoiterEdit.edit = edit.copy(radiusMetres = draggedGotoRadius(goto, to))
        }
    }
    fun flip() {
        val orbit = OrbitPreview.circle
        val edit = GotoLoiterEdit.edit
        when {
            grab == CircleGrab.OrbitFlip && orbit != null -> OrbitPreview.circle = orbit.copy(clockwise = !orbit.clockwise)
            grab == CircleGrab.GotoFlip && edit != null -> GotoLoiterEdit.edit = edit.copy(clockwise = !edit.clockwise)
        }
    }
    val dragging = { grab in setOf(CircleGrab.GotoRadius, CircleGrab.OrbitRadius, CircleGrab.OrbitCentre) }
    mapView.setOnTouchListener { _, event ->
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.x
                downY = event.y
                grab = grabbed(event.x, event.y)
                if (dragging()) map.uiSettings.setAllGesturesEnabled(false)
                grab != CircleGrab.None
            }

            MotionEvent.ACTION_MOVE -> {
                if (dragging()) dragTo(event.x, event.y)
                grab != CircleGrab.None
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                val consumed = grab != CircleGrab.None
                if (event.actionMasked == MotionEvent.ACTION_UP && withinTap(event.x - downX, event.y - downY)) flip()
                if (dragging()) map.uiSettings.setAllGesturesEnabled(true)
                grab = CircleGrab.None
                consumed
            }

            else -> grab != CircleGrab.None
        }
    }
}

object GotoBridge {
    fun read(): GotoLocation? = gotoLocation(runCatching { JSONObject(QGCBridge.get(GOTO_MAP_CLICK_VIEW)) }.getOrNull())
}

fun installGotoLayer(style: Style) {
    if (style.getSource(GOTO_SOURCE) != null) return
    style.addSource(GeoJsonSource(GOTO_SOURCE))
    style.addSource(GeoJsonSource(GOTO_RING_SOURCE))
    style.addSource(GeoJsonSource(GOTO_RADIUS_SOURCE))
    style.addSource(GeoJsonSource(GOTO_ARROW_SOURCE))
    style.addSource(GeoJsonSource(GOTO_HANDLE_SOURCE))
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
        CircleLayer(GOTO_HANDLE_LAYER, GOTO_HANDLE_SOURCE).withProperties(
            PropertyFactory.circleColor("#FFFFFF"),
            PropertyFactory.circleRadius(9f),
            PropertyFactory.circleStrokeColor(GOTO_COLOUR),
            PropertyFactory.circleStrokeWidth(3f),
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

fun renderGoto(style: Style, location: GotoLocation?, editing: Boolean = false) {
    (style.getSource(GOTO_HANDLE_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            listOfNotNull(gotoRadiusHandle(location.takeIf { editing })).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) },
        ),
    )
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
