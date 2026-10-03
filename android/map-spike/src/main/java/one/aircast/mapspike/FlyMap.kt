package one.aircast.mapspike

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

private const val FLY_POLL_MS = 2000L
private const val CAMERA_STORE = "fly-map-camera"

internal data class SavedCamera(val centre: TrackPoint, val zoom: Double)

private fun readCamera(context: android.content.Context): SavedCamera? =
    context.getSharedPreferences(CAMERA_STORE, android.content.Context.MODE_PRIVATE).let { prefs ->
        prefs.getString("camera", null)?.split(",")?.mapNotNull { it.toDoubleOrNull() }?.takeIf { it.size == 3 }
            ?.let { (latitude, longitude, zoom) -> SavedCamera(TrackPoint(latitude, longitude), zoom) }
            ?.takeIf { isPlottable(it.centre.latitude, it.centre.longitude) && it.zoom > 1.0 }
    }

private fun writeCamera(context: android.content.Context, camera: SavedCamera) {
    context.getSharedPreferences(CAMERA_STORE, android.content.Context.MODE_PRIVATE).edit()
        .putString("camera", "${camera.centre.latitude},${camera.centre.longitude},${camera.zoom}").apply()
}

internal fun centresOnOperator(alreadyCentred: Boolean, operator: TrackPoint?, vehiclePlaced: Boolean): Boolean =
    !alreadyCentred && operator != null && !vehiclePlaced

private data class FlownPlan(
    val items: List<MissionItem> = emptyList(),
    val linkStartToHome: Boolean = false,
    val fences: List<FencePolygon> = emptyList(),
    val circles: List<FenceCircle> = emptyList(),
    val firmwareFence: FirmwareFence? = null,
    val breachReturn: TrackPoint? = null,
    val rally: List<RallyPoint> = emptyList(),
    val surveys: List<Survey> = emptyList(),
    val operator: TrackPoint? = null,
    val operatorHeading: Double = Double.NaN,
    val shots: List<TrackPoint> = emptyList(),
    val traffic: List<TrafficMark> = emptyList(),
    val gimbals: List<GimbalAzimuth> = emptyList(),
    val roi: TrackPoint? = null,
    val goto: GotoLocation? = null,
    val orbit: OrbitCircle? = null,
    val current: Int? = null,
    val others: List<OtherMission> = emptyList(),
    val vehiclePlaced: Boolean = false,
)

private const val FLY_MISSION_ITEMS = "view.flyMissionItems(geometry)"

internal fun otherMissions(view: JSONObject?): List<OtherMission> {
    val others = view?.optJSONArray("others") ?: return emptyList()
    return (0 until others.length()).mapNotNull { others.optJSONObject(it) }.map { OtherMission(missionItems(it), linksStartToHome(it)) }
}

internal fun <T, K> missionArrived(before: List<T>, after: List<T>, key: (T) -> K): Boolean = after.isNotEmpty() && after.map(key) != before.map(key)

private fun shape(item: MissionItem) = listOf(item.sequence, item.latitude, item.longitude, item.command)

