package one.aircast.map

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import android.os.SystemClock
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.foundation.Canvas as ComposeCanvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.text.rememberTextMeasurer
import org.maplibre.android.MapLibre
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.camera.CameraUpdateFactory
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.geometry.LatLngBounds
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapLibreMapOptions
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.layers.TransitionOptions
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

private const val VEHICLE_SOURCE = "aircast-vehicle"
private const val TAP_SLOP_DP = 24f

internal val NO_FADES = TransitionOptions(0, 0, false)
private const val VEHICLE_LAYER = "aircast-vehicle-layer"
private const val VEHICLE_HEADING_LAYER = "aircast-vehicle-heading-layer"
private const val OTHER_VEHICLE_COLOUR = "#90A4AE"
private const val VEHICLE_ARROW_IMAGE = "aircast-vehicle-arrow"

const val HEADING_PROPERTY = "heading"
const val ACTIVE_PROPERTY = "active"
const val STALE_PROPERTY = "stale"
const val VEHICLE_LABEL_PROPERTY = "vehicleLabel"
private const val VEHICLE_LABEL_LAYER = "aircast-vehicle-label"
private const val TRAIL_SOURCE = "aircast-trail"
private const val HOME_SOURCE = "aircast-home"
private const val HOME_LAYER = "aircast-home-layer"
private const val HOME_LABEL_LAYER = "aircast-home-label-layer"
private const val TRAIL_LAYER = "aircast-trail-layer"

private const val DEFAULT_ZOOM = 17.0

private const val MIN_FIT_SPAN_DEGREES = 1e-5
private const val FIT_PADDING_PIXELS = 80
private const val LOGO_EDGE_MARGIN_PX = 16
private val ATTRIBUTION_CLEARANCE = 24.dp
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

object FlightMapPosition {
    @Volatile
    var latest: TrackPoint? = null

    @Volatile
    var operatorCentred: Boolean = false
}

fun isPlottable(latitude: Double, longitude: Double): Boolean =
    !latitude.isNaN() && !longitude.isNaN() &&
        !(latitude == 0.0 && longitude == 0.0) &&
        latitude in -90.0..90.0 && longitude in -180.0..180.0

fun vehicleFeatures(
    latitude: Double,
    longitude: Double,
    heading: Double,
    stale: Boolean = false,
    lastSeen: String? = null,
): FeatureCollection =
    if (isPlottable(latitude, longitude)) {
        FeatureCollection.fromFeatures(listOf(vehicleFeature(latitude, longitude, heading, stale, label = lastSeen)))
    } else {
        FeatureCollection.fromFeatures(emptyList())
    }

fun fleetFeatures(fleet: List<VehicleChoice>, lastSeen: String? = null): FeatureCollection =
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
                    label = listOfNotNull(
                        "Vehicle ${flown.id}".takeIf { fleet.size > 1 },
                        lastSeen.takeIf { flown.active },
                    ).joinToString("\n").ifEmpty { null },
                )
            },
    )

fun vehicleFeature(
    latitude: Double,
    longitude: Double,
    heading: Double,
    stale: Boolean = false,
    active: Boolean = true,
    label: String? = null,
): Feature =
    Feature.fromGeometry(Point.fromLngLat(longitude, latitude)).apply {
        addBooleanProperty(STALE_PROPERTY, stale)
        addBooleanProperty(ACTIVE_PROPERTY, active)
        label?.let { addStringProperty(VEHICLE_LABEL_PROPERTY, it) }
        if (!heading.isNaN()) {
            addNumberProperty(HEADING_PROPERTY, ((heading % 360) + 360) % 360)
        }
    }

