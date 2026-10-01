package one.aircast.mapspike

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
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

private const val FLY_POLL_MS = 2000L

private data class FlownPlan(
    val items: List<MissionItem> = emptyList(),
    val linkStartToHome: Boolean = false,
    val fences: List<FencePolygon> = emptyList(),
    val circles: List<FenceCircle> = emptyList(),
    val rally: List<RallyPoint> = emptyList(),
    val surveys: List<Survey> = emptyList(),
    val operator: TrackPoint? = null,
    val shots: List<TrackPoint> = emptyList(),
    val traffic: List<TrafficMark> = emptyList(),
    val gimbals: List<GimbalAzimuth> = emptyList(),
    val roi: TrackPoint? = null,
    val goto: GotoLocation? = null,
)

@Composable
fun FlyMap(
    modifier: Modifier = Modifier,
    cameraBottomPx: Int = 0,
    onMapClick: ((Double, Double) -> Unit)? = null,
    onMissionItemClick: ((Int) -> Unit)? = null,
    onRoiClick: ((TrackPoint) -> Unit)? = null,
) {
    val context = LocalContext.current
    var style by remember(context) { mutableStateOf(planMapStyle(context)) }
    var plan by remember { mutableStateOf(FlownPlan()) }
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableDoubleStateOf(0.0) }
    val keepCentered by mapBool("view.control(settings.flyViewSettings.keepMapCenteredOnVehicle)")

    DisposableEffect(Unit) {
        onDispose { MapBridge.release() }
    }

    val lifecycleOwner = LocalLifecycleOwner.current
    LaunchedEffect(lifecycleOwner) {
        lifecycleOwner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                val next = withContext(Dispatchers.Default) {
                    val raw = PlanBridge.rawItems()
                    val fences = FenceBridge.read()
                    if (raw != null) {
                        MapBridge.markReachable()
                    }
                    planMapStyle(context).takeIf { it != style }?.let { style = it }
                    FlownPlan(
                        items = missionItems(raw),
                        linkStartToHome = linksStartToHome(raw),
                        fences = fencePolygons(fences),
                        circles = fenceCircles(fences),
                        rally = rallyPoints(fences),
                        surveys = SurveyBridge.surveysFrom(raw),
                        operator = operatorPoint(OperatorBridge.read()),
                        shots = shotPoints(VideoBridge.read()),
                        traffic = TrafficBridge.read(),
                        gimbals = GimbalBridge.read(),
                        roi = RoiBridge.read(),
                        goto = GotoBridge.read(),
                    )
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
            keepCentered = keepCentered,
            gimbals = plan.gimbals,
            proximityRadar = true,
            cameraBottomPx = cameraBottomPx,
            missionItems = plan.items,
            linkStartToHome = plan.linkStartToHome,
            fencePolygons = plan.fences,
            fenceCircles = plan.circles,
            rallyPoints = plan.rally,
            operator = plan.operator,
            surveys = plan.surveys,
            shots = plan.shots,
            traffic = plan.traffic,
            editable = false,
            onMapClick = onMapClick,
            onMissionItemClick = onMissionItemClick,
            roi = plan.roi,
            onRoiClick = onRoiClick,
            goto = plan.goto,
            onCentreChanged = { at, level ->
                centre = at
                zoom = level
            },
        )
        centre?.takeIf { zoom > 0.0 }?.let { at ->
            ScaleBarView(
                at.latitude,
                zoom,
                Modifier.align(Alignment.BottomStart).padding(start = 4.dp, bottom = 4.dp),
            )
        }
        }
    }
}