@Composable
fun FlyMap(
    modifier: Modifier = Modifier,
    cameraBottomPx: Int = 0,
    topInsetPx: Int = 0,
    bottomInsetPx: Int = 0,
    logoEndInsetPx: Int? = null,
    pip: Boolean = false,
    onMapClick: ((Double, Double) -> Unit)? = null,
    onMissionItemClick: ((Int) -> Unit)? = null,
    onRoiClick: ((TrackPoint) -> Unit)? = null,
    clickMarker: TrackPoint? = null,
) {
    val context = LocalContext.current
    var style by remember(context) { mutableStateOf(planMapStyle(context)) }
    var plan by remember { mutableStateOf(FlownPlan()) }
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableDoubleStateOf(0.0) }
    var fitRequest by remember { mutableIntStateOf(0) }
    val saved = remember(context) { readCamera(context) }
    var centreRequest by remember { mutableIntStateOf(if (saved != null) 1 else 0) }
    var centreOn by remember { mutableStateOf(saved?.centre) }
    var centreZoom by remember { mutableStateOf(saved?.zoom) }
    val keepCentered by mapBool("view.control(settings.flyViewSettings.keepMapCenteredOnVehicle)")

    var mainZoom by remember { mutableDoubleStateOf(saved?.zoom ?: 0.0) }
    var zoomedForPip by remember { mutableStateOf(false) }
    LaunchedEffect(pip, centre != null) {
        val at = centre ?: return@LaunchedEffect
        if (pip == zoomedForPip) return@LaunchedEffect
        zoomedForPip = pip
        pipZoom(mainZoom, pip)?.let { level ->
            centreOn = at
            centreZoom = level
            centreRequest++
        }
    }

    val lifecycleOwner = LocalLifecycleOwner.current
    LaunchedEffect(lifecycleOwner) {
        lifecycleOwner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                val next = withContext(Dispatchers.Default) {
                    val raw = runCatching { JSONObject(QGCBridge.get(FLY_MISSION_ITEMS)) }.getOrNull()
                    val fences = FenceBridge.readFlown()
                    val gcs = OperatorBridge.read()
                    if (raw != null) {
                        MapBridge.markReachable()
                    }
                    planMapStyle(context).takeIf { it != style }?.let { style = it }
                    FlownPlan(
                        items = missionItems(raw),
                        linkStartToHome = linksStartToHome(raw),
                        fences = fencePolygons(fences),
                        circles = fenceCircles(fences),
                        firmwareFence = firmwareFence(fences),
                        breachReturn = breachReturn(fences)?.point,
                        rally = rallyPoints(fences),
                        surveys = SurveyBridge.surveysFrom(raw),
                        operator = gcs.let(::operatorPoint),
                        operatorHeading = operatorHeading(gcs),
                        shots = shotPoints(VideoBridge.read()),
                        traffic = TrafficBridge.read(),
                        gimbals = GimbalBridge.read(),
                        roi = RoiBridge.read(),
                        goto = GotoBridge.read(),
                        orbit = OrbitBridge.read(),
                        current = raw?.optInt("selected", -1)?.takeIf { it > 0 },
                        others = otherMissions(raw),
                        vehiclePlaced = vehicleChoices(runCatching { JSONObject(QGCBridge.get(VEHICLES_VIEW)) }.getOrNull()).choices.any { isPlottable(it.latitude, it.longitude) },
                    )
                }
                if (missionArrived(plan.items, next.items, ::shape)) fitRequest++
                if (centresOnOperator(FlightMapPosition.operatorCentred, next.operator, next.vehiclePlaced)) {
                    FlightMapPosition.operatorCentred = true
                    centreOn = next.operator
                    centreZoom = null
                    centreRequest++
                }
                plan = next
                delay(FLY_POLL_MS)
            }
        }
    }

    Surface(modifier, color = MaterialTheme.colorScheme.surface) {
        Box(Modifier.fillMaxSize()) {
        VehicleMap(
            modifier = Modifier.fillMaxSize(),
            mapStyle = style,
            follow = true,
            keepCentered = keepCentered || pip,
            pip = pip,
            gimbals = plan.gimbals,
            proximityRadar = true,
            cameraBottomPx = cameraBottomPx,
            topInsetPx = topInsetPx,
            bottomInsetPx = bottomInsetPx,
            logoEndInsetPx = logoEndInsetPx,
            missionItems = plan.items,
            linkStartToHome = plan.linkStartToHome,
            fencePolygons = plan.fences,
            fenceCircles = plan.circles,
            firmwareFence = plan.firmwareFence,
            breachReturn = plan.breachReturn,
            rallyPoints = plan.rally,
            operator = plan.operator,
            operatorHeading = plan.operatorHeading,
            surveys = plan.surveys,
            shots = plan.shots,
            traffic = plan.traffic,
            editable = false,
            onMapClick = onMapClick,
            onMissionItemClick = onMissionItemClick,
            roi = plan.roi,
            onRoiClick = onRoiClick,
            goto = plan.goto,
            clickMarker = clickMarker,
            orbit = plan.orbit,
            selectedWaypoint = plan.current,
            otherMissions = plan.others,
            fitRequest = fitRequest,
            centreRequest = centreRequest,
            centreOn = centreOn,
            centreZoom = centreZoom,
            onCentreChanged = { at, level ->
                centre = at
                zoom = level
                FlightMapPosition.latest = at
                if (level > 1.0 && !pip) {
                    mainZoom = level
                    writeCamera(context, SavedCamera(at, level))
                }
            },
        )
        centre?.takeIf { zoom > 0.0 }?.let { at ->
            val density = androidx.compose.ui.platform.LocalDensity.current
            ScaleBarView(
                at.latitude,
                zoom,
                logoEndInsetPx?.let { end ->
                    with(density) { Modifier.align(Alignment.BottomEnd).padding(end = end.toDp(), bottom = bottomInsetPx.toDp() + SCALE_ABOVE_LOGO) }
                } ?: Modifier.align(Alignment.BottomStart).padding(start = 4.dp, bottom = 4.dp),
            )
        }
        }
    }
}

private val SCALE_ABOVE_LOGO = 40.dp

internal fun pipZoom(mainZoom: Double, pip: Boolean): Double? = when {
    mainZoom <= 0.0 -> null
    !pip -> mainZoom
    mainZoom > 3 -> mainZoom - 3
    else -> null
}
