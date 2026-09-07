package one.aircast.mapspike

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import org.maplibre.android.MapLibre
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.camera.CameraUpdateFactory
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.geometry.LatLngBounds
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
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

private const val VEHICLE_SOURCE = "aircast-vehicle"
private const val VEHICLE_LAYER = "aircast-vehicle-layer"
private const val VEHICLE_HEADING_LAYER = "aircast-vehicle-heading-layer"
private const val VEHICLE_ARROW_IMAGE = "aircast-vehicle-arrow"

const val HEADING_PROPERTY = "heading"
private const val TRAIL_SOURCE = "aircast-trail"
private const val HOME_SOURCE = "aircast-home"
private const val HOME_LAYER = "aircast-home-layer"
private const val HOME_LABEL_LAYER = "aircast-home-label-layer"
private const val TRAIL_LAYER = "aircast-trail-layer"

private const val DEFAULT_ZOOM = 16.0
private const val MAX_TRAIL_POINTS = 500

// No aircraft covers this between samples. A jump this large means the active
// vehicle changed, and joining the two tracks draws a line across the world.
const val TRAIL_BREAK_DEGREES = 0.5
private const val MIN_FIT_SPAN_DEGREES = 1e-5
private const val FIT_PADDING_PIXELS = 80
private const val LOGO_EDGE_MARGIN_PX = 16

const val DEMO_STYLE_URL = "https://demotiles.maplibre.org/style.json"

// Demo only. Production points at QGC's existing SQLite tile cache.
const val OSM_RASTER_STYLE = """
{
  "version": 8,
  "sources": {
    "osm": {
      "type": "raster",
      "tiles": ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
      "tileSize": 256,
      "attribution": "(c) OpenStreetMap contributors"
    }
  },
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "layers": [ { "id": "osm", "type": "raster", "source": "osm" } ]
}
"""

data class TrackPoint(val latitude: Double, val longitude: Double)

fun isPlottable(latitude: Double, longitude: Double): Boolean =
    !latitude.isNaN() && !longitude.isNaN() &&
        !(latitude == 0.0 && longitude == 0.0) &&
        latitude in -90.0..90.0 && longitude in -180.0..180.0

// Heading only means something once the vehicle reports it. Without it the
// feature carries no heading property and the arrow layer filters itself out,
// leaving the plain position dot.
fun vehicleFeature(latitude: Double, longitude: Double, heading: Double): Feature =
    Feature.fromGeometry(Point.fromLngLat(longitude, latitude)).apply {
        if (!heading.isNaN()) {
            addNumberProperty(HEADING_PROPERTY, ((heading % 360) + 360) % 360)
        }
    }

// The plan is re-read from the bridge every poll, so it comes back on its own.
// A flown trail cannot: it is accumulated over time and nothing can replay it.
// Scoped to a composition it was lost on every tab switch, which is the one
// piece of state here that has to outlive the screen showing it.
object VehicleTrail {
    val track = VehicleTrack()
}

class VehicleTrack(private val limit: Int = MAX_TRAIL_POINTS) {
    private val points = ArrayDeque<TrackPoint>()

    fun add(latitude: Double, longitude: Double): Boolean {
        if (!isPlottable(latitude, longitude)) {
            return false
        }
        val point = TrackPoint(latitude, longitude)
        val last = points.lastOrNull()
        if (last == point) {
            return false
        }
        if (last != null && isJump(last, point)) {
            points.clear()
        }
        points.addLast(point)
        while (points.size > limit) {
            points.removeFirst()
        }
        return true
    }

    private fun isJump(from: TrackPoint, to: TrackPoint): Boolean =
        kotlin.math.abs(from.latitude - to.latitude) > TRAIL_BREAK_DEGREES ||
            kotlin.math.abs(from.longitude - to.longitude) > TRAIL_BREAK_DEGREES

    fun points(): List<TrackPoint> = points.toList()

    val size: Int get() = points.size
}

