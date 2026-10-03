package one.aircast.mapspike

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import org.maplibre.android.MapLibre
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapLibreMapOptions
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.FillExtrusionLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon
import org.mavlink.qgroundcontrol.QGCBridge

const val VIEWER3D_VIEW = "view.viewer3d"
private const val VIEWER3D_MISSION = "view.flyMissionItems(geometry)"
private const val V3D_BUILDING_SOURCE = "viewer3d-buildings"
private const val V3D_BUILDING_LAYER = "viewer3d-buildings-layer"
private const val V3D_MISSION_SOURCE = "viewer3d-mission"
private const val V3D_MISSION_LAYER = "viewer3d-mission-layer"
private const val V3D_VEHICLE_SOURCE = "viewer3d-vehicle"
private const val V3D_VEHICLE_LAYER = "viewer3d-vehicle-layer"
private const val SCENE_PITCH = 60.0
private const val SCENE_ZOOM = 16.0
private const val MISSION_POLL_MS = 1000L

data class Building3D(val outer: List<Pair<Double, Double>>, val inner: List<Pair<Double, Double>>, val height: Double)

data class Scene3D(
    val available: Boolean,
    val reason: String,
    val buildings: List<Building3D>,
    val centre: Pair<Double, Double>?,
)

private fun lonLats(array: JSONArray?): List<Pair<Double, Double>> =
    (0 until (array?.length() ?: 0)).mapNotNull { array?.optJSONArray(it) }.map { it.optDouble(0) to it.optDouble(1) }

fun scene3d(view: JSONObject?): Scene3D {
    val listed = view?.optJSONArray("buildings")
    val bounds = view?.optJSONObject("bounds")
    return Scene3D(
        available = view?.optBoolean("available") == true,
        reason = view?.optText("reason").orEmpty(),
        buildings = (0 until (listed?.length() ?: 0)).mapNotNull { listed?.optJSONObject(it) }.map {
            Building3D(lonLats(it.optJSONArray("outer")), lonLats(it.optJSONArray("inner")), it.optDouble("height", 0.0))
        }.filter { it.outer.size > 2 && it.height > 0.0 },
        centre = bounds?.let { (it.optDouble("west") + it.optDouble("east")) / 2 to (it.optDouble("south") + it.optDouble("north")) / 2 },
    )
}

internal fun buildingFeatures(buildings: List<Building3D>): FeatureCollection = FeatureCollection.fromFeatures(
    buildings.map { building ->
        val ring = { points: List<Pair<Double, Double>> -> points.map { (lon, lat) -> Point.fromLngLat(lon, lat) } }
        val rings = listOf(ring(building.outer)) + listOfNotNull(ring(building.inner).takeIf { it.size > 2 })
        Feature.fromGeometry(Polygon.fromLngLats(rings)).also { it.addNumberProperty("height", building.height) }
    },
)

private fun installScene(style: Style) {
    style.addSource(GeoJsonSource(V3D_BUILDING_SOURCE))
    style.addLayer(
        FillExtrusionLayer(V3D_BUILDING_LAYER, V3D_BUILDING_SOURCE).withProperties(
            PropertyFactory.fillExtrusionColor("#B0BEC5"),
            PropertyFactory.fillExtrusionHeight(Expression.get("height")),
            PropertyFactory.fillExtrusionBase(0f),
            PropertyFactory.fillExtrusionOpacity(0.85f),
        ),
    )
    style.addSource(GeoJsonSource(V3D_MISSION_SOURCE))
    style.addLayer(LineLayer(V3D_MISSION_LAYER, V3D_MISSION_SOURCE).withProperties(PropertyFactory.lineColor("#FFD54F"), PropertyFactory.lineWidth(3f)))
    style.addSource(GeoJsonSource(V3D_VEHICLE_SOURCE))
    style.addLayer(
        CircleLayer(V3D_VEHICLE_LAYER, V3D_VEHICLE_SOURCE).withProperties(
            PropertyFactory.circleColor("#E53935"),
            PropertyFactory.circleRadius(8f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
            PropertyFactory.circlePitchAlignment("map"),
        ),
    )
}

@Composable
fun Viewer3DPane(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val viewJson by mapPath(VIEWER3D_VIEW)
    val scene = remember(viewJson) { scene3d(viewJson) }
    val vehiclesJson by mapPath(VEHICLES_VIEW)
    val vehicle = remember(vehiclesJson) { vehicleChoices(vehiclesJson).choices.firstOrNull { it.active && isPlottable(it.latitude, it.longitude) } }
    var mission by remember { mutableStateOf(emptyList<MissionItem>()) }
    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    var framed by remember { mutableStateOf(false) }
    val mapView = remember {
        MapLibre.getInstance(context)
        MapView(context, MapLibreMapOptions.createFromAttributes(context).textureMode(true))
    }
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
    DisposableEffect(mapView) {
        mapView.getMapAsync { loaded ->
            map = loaded
            loaded.uiSettings.isRotateGesturesEnabled = true
            loaded.uiSettings.isTiltGesturesEnabled = true
            val mapStyle = planMapStyle(context)
            val builder = if (mapStyle.trimStart().startsWith("{")) Style.Builder().fromJson(mapStyle) else Style.Builder().fromUri(mapStyle)
            loaded.setStyle(builder) { ready ->
                installScene(ready)
                style = ready
            }
        }
        onDispose { }
    }
    LaunchedEffect(Unit) {
        while (isActive) {
            mission = withContext(Dispatchers.Default) { missionItems(runCatching { JSONObject(QGCBridge.get(VIEWER3D_MISSION)) }.getOrNull()) }
            delay(MISSION_POLL_MS)
        }
    }
    LaunchedEffect(style, scene) {
        (style?.getSource(V3D_BUILDING_SOURCE) as? GeoJsonSource)?.setGeoJson(buildingFeatures(scene.buildings))
    }
    LaunchedEffect(style, mission) {
        val points = mission.map { Point.fromLngLat(it.longitude, it.latitude) }
        (style?.getSource(V3D_MISSION_SOURCE) as? GeoJsonSource)?.setGeoJson(
            FeatureCollection.fromFeatures(listOfNotNull(points.takeIf { it.size > 1 }?.let { Feature.fromGeometry(LineString.fromLngLats(it)) })),
        )
    }
    LaunchedEffect(style, vehicle) {
        (style?.getSource(V3D_VEHICLE_SOURCE) as? GeoJsonSource)?.setGeoJson(
            FeatureCollection.fromFeatures(listOfNotNull(vehicle?.let { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) })),
        )
    }
    LaunchedEffect(map, scene.centre, vehicle != null) {
        val shown = map ?: return@LaunchedEffect
        val target = scene.centre?.let { (lon, lat) -> LatLng(lat, lon) } ?: vehicle?.let { LatLng(it.latitude, it.longitude) } ?: return@LaunchedEffect
        if (framed) return@LaunchedEffect
        framed = true
        shown.cameraPosition = CameraPosition.Builder().target(target).zoom(SCENE_ZOOM).tilt(SCENE_PITCH).build()
    }
    Box(modifier) {
        AndroidView(factory = { mapView }, modifier = Modifier.fillMaxSize())
        if (!scene.available && scene.reason.isNotBlank()) {
            Text(
                scene.reason,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.align(Alignment.Center).padding(24.dp),
            )
        }
    }
}
