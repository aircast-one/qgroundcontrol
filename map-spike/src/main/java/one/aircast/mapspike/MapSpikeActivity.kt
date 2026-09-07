package one.aircast.mapspike

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val PLAN_POLL_MS = 700L
private const val FAILURE_MESSAGE_MS = 2500L

class MapSpikeActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                PlanMapScreen(Modifier.fillMaxSize())
            }
        }
    }
}

@Composable
internal fun MapSpikeScreen(mapStyle: String) {
    var follow by remember { mutableStateOf(true) }
    var fitRequest by remember { mutableStateOf(0) }
    var items by remember { mutableStateOf<List<MissionItem>>(emptyList()) }
    var fences by remember { mutableStateOf<List<FencePolygon>>(emptyList()) }
    var rally by remember { mutableStateOf<List<RallyPoint>>(emptyList()) }
    var circles by remember { mutableStateOf<List<FenceCircle>>(emptyList()) }
    var surveyList by remember { mutableStateOf<List<Survey>>(emptyList()) }
    var profile by remember { mutableStateOf(TerrainProfile(emptyList())) }
    var selected by remember { mutableStateOf<MapHit?>(null) }
    var busy by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableStateOf(0.0) }
    var controlsHeightPx by remember { mutableStateOf(0) }

    // A bridge call that fails returns false rather than throwing, so without
    // this a refused operation looks exactly like one that worked.
    fun onBridge(label: String? = null, work: () -> Boolean) {
        busy = label
        scope.launch {
            val ok = withContext(Dispatchers.Default) { work() }
            if (ok) {
                busy = null
            } else {
                busy = "${label ?: "That"} did not work"
                delay(FAILURE_MESSAGE_MS)
                busy = null
            }
        }
    }

    val latitude by mapDouble("vehicle.latitude")
    val longitude by mapDouble("vehicle.longitude")
    val missionDistance by mapDouble("plan.missionController.missionTotalDistance")
    val missionTime by mapDouble("plan.missionController.missionTime")
    val mode by mapString("vehicle.flightMode")
    val vehicleId by mapInt("vehicle.id")
    val vehicleCount by mapCount("vehicles.vehicles")

    // Placing something needs a position. The vehicle's is the useful one, but
    // the map centre lets the spike be driven with no vehicle connected.
    fun placeAt(): TrackPoint? = when {
        isPlottable(latitude, longitude) -> TrackPoint(latitude, longitude)
        else -> centre?.takeIf { isPlottable(it.latitude, it.longitude) }
    }

    suspend fun refresh() {
        withContext(Dispatchers.Default) {
            val plan = PlanBridge.rawItems()
            val nextItems = missionItems(plan)
            val nextFences = FenceBridge.polygons()
            val nextRally = FenceBridge.rally()
            val nextCircles = FenceBridge.circles()
            val nextSurveys = SurveyBridge.surveysFrom(plan)
            val nextProfile = terrainProfile(plan)
            withContext(Dispatchers.Main) {
                items = nextItems
                fences = nextFences
                rally = nextRally
                circles = nextCircles
                surveyList = nextSurveys
                profile = nextProfile
            }
        }
    }

    // Every poll is a blocking trip into the Qt thread. Hosted as a tab the
    // screen stays composed while the app is in the background, so an ungated
    // loop would go on paying that for a map nobody is looking at.
    val lifecycleOwner = LocalLifecycleOwner.current
    LaunchedEffect(lifecycleOwner) {
        lifecycleOwner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                refresh()
                delay(PLAN_POLL_MS)
            }
        }
    }

    Box(Modifier.fillMaxSize()) {
        VehicleMap(
            modifier = Modifier.fillMaxSize(),
            mapStyle = mapStyle,
            follow = follow,
            missionItems = items,
            fencePolygons = fences,
            fenceCircles = circles,
            rallyPoints = rally,
            surveys = surveyList,
            editable = true,
            onAdd = { lat, lon -> onBridge { PlanBridge.appendWaypoint(lat, lon) } },
            onMove = { hit, lat, lon ->
                onBridge {
                    when (hit) {
                        is MapHit.Waypoint -> PlanBridge.moveItem(hit.index, lat, lon)
                        is MapHit.FenceVertex -> FenceBridge.adjustVertex(hit.polygon, hit.vertex, lat, lon)
                        is MapHit.SurveyVertex -> SurveyBridge.adjustAreaVertex(hit.item, hit.vertex, lat, lon)
                        is MapHit.Rally -> FenceBridge.moveRallyPoint(hit.index, lat, lon)
                        is MapHit.CircleCentre -> FenceBridge.moveCircle(hit.index, lat, lon)
                        // Tapping the fill selects a circle; its centre handle moves
                        // it. Nothing to do here is not a failure.
                        is MapHit.Circle -> true
                    }
                }
            },
            onWaypointSelected = { selected = it },
            selectedWaypoint = (selected as? MapHit.Waypoint)?.index,
            onCentreChanged = { at, level ->
                centre = at
                zoom = level
            },
            bottomInsetPx = controlsHeightPx,
            fitRequest = fitRequest,
            onFitFailed = { onBridge("Fitting the plan") { false } },
        )

        Column(
            Modifier.align(Alignment.TopStart).padding(8.dp),
        ) {
            Surface(
                color = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
                shape = MaterialTheme.shapes.small,
            ) {
                val ready by MapBridge.bridgeReady.collectAsState()
                Text(
                    busy ?: if (!ready) {
                        "Bridge not running (start the main app first)"
                    } else {
                        planSummary(
                            items, fences, circles, rally, surveyList,
                            missionDistance, missionTime, selected,
                        )
                    },
                    Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                    style = MaterialTheme.typography.bodySmall,
                )
            }

            centre?.let { at ->
                ScaleBarView(at.latitude, zoom, Modifier.padding(start = 4.dp, top = 8.dp))
            }
        }

        Surface(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                .onGloballyPositioned { controlsHeightPx = it.size.height },
            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
        ) {
            Column(Modifier.padding(horizontal = 8.dp, vertical = 4.dp)) {
                TerrainProfileView(profile)

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Switch(checked = follow, onCheckedChange = { follow = it })
                    Text(
                        "Follow",
                        Modifier.padding(start = 8.dp),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }

                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    TextButton(onClick = {
                        onBridge("Loading from vehicle") { PlanBridge.loadFromVehicle() }
                    }) { Text("Load") }

                    TextButton(onClick = {
                        onBridge("Sending to vehicle") { PlanBridge.sendToVehicle() }
                    }) { Text("Send") }

                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding fence") {
                            at != null && FenceBridge.addInclusionPolygon(
                                TrackPoint(at.latitude + 0.002, at.longitude - 0.002),
                                TrackPoint(at.latitude - 0.002, at.longitude + 0.002),
                            )
                        }
                    }) { Text("Fence") }

                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding survey") {
                            at != null && SurveyBridge.insertSurvey(at.latitude, at.longitude)
                        }
                    }) { Text("Survey") }

                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding circle") {
                            at != null && FenceBridge.addInclusionCircle(
                                TrackPoint(at.latitude + 0.002, at.longitude - 0.002),
                                TrackPoint(at.latitude - 0.002, at.longitude + 0.002),
                            )
                        }
                    }) { Text("Circle") }

                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding rally") {
                            at != null && FenceBridge.addRallyPoint(at.latitude, at.longitude)
                        }
                    }) { Text("Rally") }
                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding a takeoff") {
                            at != null && PlanBridge.appendTakeoff(at.latitude, at.longitude)
                        }
                    }) { Text("Takeoff") }
                    TextButton(onClick = {
                        val at = placeAt()
                        onBridge("Adding a landing") {
                            at != null && PlanBridge.appendLanding(at.latitude, at.longitude)
                        }
                    }) { Text("Land") }
                    TextButton(onClick = {
                        follow = false
                        fitRequest += 1
                    }) { Text("Fit") }

                }

                // Actions for what is selected live on their own line. They used
                // to sit at the end of the row above, where they scrolled out of
                // sight and read as missing.
                val survey = surveyList.firstOrNull()
                val waypoint = (selected as? MapHit.Waypoint)
                    ?.let { hit -> items.firstOrNull { it.index == hit.index } }
                val fenceHit = selected as? MapHit.FenceVertex
                val surveyHit = selected as? MapHit.SurveyVertex
                val rallyHit = selected as? MapHit.Rally
                val circleIndex = (selected as? MapHit.Circle)?.index
                    ?: (selected as? MapHit.CircleCentre)?.index
                val circle = circleIndex?.let { index -> circles.firstOrNull { it.index == index } }

                if (survey != null || waypoint != null || fenceHit != null ||
                    rallyHit != null || circle != null
                ) {
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        survey?.let {
                            TextButton(onClick = {
                                val next = (if (it.gridAngle.isNaN()) 0.0 else it.gridAngle) + 30.0
                                onBridge("Rotating grid") {
                                    SurveyBridge.setGridAngle(it.index, next % 360.0)
                                }
                            }) { Text("Rotate") }
                        }

                        waypoint?.let { item ->
                            if (!item.altitude.isNaN()) {
                                TextButton(onClick = {
                                    onBridge { PlanBridge.setAltitude(item.index, item.altitude + 10.0) }
                                }) { Text("Alt +10") }

                                TextButton(
                                    enabled = item.altitude >= 10.0,
                                    onClick = {
                                        onBridge { PlanBridge.setAltitude(item.index, item.altitude - 10.0) }
                                    },
                                ) { Text("Alt -10") }
                            }

                            TextButton(onClick = {
                                onBridge { PlanBridge.removeItem(item.index) }
                                selected = null
                            }) { Text("Delete #${item.index}") }
                        }

                        fenceHit?.let { hit ->
                            TextButton(onClick = {
                                onBridge { FenceBridge.deletePolygon(hit.polygon) }
                                selected = null
                            }) { Text("Delete fence") }
                        }

                        surveyHit?.let { hit ->
                            TextButton(onClick = {
                                onBridge { PlanBridge.removeItem(hit.item) }
                                selected = null
                            }) { Text("Delete survey") }
                        }

                        circle?.let { it ->
                            TextButton(onClick = {
                                onBridge { FenceBridge.setCircleRadius(it.index, it.radius * 1.5) }
                            }) { Text("Bigger") }

                            TextButton(
                                enabled = it.radius > 20.0,
                                onClick = {
                                    onBridge { FenceBridge.setCircleRadius(it.index, it.radius / 1.5) }
                                },
                            ) { Text("Smaller") }

                            TextButton(onClick = {
                                onBridge { FenceBridge.deleteCircle(it.index) }
                                selected = null
                            }) { Text("Delete circle") }
                        }

                        rallyHit?.let { hit ->
                            TextButton(onClick = {
                                onBridge { FenceBridge.removeRallyPoint(hit.index) }
                                selected = null
                            }) { Text("Delete rally") }
                        }

                    }
                }
            }
        }
    }
}
