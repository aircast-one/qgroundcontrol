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
private const val OTHER_VEHICLE_COLOUR = "#90A4AE"
private const val VEHICLE_ARROW_IMAGE = "aircast-vehicle-arrow"

const val HEADING_PROPERTY = "heading"
const val ACTIVE_PROPERTY = "active"
const val STALE_PROPERTY = "stale"
private const val TRAIL_SOURCE = "aircast-trail"
private const val HOME_SOURCE = "aircast-home"
private const val HOME_LAYER = "aircast-home-layer"
private const val HOME_LABEL_LAYER = "aircast-home-label-layer"
private const val TRAIL_LAYER = "aircast-trail-layer"

private const val DEFAULT_ZOOM = 16.0

private const val MIN_FIT_SPAN_DEGREES = 1e-5
private const val FIT_PADDING_PIXELS = 80
private const val LOGO_EDGE_MARGIN_PX = 16
private const val STALE_COLOUR = "#9E9E9E"

const val DEMO_STYLE_URL = "https://demotiles.maplibre.org/style.json"

const val OSM_TILE_URL = "https://tile.openstreetmap.org/{z}/{x}/{y}.png"

const val OSM_RASTER_STYLE = """
{
  "version": 8,
  "sources": {
    "osm": {
      "type": "raster",
      "tiles": ["$OSM_TILE_URL"],
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

fun vehicleFeatures(
    latitude: Double,
    longitude: Double,
    heading: Double,
    stale: Boolean = false,
): FeatureCollection =
    if (isPlottable(latitude, longitude)) {
        FeatureCollection.fromFeatures(listOf(vehicleFeature(latitude, longitude, heading, stale)))
    } else {
        FeatureCollection.fromFeatures(emptyList())
    }

fun fleetFeatures(fleet: List<VehicleChoice>): FeatureCollection =
    FeatureCollection.fromFeatures(
        fleet
            .filter { isPlottable(it.latitude, it.longitude) }
            .map { flown ->
                vehicleFeature(
                    latitude = flown.latitude,
                    longitude = flown.longitude,
                    heading = flown.heading,
                    stale = flown.contactLost,
                    active = flown.active,
                )
            },
    )

fun vehicleFeature(
    latitude: Double,
    longitude: Double,
    heading: Double,
    stale: Boolean = false,
    active: Boolean = true,
): Feature =
    Feature.fromGeometry(Point.fromLngLat(longitude, latitude)).apply {
        addBooleanProperty(STALE_PROPERTY, stale)
        addBooleanProperty(ACTIVE_PROPERTY, active)
        if (!heading.isNaN()) {
            addNumberProperty(HEADING_PROPERTY, ((heading % 360) + 360) % 360)
        }
    }

@Composable
fun VehicleMap(
    modifier: Modifier = Modifier,
    mapStyle: String = OSM_RASTER_STYLE,
    follow: Boolean = true,
    missionItems: List<MissionItem> = emptyList(),
    linkStartToHome: Boolean = false,
    fencePolygons: List<FencePolygon> = emptyList(),
    fenceCircles: List<FenceCircle> = emptyList(),
    rallyPoints: List<RallyPoint> = emptyList(),
    operator: TrackPoint? = null,
    surveys: List<Survey> = emptyList(),
    shots: List<TrackPoint> = emptyList(),
    landings: List<LandingPattern> = emptyList(),
    editable: Boolean = false,
    selectedWaypoint: Int? = null,
    firmwareFence: FirmwareFence? = null,
    onAdd: (Double, Double) -> Unit = { _, _ -> },
    onMove: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    onWaypointSelected: (MapHit?) -> Unit = {},
    onMoved: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    onCentreChanged: (TrackPoint, Double) -> Unit = { _, _ -> },
    onViewChanged: (List<TrackPoint>) -> Unit = { },
    bottomInsetPx: Int = 0,
    topInsetPx: Int = 0,
    cameraBottomPx: Int = 0,
    fitRequest: Int = 0,
    onFitFailed: () -> Unit = {},
    centreRequest: Int = 0,
    centreOn: TrackPoint? = null,
) {
    val linkLost by mapViewFlag(FLY_STATE_VIEW, "contactLost")
    val fleetJson by mapPath(VEHICLES_VIEW)
    val fleet = remember(fleetJson) { vehicleChoices(fleetJson).choices }
    // The raw vehicle.* reads answered for the active vehicle only; view.vehicles carries the same
    // position, heading and home for every aircraft, and the active one is read from it here.
    val flown = fleet.firstOrNull { it.active }
    val latitude = flown?.latitude ?: Double.NaN
    val longitude = flown?.longitude ?: Double.NaN
    val heading = flown?.heading ?: Double.NaN
    val home = flown?.home

    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    val trackJson by mapPath(TRACK_VIEW)
    val track = trackReading(trackJson)

    val context = androidx.compose.ui.platform.LocalContext.current
    val mapView = remember {
        MapLibre.getInstance(context)
        MapView(context)
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

    DisposableEffect(mapView, mapStyle) {
        mapView.getMapAsync { loaded ->
            map = loaded
            fun reportCentre() {
                val target = loaded.cameraPosition.target ?: return
                onCentreChanged(TrackPoint(target.latitude, target.longitude), loaded.cameraPosition.zoom)
                val seen = loaded.projection.visibleRegion.latLngBounds
                onViewChanged(
                    listOf(
                        TrackPoint(seen.latitudeNorth, seen.longitudeWest),
                        TrackPoint(seen.latitudeNorth, seen.longitudeEast),
                        TrackPoint(seen.latitudeSouth, seen.longitudeEast),
                        TrackPoint(seen.latitudeSouth, seen.longitudeWest),
                    ),
                )
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
                    installLandingLayers(loadedStyle)
                    installMidpointLayer(loadedStyle)
                installShotLayer(loadedStyle)
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
                        onMoved = onMoved,
                    )
                }
                style = loadedStyle
            }
        }
        onDispose { }
    }

    LaunchedEffect(style, shots) {
        val shotStyle = style ?: return@LaunchedEffect
        (shotStyle.getSource(SHOT_SOURCE) as? GeoJsonSource)?.setGeoJson(shotFeatures(shots))
    }

    LaunchedEffect(style, latitude, longitude, heading, home, linkLost, fleet) {
        val currentStyle = style ?: return@LaunchedEffect

        (currentStyle.getSource(VEHICLE_SOURCE) as? GeoJsonSource)
            ?.setGeoJson(
                when {
                    fleet.isEmpty() -> vehicleFeatures(latitude, longitude, heading, linkLost)
                    else -> fleetFeatures(fleet)
                },
            )

        (currentStyle.getSource(HOME_SOURCE) as? GeoJsonSource)?.setGeoJson(
            home?.let { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) }
                ?.let { FeatureCollection.fromFeatures(listOf(it)) }
                ?: FeatureCollection.fromFeatures(emptyList()),
        )

        (currentStyle.getSource(TRAIL_SOURCE) as? GeoJsonSource)?.setGeoJson(
            when {
                trackDraws(track) -> FeatureCollection.fromFeatures(
                    listOf(
                        Feature.fromGeometry(
                            LineString.fromLngLats(
                                track.points.map { Point.fromLngLat(it.longitude, it.latitude) },
                            ),
                        ),
                    ),
                )
                else -> FeatureCollection.fromFeatures(emptyList())
            },
        )

        if (follow && isPlottable(latitude, longitude)) {
            map?.cameraPosition = CameraPosition.Builder()
                .target(LatLng(latitude, longitude))
                .zoom(map?.cameraPosition?.zoom?.takeIf { it > 1.0 } ?: DEFAULT_ZOOM)
                .build()
        }
    }

    LaunchedEffect(map, cameraBottomPx) {
        map?.setPadding(0, 0, 0, cameraBottomPx)
    }

    LaunchedEffect(map, bottomInsetPx) {
        val settings = map?.uiSettings ?: return@LaunchedEffect
        settings.setLogoMargins(LOGO_EDGE_MARGIN_PX, 0, 0, bottomInsetPx + LOGO_EDGE_MARGIN_PX)
        settings.setAttributionMargins(LOGO_EDGE_MARGIN_PX, 0, 0, bottomInsetPx + LOGO_EDGE_MARGIN_PX)
    }

    LaunchedEffect(centreRequest) {
        if (centreRequest == 0) return@LaunchedEffect
        val at = centreOn?.takeIf { isPlottable(it.latitude, it.longitude) } ?: return@LaunchedEffect
        map?.animateCamera(CameraUpdateFactory.newLatLng(LatLng(at.latitude, at.longitude)))
    }

    LaunchedEffect(fitRequest) {
        if (fitRequest == 0) return@LaunchedEffect
        val currentMap = map ?: return@LaunchedEffect
        val bounds = planBounds(
            fitPoints(
                planPoints(missionItems, fencePolygons, fenceCircles, rallyPoints, surveys),
                latitude,
                longitude,
            ),
        ) ?: return@LaunchedEffect onFitFailed()

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
                FIT_PADDING_PIXELS + topInsetPx,
                FIT_PADDING_PIXELS,
                FIT_PADDING_PIXELS + bottomInsetPx,
            ),
        )
    }

    LaunchedEffect(
        style, missionItems, fencePolygons, fenceCircles, rallyPoints, surveys,
        landings, firmwareFence, selectedWaypoint, linkStartToHome, operator,
    ) {
        val currentStyle = style ?: return@LaunchedEffect
        renderSurveys(currentStyle, surveys)
        renderLandings(currentStyle, landings)
        renderMidpoints(currentStyle, fencePolygons, surveys)
        renderFences(currentStyle, fencePolygons, rallyPoints, circlesAsPolygons(fenceCircles), firmwareFence)
        (currentStyle.getSource(GCS_SOURCE) as? GeoJsonSource)?.setGeoJson(operatorFeatures(operator))
        renderVertexHandles(currentStyle, fencePolygons, surveys, fenceCircles, landings)
        renderMission(currentStyle, missionItems, linkStartToHome, selectedWaypoint)
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

private fun installVehicleLayer(style: Style) {
    if (style.getSource(VEHICLE_SOURCE) == null) {
        style.addSource(GeoJsonSource(VEHICLE_SOURCE))
        style.addLayer(
            CircleLayer(VEHICLE_LAYER, VEHICLE_SOURCE).withProperties(
                PropertyFactory.circleColor(
                    Expression.switchCase(
                        Expression.get(STALE_PROPERTY), Expression.literal(STALE_COLOUR),
                        Expression.not(Expression.get(ACTIVE_PROPERTY)), Expression.literal(OTHER_VEHICLE_COLOUR),
                        Expression.literal("#E53935"),
                    ),
                ),
                PropertyFactory.circleRadius(
                    Expression.switchCase(
                        Expression.get(ACTIVE_PROPERTY), Expression.literal(9f),
                        Expression.literal(6f),
                    ),
                ),
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
                PropertyFactory.iconOpacity(
                    Expression.switchCase(
                        Expression.get(STALE_PROPERTY), Expression.literal(0.4f),
                        Expression.literal(1.0f),
                    ),
                ),
                PropertyFactory.iconAllowOverlap(true),
                PropertyFactory.iconIgnorePlacement(true),
            ).withFilter(Expression.has(HEADING_PROPERTY)),
        )
    }
}

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

private fun renderLandings(style: Style, landings: List<LandingPattern>) {
    (style.getSource(LANDING_PATH_SOURCE) as? GeoJsonSource)?.setGeoJson(landingPathFeatures(landings))
    (style.getSource(LANDING_LOITER_SOURCE) as? GeoJsonSource)?.setGeoJson(landingLoiterFeatures(landings))
}
