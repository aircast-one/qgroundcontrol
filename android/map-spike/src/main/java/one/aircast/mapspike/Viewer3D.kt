package one.aircast.mapspike

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.IntOffset
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
import org.maplibre.android.style.layers.FillExtrusionLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon

const val VIEWER3D_VIEW = "view.viewer3d"
const val VIEWER3D_PATH = "view.viewer3dPath"
const val VIEWER3D_VEHICLE = "view.viewer3dVehicle"
private const val V3D_FLOATING_SOURCE = "viewer3d-floating"
private const val V3D_FLOATING_LAYER = "viewer3d-floating-layer"
private const val V3D_VEHICLE_SOURCE = "viewer3d-vehicle"
private const val V3D_VEHICLE_LAYER = "viewer3d-vehicle-layer"
private const val RIBBON_MAX_PIECES = 64
private const val METRES_PER_DEGREE = 111_320.0
private const val RIBBON_WIDTH = 1.5
private const val RIBBON_THICKNESS = 1.0
private const val RIBBON_STEP = 8.0
private const val MARKER_SIZE = 3.0
private const val LABEL_LIFT = 3.0
private const val ARM_LENGTH = 3.0
private const val ARM_WIDTH = 0.5
private const val ROTOR_SIZE = 1.6
private const val HUB_SIZE = 1.4
private const val FRONT_ARM_COLOUR = "#E53935"
private const val REAR_ARM_COLOUR = "#ECEFF1"
private const val ROTOR_COLOUR = "#37474F"
private const val HUB_COLOUR = "#90A4AE"

data class Point3D(val lon: Double, val lat: Double, val alt: Double)

data class Slab(val corners: List<Pair<Double, Double>>, val base: Double, val top: Double, val colour: String)

private fun point3d(array: JSONArray?): Point3D? =
    array?.takeIf { it.length() == 3 }?.let { Point3D(it.optDouble(0), it.optDouble(1), it.optDouble(2)) }

private fun box(at: Point3D, size: Double, colour: String, height: Double = size): Slab {
    val dLat = size / 2 / METRES_PER_DEGREE
    val dLon = dLat / kotlin.math.cos(Math.toRadians(at.lat))
    return Slab(listOf(at.lon - dLon to at.lat - dLat, at.lon + dLon to at.lat - dLat, at.lon + dLon to at.lat + dLat, at.lon - dLon to at.lat + dLat), at.alt - height / 2, at.alt + height / 2, colour)
}

internal fun ribbon(from: Point3D, to: Point3D, colour: String, width: Double = RIBBON_WIDTH, thickness: Double = RIBBON_THICKNESS): List<Slab> {
    val scale = kotlin.math.cos(Math.toRadians((from.lat + to.lat) / 2))
    val dx = (to.lon - from.lon) * METRES_PER_DEGREE * scale
    val dy = (to.lat - from.lat) * METRES_PER_DEGREE
    val length = kotlin.math.hypot(dx, dy)
    if (length < width / 2) return listOf(box(from, width, colour, 0.0).copy(base = minOf(from.alt, to.alt) - thickness / 2, top = maxOf(from.alt, to.alt) + thickness / 2))
    val pieces = kotlin.math.ceil(length / RIBBON_STEP).toInt().coerceIn(1, RIBBON_MAX_PIECES)
    val climb = kotlin.math.abs(to.alt - from.alt) / pieces
    val (nx, ny) = -dy / length * width / 2 to dx / length * width / 2
    val toLonLat = { x: Double, y: Double -> from.lon + x / (METRES_PER_DEGREE * scale) to from.lat + y / METRES_PER_DEGREE }
    return (0 until pieces).map { piece ->
        val (t0, t1) = piece.toDouble() / pieces to (piece + 1).toDouble() / pieces
        val alt = from.alt + (to.alt - from.alt) * (t0 + t1) / 2
        val half = maxOf(thickness, climb) / 2
        val (x0, y0, x1, y1) = listOf(dx * t0, dy * t0, dx * t1, dy * t1)
        Slab(listOf(toLonLat(x0 + nx, y0 + ny), toLonLat(x1 + nx, y1 + ny), toLonLat(x1 - nx, y1 - ny), toLonLat(x0 - nx, y0 - ny)), alt - half, alt + half, colour)
    }
}