@Composable
fun VehicleMap(
    modifier: Modifier = Modifier,
    mapStyle: String = OSM_RASTER_STYLE,
    follow: Boolean = true,
    missionItems: List<MissionItem> = emptyList(),
    fencePolygons: List<FencePolygon> = emptyList(),
    fenceCircles: List<FenceCircle> = emptyList(),
    rallyPoints: List<RallyPoint> = emptyList(),
    surveys: List<Survey> = emptyList(),
    editable: Boolean = false,
    selectedWaypoint: Int? = null,
    onAdd: (Double, Double) -> Unit = { _, _ -> },
    onMove: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    onWaypointSelected: (MapHit?) -> Unit = {},
    onCentreChanged: (TrackPoint, Double) -> Unit = { _, _ -> },
    bottomInsetPx: Int = 0,
    fitRequest: Int = 0,
    onFitFailed: () -> Unit = {},
) {
    val latitude by mapDouble("vehicle.latitude")
    val longitude by mapDouble("vehicle.longitude")
    val heading by mapDouble("vehicle.heading")
    val home by mapCoordinate("vehicle.homePosition")

    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    val track = VehicleTrail.track

    val context = androidx.compose.ui.platform.LocalContext.current
    val mapView = remember {
        MapLibre.getInstance(context)
        MapView(context)
    }

    // A MapView holds native resources and frees them only on onDestroy. As its
    // own activity that always arrived. Hosted as a tab the composable can leave
    // composition while the activity lives on, so leaving has to wind the view
    // down itself or every visit to the tab strands a map.
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, mapView) {
        var started = false
        var resumed = false
        var destroyed = false
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_CREATE -> mapView.onCreate(null)
                Lifecycle.Event.ON_START -> { mapView.onStart(); started = true }
                Lifecycle.Event.ON_RESUME -> { mapView.onResume(); resumed = true }
                Lifecycle.Event.ON_PAUSE -> { mapView.onPause(); resumed = false }
                Lifecycle.Event.ON_STOP -> { mapView.onStop(); started = false }
                Lifecycle.Event.ON_DESTROY -> { mapView.onDestroy(); destroyed = true }
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            if (!destroyed) {
                if (resumed) mapView.onPause()
                if (started) mapView.onStop()
                mapView.onDestroy()
            }
        }
    }

    DisposableEffect(mapView, mapStyle) {
        mapView.getMapAsync { loaded ->
            map = loaded
            // Camera idle only fires once the map moves, so report where it
            // already is too, or nothing can be placed until the user pans.
            fun reportCentre() {
                val target = loaded.cameraPosition.target ?: return
                onCentreChanged(TrackPoint(target.latitude, target.longitude), loaded.cameraPosition.zoom)
            }
            reportCentre()
            loaded.addOnCameraIdleListener { reportCentre() }
            val builder = if (mapStyle.trimStart().startsWith("{")) {
                Style.Builder().fromJson(mapStyle)
            } else {
                Style.Builder().fromUri(mapStyle)
            }
            loaded.setStyle(builder) { loadedStyle ->
                installLayers(loadedStyle)
                installSurveyLayers(loadedStyle)
                installFenceLayers(loadedStyle)
                installMissionLayers(loadedStyle)
                installFenceHandleLayer(loadedStyle)
                installVehicleLayer(loadedStyle)
                if (editable) {
                    attachMissionEditing(
                        mapView, loaded, loadedStyle,
                        onAdd = onAdd,
                        onMove = onMove,
                        onSelected = onWaypointSelected,
                    )
                }
                style = loadedStyle
            }
        }
        onDispose { }
    }

    LaunchedEffect(style, latitude, longitude) {
        val currentStyle = style ?: return@LaunchedEffect
        if (!track.add(latitude, longitude)) return@LaunchedEffect

        (currentStyle.getSource(VEHICLE_SOURCE) as? GeoJsonSource)
            ?.setGeoJson(vehicleFeature(latitude, longitude, heading))

        (currentStyle.getSource(HOME_SOURCE) as? GeoJsonSource)?.setGeoJson(
            home?.let { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) }
                ?.let { FeatureCollection.fromFeatures(listOf(it)) }
                ?: FeatureCollection.fromFeatures(emptyList()),
        )

        if (track.size >= 2) {
            val line = LineString.fromLngLats(track.points().map { Point.fromLngLat(it.longitude, it.latitude) })
            (currentStyle.getSource(TRAIL_SOURCE) as? GeoJsonSource)
                ?.setGeoJson(Feature.fromGeometry(line))
        }

        if (follow) {
            map?.cameraPosition = CameraPosition.Builder()
                .target(LatLng(latitude, longitude))
                .zoom(map?.cameraPosition?.zoom?.takeIf { it > 1.0 } ?: DEFAULT_ZOOM)
                .build()
        }
    }

    // MapLibre's logo and attribution sit at the bottom left, under the control
    // panel, and attribution is a licence condition rather than decoration. The
    // panel measures itself after the map loads, so this follows its height
    // instead of reading it once while it is still zero.
    LaunchedEffect(map, bottomInsetPx) {
        val settings = map?.uiSettings ?: return@LaunchedEffect
        settings.setLogoMargins(LOGO_EDGE_MARGIN_PX, 0, 0, bottomInsetPx + LOGO_EDGE_MARGIN_PX)
        settings.setAttributionMargins(LOGO_EDGE_MARGIN_PX, 0, 0, bottomInsetPx + LOGO_EDGE_MARGIN_PX)
    }

    LaunchedEffect(fitRequest) {
        if (fitRequest == 0) return@LaunchedEffect
        val currentMap = map ?: return@LaunchedEffect
        val bounds = planBounds(
            planPoints(missionItems, fencePolygons, fenceCircles, rallyPoints, surveys),
        ) ?: return@LaunchedEffect onFitFailed()

        // A plan of one point has no extent, so bounds would be a zero-sized box
        // that MapLibre cannot frame. Centring on it at a sane zoom is the fit.
        if (bounds.spanDegrees < MIN_FIT_SPAN_DEGREES) {
            currentMap.animateCamera(
                CameraUpdateFactory.newLatLngZoom(
                    LatLng(bounds.centre.latitude, bounds.centre.longitude),
                    DEFAULT_ZOOM,
                ),
            )
            return@LaunchedEffect
        }

        currentMap.animateCamera(
            CameraUpdateFactory.newLatLngBounds(
                LatLngBounds.from(bounds.north, bounds.east, bounds.south, bounds.west),
                FIT_PADDING_PIXELS,
            ),
        )
    }

    LaunchedEffect(style, missionItems, fencePolygons, fenceCircles, rallyPoints, surveys, selectedWaypoint) {
        val currentStyle = style ?: return@LaunchedEffect
        renderSurveys(currentStyle, surveys)
        renderFences(currentStyle, fencePolygons, rallyPoints, circlesAsPolygons(fenceCircles))
        renderVertexHandles(currentStyle, fencePolygons, surveys, fenceCircles)
        renderMission(currentStyle, missionItems, selectedWaypoint)
    }

    AndroidView(factory = { mapView }, modifier = modifier)
}

