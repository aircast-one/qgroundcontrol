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

data class Building3D(val outer: List<List<Pair<Double, Double>>>, val inner: List<List<Pair<Double, Double>>>, val height: Double)

data class Scene3D(
    val available: Boolean,
    val reason: String,
    val buildings: List<Building3D>,
    val centre: Pair<Double, Double>?,
)

private fun lonLats(array: JSONArray?): List<Pair<Double, Double>> =
    (0 until (array?.length() ?: 0)).mapNotNull { array?.optJSONArray(it) }.map { it.optDouble(0) to it.optDouble(1) }

private fun rings(array: JSONArray?): List<List<Pair<Double, Double>>> =
    (0 until (array?.length() ?: 0)).mapNotNull { array?.optJSONArray(it) }.map(::lonLats).filter { it.size > 2 }

internal fun signedArea(ring: List<Pair<Double, Double>>): Double =
    ring.zip(ring.drop(1) + ring.take(1)).sumOf { (a, b) -> a.first * b.second - b.first * a.second } / 2.0

internal fun wound(ring: List<Pair<Double, Double>>, counterClockwise: Boolean): List<Pair<Double, Double>> {
    val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
    return if ((signedArea(closed) > 0) == counterClockwise) closed else closed.reversed()
}

fun scene3d(view: JSONObject?): Scene3D {
    val listed = view?.optJSONArray("buildings")
    val bounds = view?.optJSONObject("bounds")
    return Scene3D(
        available = view?.optBoolean("available") == true,
        reason = view?.optText("reason").orEmpty(),
        buildings = (0 until (listed?.length() ?: 0)).mapNotNull { listed?.optJSONObject(it) }.map {
            Building3D(rings(it.optJSONArray("outer")), rings(it.optJSONArray("inner")), it.optDouble("height", 0.0))
        }.filter { it.outer.isNotEmpty() && it.height > 0.0 },
        centre = bounds?.let { (it.optDouble("west") + it.optDouble("east")) / 2 to (it.optDouble("south") + it.optDouble("north")) / 2 },
    )
}

internal fun buildingFeatures(buildings: List<Building3D>): FeatureCollection = FeatureCollection.fromFeatures(
    buildings.map { building ->
        val points = { ring: List<Pair<Double, Double>> -> ring.map { (lon, lat) -> Point.fromLngLat(lon, lat) } }
        val outers = building.outer.map { points(wound(it, counterClockwise = true)) }
        val holes = building.inner.map { points(wound(it, counterClockwise = false)) }
        val geometry = when (outers.size) {
            1 -> Polygon.fromLngLats(outers + holes)
            else -> org.maplibre.geojson.MultiPolygon.fromLngLats(outers.map { listOf(it) })
        }
        Feature.fromGeometry(geometry).also { it.addNumberProperty("height", building.height) }
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
    val missionJson by mapPath(VIEWER3D_MISSION)
    val mission = remember(missionJson) { missionItems(missionJson) }
    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    var framedOn by remember { mutableStateOf<LatLng?>(null) }
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
        if (framedOn == target || (framedOn != null && scene.centre == null)) return@LaunchedEffect
        framedOn = target
        shown.cameraPosition = CameraPosition.Builder().target(target).zoom(SCENE_ZOOM).tilt(SCENE_PITCH).build()
    }
    Box(modifier) {
        AndroidView(factory = { mapView }, modifier = Modifier.fillMaxSize())
        if (!scene.available && scene.reason.isNotBlank()) {
            androidx.compose.material3.Surface(
                shape = MaterialTheme.shapes.medium,
                color = MaterialTheme.colorScheme.surfaceContainerHigh,
                modifier = Modifier.align(Alignment.Center).padding(24.dp),
            ) {
                Text(scene.reason, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(16.dp))
            }
        }
    }
}