private fun offset(at: Point3D, east: Double, north: Double): Point3D =
    at.copy(lon = at.lon + east / (METRES_PER_DEGREE * kotlin.math.cos(Math.toRadians(at.lat))), lat = at.lat + north / METRES_PER_DEGREE)

internal fun quadFrame(at: Point3D, heading: Double): List<Slab> =
    listOf(45.0, -45.0, 135.0, -135.0).flatMap { arm ->
        val bearing = Math.toRadians(heading + arm)
        val motor = offset(at, kotlin.math.sin(bearing) * ARM_LENGTH, kotlin.math.cos(bearing) * ARM_LENGTH)
        val colour = if (kotlin.math.abs(arm) < 90) FRONT_ARM_COLOUR else REAR_ARM_COLOUR
        ribbon(at, motor, colour, ARM_WIDTH, ARM_WIDTH) + box(motor.copy(alt = at.alt + ARM_WIDTH), ROTOR_SIZE, ROTOR_COLOUR, ARM_WIDTH / 2)
    } + box(at, HUB_SIZE, HUB_COLOUR, HUB_SIZE / 2)

internal fun pathSlabs(view: JSONObject?): List<Slab> {
    val segments = view?.optJSONArray("segments")
    val markers = view?.optJSONArray("markers")
    val ribbons = (0 until (segments?.length() ?: 0)).mapNotNull { segments?.optJSONObject(it) }.flatMap { segment ->
        val from = point3d(segment.optJSONArray("from"))
        val to = point3d(segment.optJSONArray("to"))
        if (from == null || to == null) emptyList() else ribbon(from, to, segment.optText("colour"))
    }
    val boxes = (0 until (markers?.length() ?: 0)).mapNotNull { markers?.optJSONObject(it) }.mapNotNull { marker ->
        point3d(marker.optJSONArray("at"))?.let { box(it, MARKER_SIZE, marker.optText("colour")) }
    }
    return ribbons + boxes
}

data class Label3D(val at: Point3D, val text: String)

internal fun pathLabels(view: JSONObject?): List<Label3D> {
    val markers = view?.optJSONArray("markers")
    return (0 until (markers?.length() ?: 0)).mapNotNull { markers?.optJSONObject(it) }.mapNotNull { marker ->
        point3d(marker.optJSONArray("at"))?.let { Label3D(it.copy(alt = it.alt + LABEL_LIFT), marker.optText("label")) }
    }.filter { it.text.isNotBlank() }
}

internal fun lifted(ground: Pair<Float, Float>, pixelsPerMetre: Double, tilt: Double, height: Double): Pair<Float, Float> =
    ground.first to (ground.second - height.coerceAtLeast(0.0) * pixelsPerMetre * kotlin.math.sin(Math.toRadians(tilt))).toFloat()

private fun onScreen(map: MapLibreMap, label: Label3D): Pair<Float, Float> {
    val across = Math.toRadians(map.cameraPosition.bearing + 90)
    val ground = map.projection.toScreenLocation(LatLng(label.at.lat, label.at.lon))
    val beside = offset(label.at, kotlin.math.sin(across), kotlin.math.cos(across)).let { map.projection.toScreenLocation(LatLng(it.lat, it.lon)) }
    val pixelsPerMetre = kotlin.math.hypot((beside.x - ground.x).toDouble(), (beside.y - ground.y).toDouble())
    return lifted(ground.x to ground.y, pixelsPerMetre, map.cameraPosition.tilt, label.at.alt)
}

