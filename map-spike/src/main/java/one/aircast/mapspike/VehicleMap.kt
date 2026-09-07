package one.aircast.mapspike

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import org.maplibre.android.MapLibre
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

private const val VEHICLE_SOURCE = "aircast-vehicle"
private const val VEHICLE_LAYER = "aircast-vehicle-layer"
private const val TRAIL_SOURCE = "aircast-trail"
private const val TRAIL_LAYER = "aircast-trail-layer"

private const val DEFAULT_ZOOM = 16.0
private const val MAX_TRAIL_POINTS = 500

// No aircraft covers this between samples. A jump this large means the active
// vehicle changed, and joining the two tracks draws a line across the world.
const val TRAIL_BREAK_DEGREES = 0.5

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
    rallyPoints: List<RallyPoint> = emptyList(),
    surveys: List<Survey> = emptyList(),
    editable: Boolean = false,
    onAdd: (Double, Double) -> Unit = { _, _ -> },
    onMove: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    onWaypointSelected: (MapHit?) -> Unit = {},
) {
    val latitude by mapDouble("vehicle.latitude")
    val longitude by mapDouble("vehicle.longitude")

    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    val track = remember { VehicleTrack() }

    val context = androidx.compose.ui.platform.LocalContext.current
    val mapView = remember {
        MapLibre.getInstance(context)
        MapView(context)
    }

    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, mapView) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_CREATE -> mapView.onCreate(null)
                Lifecycle.Event.ON_START -> mapView.onStart()
                Lifecycle.Event.ON_RESUME -> mapView.onResume()
                Lifecycle.Event.ON_PAUSE -> mapView.onPause()
                Lifecycle.Event.ON_STOP -> mapView.onStop()
                Lifecycle.Event.ON_DESTROY -> mapView.onDestroy()
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    DisposableEffect(mapView, mapStyle) {
        mapView.getMapAsync { loaded ->
            map = loaded
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
            ?.setGeoJson(Feature.fromGeometry(Point.fromLngLat(longitude, latitude)))

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

    LaunchedEffect(style, missionItems, fencePolygons, rallyPoints, surveys) {
        val currentStyle = style ?: return@LaunchedEffect
        renderSurveys(currentStyle, surveys)
        renderFences(currentStyle, fencePolygons, rallyPoints)
        renderFenceHandles(currentStyle, fencePolygons)
        renderMission(currentStyle, missionItems)
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
    }
}