@Composable
fun VehicleMap(
    modifier: Modifier = Modifier,
    mapStyle: String = OSM_RASTER_STYLE,
    follow: Boolean = true,
    keepCentered: Boolean = true,
    missionItems: List<MissionItem> = emptyList(),
    linkStartToHome: Boolean = false,
    fencePolygons: List<FencePolygon> = emptyList(),
    fenceCircles: List<FenceCircle> = emptyList(),
    rallyPoints: List<RallyPoint> = emptyList(),
    operator: TrackPoint? = null,
    operatorHeading: Double = Double.NaN,
    surveys: List<Survey> = emptyList(),
    shots: List<TrackPoint> = emptyList(),
    landings: List<LandingPattern> = emptyList(),
    editable: Boolean = false,
    selectedWaypoint: Int? = null,
    circledShapes: Set<String> = emptySet(),
    firmwareFence: FirmwareFence? = null,
    onAdd: (Double, Double) -> Unit = { _, _ -> },
    onMove: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    onWaypointSelected: (MapHit?) -> Unit = {},
    onMoved: (MapHit, Double, Double) -> Unit = { _, _, _ -> },
    canDrag: (MapHit) -> Boolean = { true },
    onCentreChanged: (TrackPoint, Double) -> Unit = { _, _ -> },
    onViewChanged: (List<TrackPoint>) -> Unit = { },
    bottomInsetPx: Int = 0,
    topInsetPx: Int = 0,
    leftInsetPx: Int = 0,
    logoEndInsetPx: Int? = null,
    cameraBottomPx: Int = 0,
    pip: Boolean = false,
    fitRequest: Int = 0,
    fitOnly: List<TrackPoint>? = null,
    onFitFailed: () -> Unit = {},
    centreRequest: Int = 0,
    centreOn: TrackPoint? = null,
    centreZoom: Double? = null,
    gestures: Boolean = true,
    onMapClick: ((Double, Double) -> Unit)? = null,
    onMissionItemClick: ((Int) -> Unit)? = null,
    traffic: List<TrafficMark> = emptyList(),
    onTrafficClick: (() -> Unit)? = null,
    gimbals: List<GimbalAzimuth> = emptyList(),
    breachReturn: TrackPoint? = null,
    proximityRadar: Boolean = false,
    obstacleOverlay: Boolean = false,
    tracePoints: List<TrackPoint> = emptyList(),
    traceLine: Boolean = false,
    roi: TrackPoint? = null,
    onRoiClick: ((TrackPoint) -> Unit)? = null,
    goto: GotoLocation? = null,
    clickMarker: TrackPoint? = null,
    collisionLegs: List<Pair<TrackPoint, TrackPoint>> = emptyList(),
    orbit: OrbitCircle? = null,
    otherMissions: List<OtherMission> = emptyList(),
) {
    val mapEdits = one.aircast.map.LocalFlyMapEdits.current
    val latestMapClick by rememberUpdatedState(onMapClick)
    val latestItemClick by rememberUpdatedState(onMissionItemClick)
    val latestRoi by rememberUpdatedState(roi)
    val latestRoiClick by rememberUpdatedState(onRoiClick)
    val latestTrafficClick by rememberUpdatedState(onTrafficClick)
    val shownGoto = editedGoto(goto, mapEdits.gotoLoiter)
    val gotoEditing = mapEdits.gotoLoiter != null && shownGoto?.loiterRadiusMetres != null
    val latestGoto by rememberUpdatedState(shownGoto)
    val latestItems by rememberUpdatedState(missionItems)
    val latestOnAdd by rememberUpdatedState(onAdd)
    val latestOnMove by rememberUpdatedState(onMove)
    val latestOnSelected by rememberUpdatedState(onWaypointSelected)
    val latestOnMoved by rememberUpdatedState(onMoved)
    val latestCanDrag by rememberUpdatedState(canDrag)
    val linkLost by mapViewFlag(FLY_STATE_VIEW, "contactLost")
    val lastSeen = silentSeconds(linkLost)?.let(::lastSeenText)
    val fleetJson by mapPath(VEHICLES_VIEW)
    val fleet = remember(fleetJson) { vehicleChoices(fleetJson).choices }
    val flown = fleet.firstOrNull { it.active }
    val latitude = flown?.latitude ?: Double.NaN
    val longitude = flown?.longitude ?: Double.NaN
    val heading = flown?.heading ?: Double.NaN
    val home = flown?.home

    var map by remember { mutableStateOf<MapLibreMap?>(null) }
    var style by remember { mutableStateOf<Style?>(null) }
    var draggingVertex by remember { mutableStateOf<MapHit?>(null) }
    var panning by remember { mutableStateOf(false) }
    var cameraMoves by remember { mutableIntStateOf(0) }
    var trackingResumesAtMs by remember { mutableLongStateOf(0L) }
    val trackJson by mapPath(TRACK_TAIL_VIEW)
    var track by remember { mutableStateOf(trackReading(null)) }
    LaunchedEffect(trackJson) {
        val tail = trackReading(trackJson)
        track = mergedTrack(track, tail)
            ?: withContext(Dispatchers.Default) { MapBridge.read(TRACK_VIEW) }?.let(::trackReading)
            ?: tail.copy(points = emptyList(), from = 0)
    }

    val context = androidx.compose.ui.platform.LocalContext.current
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

    val latestCentreChanged by rememberUpdatedState(onCentreChanged)
    val latestViewChanged by rememberUpdatedState(onViewChanged)
    DisposableEffect(mapView, mapStyle) {
        mapView.getMapAsync { loaded ->
            map = loaded
            fun reportCentre() {
                val target = loaded.cameraPosition.target ?: return
                latestCentreChanged(TrackPoint(target.latitude, target.longitude), loaded.cameraPosition.zoom)
                val seen = loaded.projection.visibleRegion.latLngBounds
                latestViewChanged(
                    listOf(
                        TrackPoint(seen.latitudeNorth, seen.longitudeWest),
                        TrackPoint(seen.latitudeNorth, seen.longitudeEast),
                        TrackPoint(seen.latitudeSouth, seen.longitudeEast),
                        TrackPoint(seen.latitudeSouth, seen.longitudeWest),
                    ),
                )
            }
            reportCentre()
            loaded.addOnCameraIdleListener {
                reportCentre()
                if (panning) {
                    panning = false
                    trackingResumesAtMs = SystemClock.elapsedRealtime() + PAN_RECENTER_DELAY_MS
                }
            }
            loaded.addOnCameraMoveListener { cameraMoves++ }
            loaded.addOnCameraMoveStartedListener { reason ->
                if (reason == MapLibreMap.OnCameraMoveStartedListener.REASON_API_GESTURE) panning = true
            }
            val builder = if (mapStyle.trimStart().startsWith("{")) {
                Style.Builder().fromJson(mapStyle)
            } else {
                Style.Builder().fromUri(mapStyle)
            }
            loaded.setStyle(builder) { loadedStyle ->
                loadedStyle.transition = NO_FADES
                installLayers(loadedStyle)
                installSurveyLayers(loadedStyle)
                installTransectMarks(loadedStyle)
                    installLandingLayers(loadedStyle)
                    installMidpointLayer(loadedStyle)
                installShotLayer(loadedStyle)
                installFenceLayers(loadedStyle)
                installMissionLayers(loadedStyle)
                installTraceLayer(loadedStyle)
                installFenceHandleLayer(loadedStyle)
                installVehicleLayer(loadedStyle)
                installTrafficLayer(loadedStyle)
                installRoiLayer(loadedStyle)
                installGotoLayer(loadedStyle)
                installClickMarker(loadedStyle)
                installOrbitLayer(loadedStyle)
                if (!editable && onMapClick != null) {
                    loaded.addOnMapClickListener { at ->
                        val screen = loaded.projection.toScreenLocation(at)
                        val slop = TAP_SLOP_DP * mapView.resources.displayMetrics.density
                        val near = android.graphics.RectF(screen.x - slop, screen.y - slop, screen.x + slop, screen.y + slop)
                        val trafficClick = latestTrafficClick
                        if (trafficClick != null && loaded.queryRenderedFeatures(near, TRAFFIC_LAYER).isNotEmpty()) {
                            trafficClick()
                            return@addOnMapClickListener true
                        }
                        val roiTapped = latestRoi?.takeIf {
                            loaded.queryRenderedFeatures(near, ROI_LAYER).isNotEmpty()
                        }
                        if (roiTapped != null && latestRoiClick != null) {
                            latestRoiClick?.invoke(roiTapped)
                            return@addOnMapClickListener true
                        }
                        val item = (hitTest(loaded, screen.x, screen.y) as? MapHit.Waypoint)
                            ?.let { hit -> latestItems.firstOrNull { it.index == hit.index } }
                        val itemClick = latestItemClick
                        if (item != null && itemClick != null) itemClick(item.sequence)
                        item != null && itemClick != null
                    }
                    loaded.addOnMapLongClickListener { at ->
                        latestMapClick?.invoke(at.latitude, at.longitude)
                        latestMapClick != null
                    }
                    attachGotoRadiusDrag(mapView, loaded, mapEdits) { latestGoto }
                }
                if (editable) {
                    attachMissionEditing(
                        mapView, loaded, loadedStyle,
                        onAdd = { latitude, longitude -> latestOnAdd(latitude, longitude) },
                        onMove = { hit, latitude, longitude -> latestOnMove(hit, latitude, longitude) },
                        onSelected = { hit -> latestOnSelected(hit) },
                        onMoved = { hit, latitude, longitude -> latestOnMoved(hit, latitude, longitude) },
                        onDragging = { draggingVertex = it },
                        canDrag = { hit -> latestCanDrag(hit) },
                    )
                }
                draggingVertex = null
                style = loadedStyle
            }
        }
        onDispose { }
    }

    LaunchedEffect(style, draggingVertex, fencePolygons, surveys) {
        val edgeStyle = style ?: return@LaunchedEffect
        (edgeStyle.getSource(EDGE_LABEL_SOURCE) as? GeoJsonSource)?.setGeoJson(
            FeatureCollection.fromFeatures(
                edgeLabels(draggingVertex, fencePolygons, surveys).map { label ->
                    Feature.fromGeometry(Point.fromLngLat(label.at.longitude, label.at.latitude)).apply { addStringProperty(LANDING_LABEL_TEXT, label.text) }
                },
            ),
        )
    }

    LaunchedEffect(style, traffic) {
        val trafficStyle = style ?: return@LaunchedEffect
        (trafficStyle.getSource(TRAFFIC_SOURCE) as? GeoJsonSource)?.setGeoJson(trafficFeatures(traffic))
    }

    val radars = if (proximityRadar) placedRadars(fleet, TrackPoint(latitude, longitude), heading) else emptyList()
    LaunchedEffect(style, radars) {
        val radarStyle = style ?: return@LaunchedEffect
        renderProximityRadars(radarStyle, radars)
    }

    val orbitPreview = mapEdits.orbit
    LaunchedEffect(style, orbit, shownGoto, orbitPreview) {
        val orbitStyle = style ?: return@LaunchedEffect
        renderOrbit(orbitStyle, orbit, gotoShown = shownGoto != null, preview = orbitPreview)
    }
    LaunchedEffect(style, collisionLegs) {
        val legStyle = style ?: return@LaunchedEffect
        renderCollisionLegs(legStyle, collisionLegs)
    }
    LaunchedEffect(style, clickMarker, orbitPreview) {
        val markerStyle = style ?: return@LaunchedEffect
        renderClickMarker(markerStyle, clickMarker.takeIf { orbitPreview == null })
    }
    LaunchedEffect(style, shownGoto, gotoEditing) {
        val gotoStyle = style ?: return@LaunchedEffect
        renderGoto(gotoStyle, shownGoto, gotoEditing)
    }
    LaunchedEffect(style, roi) {
        val roiStyle = style ?: return@LaunchedEffect
        renderRoi(roiStyle, roi)
    }

    LaunchedEffect(style, tracePoints, traceLine) {
        val traceStyle = style ?: return@LaunchedEffect
        renderTrace(traceStyle, tracePoints, traceLine)
    }

    LaunchedEffect(style, latitude, longitude, gimbals) {
        val gimbalStyle = style ?: return@LaunchedEffect
        renderGimbals(gimbalStyle, latitude, longitude, gimbals)
    }

    LaunchedEffect(style, shots) {
        val shotStyle = style ?: return@LaunchedEffect
        (shotStyle.getSource(SHOT_SOURCE) as? GeoJsonSource)?.setGeoJson(shotFeatures(shots))
    }

    fun recenterOnVehicle() {
        val tracking = follow && isPlottable(latitude, longitude) && !panning && SystemClock.elapsedRealtime() >= trackingResumesAtMs
        val shown = map
        if (tracking && shown != null) {
            val at = LatLng(latitude, longitude)
            val zoomed = shown.cameraPosition.zoom > 1.0
            val point = shown.projection.toScreenLocation(at)
            val lift = clearAreaLift(topInsetPx.toFloat(), bottomInsetPx.toFloat()).coerceIn(-mapView.height / 4f, mapView.height / 4f)
            val centred = if (zoomed && lift != 0f) shown.projection.fromScreenLocation(android.graphics.PointF(point.x, point.y - lift)) else at
            when {
                keepCentered || !zoomed -> shown.cameraPosition = CameraPosition.Builder()
                    .target(centred)
                    .zoom(shown.cameraPosition.zoom.takeIf { zoomed } ?: DEFAULT_ZOOM)
                    .build()
                outsideCentreInset(point.x, point.y, mapView.width.toFloat(), mapView.height.toFloat(), topInsetPx.toFloat(), (bottomInsetPx + cameraBottomPx).toFloat()) ->
                    shown.animateCamera(CameraUpdateFactory.newLatLng(centred), RECENTER_ANIMATION_MS)
            }
        }
    }

    LaunchedEffect(style, latitude, longitude, heading, home, linkLost, fleet, lastSeen) {
        val currentStyle = style ?: return@LaunchedEffect

        (currentStyle.getSource(VEHICLE_SOURCE) as? GeoJsonSource)
            ?.setGeoJson(
                when {
                    fleet.isEmpty() -> vehicleFeatures(latitude, longitude, heading, linkLost, lastSeen)
                    else -> fleetFeatures(fleet, lastSeen)
                },
            )

        (currentStyle.getSource(HOME_SOURCE) as? GeoJsonSource)?.setGeoJson(
            FeatureCollection.fromFeatures(
                (if (fleet.isEmpty()) listOfNotNull(home) else fleet.mapNotNull { it.home })
                    .map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) },
            ),
        )

        recenterOnVehicle()
    }

    LaunchedEffect(style, track) {
        val trailStyle = style ?: return@LaunchedEffect
        (trailStyle.getSource(TRAIL_SOURCE) as? GeoJsonSource)?.setGeoJson(
            when {
                trackDraws(track) -> FeatureCollection.fromFeatures(
                    listOf(
                        Feature.fromGeometry(
                            LineString.fromLngLats(
                                plottedTrack(track).map { Point.fromLngLat(it.longitude, it.latitude) },
                            ),
                        ),
                    ),
                )
                else -> FeatureCollection.fromFeatures(emptyList())
            },
        )
    }

    val recenterNow by rememberUpdatedState(::recenterOnVehicle)
    LaunchedEffect(trackingResumesAtMs) {
        if (trackingResumesAtMs == 0L) return@LaunchedEffect
        delay((trackingResumesAtMs - SystemClock.elapsedRealtime()).coerceAtLeast(0L))
        recenterNow()
    }

    LaunchedEffect(style, pip) {
        style?.let { applyPip(it, pip) }
    }

    LaunchedEffect(map, pip) {
        map?.uiSettings?.let { settings ->
            settings.isLogoEnabled = !pip
            settings.isAttributionEnabled = !pip
        }
    }

    LaunchedEffect(map, cameraBottomPx) {
        map?.setPadding(0, 0, 0, cameraBottomPx)
    }

    val attributionClearancePx = with(androidx.compose.ui.platform.LocalDensity.current) { ATTRIBUTION_CLEARANCE.roundToPx() }
    LaunchedEffect(map, bottomInsetPx, leftInsetPx, logoEndInsetPx) {
        val settings = map?.uiSettings ?: return@LaunchedEffect
        val bottom = bottomInsetPx + LOGO_EDGE_MARGIN_PX
        logoEndInsetPx?.let { end ->
            settings.logoGravity = android.view.Gravity.BOTTOM or android.view.Gravity.END
            settings.attributionGravity = android.view.Gravity.BOTTOM or android.view.Gravity.END
            settings.setAttributionMargins(0, 0, end, bottom)
            settings.setLogoMargins(0, 0, end + attributionClearancePx, bottom)
        } ?: run {
            settings.setLogoMargins(leftInsetPx + LOGO_EDGE_MARGIN_PX, 0, 0, bottom)
            settings.setAttributionMargins(leftInsetPx + LOGO_EDGE_MARGIN_PX, 0, 0, bottom)
        }
    }

    LaunchedEffect(centreRequest, map) {
        if (centreRequest == 0) return@LaunchedEffect
        val at = centreOn?.takeIf { isPlottable(it.latitude, it.longitude) } ?: return@LaunchedEffect
        val loaded = map ?: return@LaunchedEffect
        val target = LatLng(at.latitude, at.longitude)
        when {
            centreZoom != null -> loaded.moveCamera(CameraUpdateFactory.newLatLngZoom(target, centreZoom))
            loaded.cameraPosition.zoom > 1.0 -> loaded.animateCamera(CameraUpdateFactory.newLatLng(target))
            else -> loaded.animateCamera(CameraUpdateFactory.newLatLngZoom(target, DEFAULT_ZOOM))
        }
    }

    LaunchedEffect(map, gestures) {
        map?.uiSettings?.setAllGesturesEnabled(gestures)
    }

    LaunchedEffect(fitRequest) {
        if (fitRequest == 0) return@LaunchedEffect
        val currentMap = map ?: return@LaunchedEffect
        val bounds = planBounds(
            fitPoints(
                fitOnly ?: planPoints(missionItems, fencePolygons, fenceCircles, rallyPoints, surveys),
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
                FIT_PADDING_PIXELS + leftInsetPx,
                FIT_PADDING_PIXELS + topInsetPx,
                FIT_PADDING_PIXELS,
                FIT_PADDING_PIXELS + bottomInsetPx,
            ),
        )
    }

    LaunchedEffect(
        style, missionItems, fencePolygons, fenceCircles, rallyPoints, surveys,
        landings, firmwareFence, selectedWaypoint, linkStartToHome, operator, operatorHeading, breachReturn, otherMissions, circledShapes,
    ) {
        val currentStyle = style ?: return@LaunchedEffect
        renderSurveys(currentStyle, surveys)
        renderTransectMarks(currentStyle, surveys, selectedWaypoint, legArrows(missionItems, linkStartToHome) + loiterRotationArrows(missionItems.takeIf { editable }.orEmpty()))
        renderGimbalWedges(currentStyle, missionItems)
        renderLandings(currentStyle, landings, missionItems.takeIf { editable }.orEmpty(), selectedWaypoint.takeIf { editable })
        renderMidpoints(currentStyle, fencePolygons.map { if (fencePath(it.index) in circledShapes) it.copy(editable = null) else it }, surveys.map { if (surveyPath(it) in circledShapes) it.copy(editable = null) else it }, missionItems.takeIf { editable }.orEmpty(), selectedWaypoint)
        renderFences(currentStyle, fencePolygons, rallyPoints, circlesAsPolygons(fenceCircles), firmwareFence, breachReturn?.takeIf { isPlottable(it.latitude, it.longitude) })
        (currentStyle.getSource(GCS_SOURCE) as? GeoJsonSource)?.setGeoJson(operatorFeatures(operator, operatorHeading))
        renderVertexHandles(currentStyle, fencePolygons, surveys, fenceCircles, landings, circledShapes, loiterHandleFeatures(missionItems.takeIf { editable }.orEmpty(), selectedWaypoint))
        renderMission(currentStyle, missionItems, linkStartToHome, selectedWaypoint, otherMissions, landings)
    }

    Box(modifier) {
        AndroidView(factory = { mapView }, modifier = Modifier.matchParentSize())
        if (obstacleOverlay) {
            ObstacleMapOverlay(map, { cameraMoves }, latitude, longitude, heading, showText = !pip)
        }
    }
}