internal fun vehicleSlabs(view: JSONObject?): List<Slab> {
    val vehicles = view?.optJSONArray("vehicles")
    return (0 until (vehicles?.length() ?: 0)).mapNotNull { vehicles?.optJSONObject(it) }.flatMap { vehicle ->
        point3d(vehicle.optJSONArray("at"))?.let { quadFrame(it, vehicle.optDouble("heading", 0.0)) }.orEmpty()
    }
}

internal fun slabFeatures(slabs: List<Slab>): FeatureCollection = FeatureCollection.fromFeatures(
    slabs.map { slab ->
        val ring = (slab.corners + slab.corners.first()).map { (lon, lat) -> Point.fromLngLat(lon, lat) }
        Feature.fromGeometry(Polygon.fromLngLats(listOf(ring))).also {
            it.addNumberProperty("base", slab.base.coerceAtLeast(0.0))
            it.addNumberProperty("top", slab.top.coerceAtLeast(0.1))
            it.addStringProperty("colour", slab.colour)
        }
    },
)
private const val V3D_BUILDING_SOURCE = "viewer3d-buildings"
private const val V3D_BUILDING_LAYER = "viewer3d-buildings-layer"
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
    addSlabLayer(style, V3D_FLOATING_SOURCE, V3D_FLOATING_LAYER)
    addSlabLayer(style, V3D_VEHICLE_SOURCE, V3D_VEHICLE_LAYER)
}

private fun addSlabLayer(style: Style, source: String, layer: String) {
    style.addSource(GeoJsonSource(source))
    style.addLayer(
        FillExtrusionLayer(layer, source).withProperties(
            PropertyFactory.fillExtrusionColor(Expression.get("colour")),
            PropertyFactory.fillExtrusionBase(Expression.get("base")),
            PropertyFactory.fillExtrusionHeight(Expression.get("top")),
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
    val pathJson by mapPath(VIEWER3D_PATH)
    val slabs = remember(pathJson) { pathSlabs(pathJson) }
    val labels = remember(pathJson) { pathLabels(pathJson) }
    var cameraMoves by remember { mutableIntStateOf(0) }
    val vehicleJson by mapPath(VIEWER3D_VEHICLE)
    val frame = remember(vehicleJson) { vehicleSlabs(vehicleJson) }
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
            loaded.addOnCameraMoveListener { cameraMoves++ }
            loaded.addOnCameraIdleListener { cameraMoves++ }
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
    LaunchedEffect(style, slabs) {
        (style?.getSource(V3D_FLOATING_SOURCE) as? GeoJsonSource)?.setGeoJson(slabFeatures(slabs))
    }
    LaunchedEffect(style, frame) {
        (style?.getSource(V3D_VEHICLE_SOURCE) as? GeoJsonSource)?.setGeoJson(slabFeatures(frame))
    }
    LaunchedEffect(map, scene.centre, vehicle != null) {
        val shown = map ?: return@LaunchedEffect
        val target = scene.centre?.let { (lon, lat) -> LatLng(lat, lon) } ?: vehicle?.let { LatLng(it.latitude, it.longitude) } ?: return@LaunchedEffect
        if (framedOn == target || (framedOn != null && scene.centre == null)) return@LaunchedEffect
        framedOn = target
        shown.cameraPosition = CameraPosition.Builder().target(target).zoom(SCENE_ZOOM).tilt(SCENE_PITCH).build()
    }
    val placed = remember(map, style, labels, cameraMoves) { map?.takeIf { style != null }?.let { shown -> labels.map { onScreen(shown, it) to it.text } }.orEmpty() }
    Box(modifier.clipToBounds()) {
        AndroidView(factory = { mapView }, modifier = Modifier.fillMaxSize())
        placed.forEach { (at, text) ->
            Text(text, color = Color.Black, style = MaterialTheme.typography.labelLarge, modifier = Modifier.offset { IntOffset(at.first.toInt(), at.second.toInt()) })
        }
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