private fun installLayers(style: Style) {
    if (style.getSource(TRAIL_SOURCE) == null) {
        style.addSource(GeoJsonSource(TRAIL_SOURCE))
        style.addLayer(
            LineLayer(TRAIL_LAYER, TRAIL_SOURCE).withProperties(
                PropertyFactory.lineColor("#4FC3F7"),
                PropertyFactory.lineWidth(3f),
            ),
        )
    }

    if (style.getSource(HOME_SOURCE) == null) {
        style.addSource(GeoJsonSource(HOME_SOURCE))
        style.addLayer(
            CircleLayer(HOME_LAYER, HOME_SOURCE).withProperties(
                PropertyFactory.circleColor("#43A047"),
                PropertyFactory.circleRadius(12f),
                PropertyFactory.circleStrokeColor("#FFFFFF"),
                PropertyFactory.circleStrokeWidth(2f),
            ),
        )
        style.addLayer(
            SymbolLayer(HOME_LABEL_LAYER, HOME_SOURCE).withProperties(
                PropertyFactory.textField("H"),
                PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
                PropertyFactory.textSize(14f),
                PropertyFactory.textColor("#FFFFFF"),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }

}

// The aircraft is installed after every plan layer so nothing can bury it.
// Waypoints, rally points and home all land on the same spot as the vehicle
// when a plan is built where it stands, and the one marker that must stay
// visible is the one showing where the aircraft actually is.
private fun installVehicleLayer(style: Style) {
    if (style.getSource(VEHICLE_SOURCE) == null) {
        style.addSource(GeoJsonSource(VEHICLE_SOURCE))
        style.addLayer(
            CircleLayer(VEHICLE_LAYER, VEHICLE_SOURCE).withProperties(
                PropertyFactory.circleColor("#E53935"),
                PropertyFactory.circleRadius(9f),
                PropertyFactory.circleStrokeColor("#FFFFFF"),
                PropertyFactory.circleStrokeWidth(2f),
            ),
        )
        style.addImage(VEHICLE_ARROW_IMAGE, headingArrow())
        style.addLayer(
            SymbolLayer(VEHICLE_HEADING_LAYER, VEHICLE_SOURCE).withProperties(
                PropertyFactory.iconImage(VEHICLE_ARROW_IMAGE),
                PropertyFactory.iconRotate(Expression.get(HEADING_PROPERTY)),
                PropertyFactory.iconRotationAlignment(Property.ICON_ROTATION_ALIGNMENT_MAP),
                PropertyFactory.iconAllowOverlap(true),
                PropertyFactory.iconIgnorePlacement(true),
            ).withFilter(Expression.has(HEADING_PROPERTY)),
        )
    }
}

// A triangle pointing north, so iconRotate can read as a compass bearing.
private fun headingArrow(): Bitmap {
    val size = 48
    val bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
    val canvas = Canvas(bitmap)
    val path = Path().apply {
        moveTo(size / 2f, 2f)
        lineTo(size - 8f, size - 6f)
        lineTo(size / 2f, size * 0.72f)
        lineTo(8f, size - 6f)
        close()
    }
    canvas.drawPath(path, Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = android.graphics.Color.WHITE
        style = Paint.Style.STROKE
        strokeWidth = 6f
        strokeJoin = Paint.Join.ROUND
    })
    canvas.drawPath(path, Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = android.graphics.Color.parseColor("#E53935")
    })
    return bitmap
}