@Composable
private fun BoxScope.ObstacleMapOverlay(map: MapLibreMap?, cameraMoves: () -> Int, latitude: Double, longitude: Double, heading: Double, showText: Boolean) {
    val json by mapPath(OBSTACLE_VIEW)
    val measurer = rememberTextMeasurer()
    val overlay = remember(json) { obstacleOverlay(json) } ?: return
    ComposeCanvas(Modifier.matchParentSize()) {
        cameraMoves()
        val shown = map ?: return@ComposeCanvas
        if (!isPlottable(latitude, longitude) || heading.isNaN()) return@ComposeCanvas
        val vehicle = shown.projection.toScreenLocation(LatLng(latitude, longitude))
        val probe = TRUE_SCALE_PROBE.toPx()
        val metresInProbe = shown.projection.fromScreenLocation(android.graphics.PointF(0f, 0f))
            .distanceTo(shown.projection.fromScreenLocation(android.graphics.PointF(probe, 0f)))
        val centre = Offset(vehicle.x, vehicle.y)
        val shape = mapOverlayShape(overlay, centre, size.height, metresInProbe, probe, heading, shown.cameraPosition.bearing)
        drawMapObstacleOverlay(measurer, overlay, shape, centre, showText)
    }
}

private fun installLayers(style: Style) {
    if (style.getSource(TRAIL_SOURCE) == null) {
        style.addSource(GeoJsonSource(TRAIL_SOURCE))
        style.addLayer(
            LineLayer(TRAIL_LAYER, TRAIL_SOURCE).withProperties(
                PropertyFactory.lineColor("#FF0000"),
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

internal const val PIP_ICON_SCALE = 1f / 3f
internal const val PIP_TRAFFIC_SCALE = 1f / 2.5f

private fun applyPip(style: Style, pip: Boolean) {
    style.getLayer(TRAIL_LAYER)?.setProperties(PropertyFactory.visibility(if (pip) Property.NONE else Property.VISIBLE))
    val scale = if (pip) PIP_ICON_SCALE else 1f
    style.getLayer(VEHICLE_LAYER)?.setProperties(
        PropertyFactory.circleRadius(Expression.switchCase(Expression.get(ACTIVE_PROPERTY), Expression.literal(9f * scale), Expression.literal(6f * scale))),
        PropertyFactory.circleStrokeWidth(2f * scale),
    )
    style.getLayer(VEHICLE_HEADING_LAYER)?.setProperties(PropertyFactory.iconSize(scale))
    style.getLayer(TRAFFIC_LAYER)?.setProperties(
        PropertyFactory.iconSize(if (pip) PIP_TRAFFIC_SCALE else 1f),
        PropertyFactory.textSize(if (pip) 11f * PIP_TRAFFIC_SCALE else 11f),
    )
}

private fun installVehicleLayer(style: Style) {
    installProximityRadarLayer(style)
    installGimbalLayer(style)
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
        style.addLayer(
            SymbolLayer(VEHICLE_LABEL_LAYER, VEHICLE_SOURCE).withProperties(
                PropertyFactory.textField(Expression.get(VEHICLE_LABEL_PROPERTY)),
                PropertyFactory.textSize(11f),
                PropertyFactory.textColor("#FFFFFF"),
                PropertyFactory.textHaloColor("#000000"),
                PropertyFactory.textHaloWidth(1f),
                PropertyFactory.textOffset(arrayOf(0f, 1.6f)),
                PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ).withFilter(Expression.has(VEHICLE_LABEL_PROPERTY)),
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

private fun renderLandings(style: Style, landings: List<LandingPattern>, items: List<MissionItem>, selected: Int?) {
    (style.getSource(LANDING_AREA_SOURCE) as? GeoJsonSource)?.setGeoJson(landingAreaFeatures(landings))
    (style.getSource(LANDING_LABEL_SOURCE) as? GeoJsonSource)?.setGeoJson(landingLabelFeatures(landings, selected, items))
    (style.getSource(LANDING_PATH_SOURCE) as? GeoJsonSource)?.setGeoJson(landingPathFeatures(landings))
    (style.getSource(LANDING_LOITER_SOURCE) as? GeoJsonSource)?.setGeoJson(landingLoiterFeatures(landings, items))
}

private const val PAN_RECENTER_DELAY_MS = 10_000L
private const val RECENTER_ANIMATION_MS = 1000
private const val CENTRE_INSET_FRACTION = 0.15f

fun clearAreaLift(topInset: Float, bottomInset: Float): Float = (topInset - bottomInset) / 2f

fun outsideCentreInset(x: Float, y: Float, width: Float, height: Float, topInset: Float, bottomInset: Float): Boolean {
    val side = width * CENTRE_INSET_FRACTION
    val top = topInset + height * CENTRE_INSET_FRACTION
    val bottom = height - bottomInset - height * CENTRE_INSET_FRACTION
    return width > 0 && height > 0 && (x < side || x > width - side || y < top || y > bottom)
}
