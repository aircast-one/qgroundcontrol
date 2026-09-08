package one.aircast.mapspike

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.VerticalDivider
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val PLAN_POLL_MS = 700L
private const val FAILURE_MESSAGE_MS = 2500L
private const val CONFIRM_TIMEOUT_MS = 5000L
private val CONTROLS_MAX_HEIGHT = 320.dp
private val PRIMARY_PADDING = PaddingValues(horizontal = 16.dp, vertical = 4.dp)

// The row mixed vehicle sync, item creation and view control with nothing to
// say where one kind ended and the next began.
@Composable
private fun GroupBreak() {
    VerticalDivider(
        Modifier.height(24.dp).padding(horizontal = 6.dp),
        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f),
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun MapSpikeScreen(
    mapStyle: String,
    onClear: (() -> Unit)? = null,
    onCentre: ((Double, Double) -> Unit)? = null,
) {
    var follow by remember { mutableStateOf(true) }
    var fitRequest by remember { mutableStateOf(0) }
    var loadArmed by remember { mutableStateOf(false) }
    var clearArmed by remember { mutableStateOf(false) }
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
    fun say(message: String) {
        busy = message
        scope.launch {
            delay(FAILURE_MESSAGE_MS)
            busy = null
        }
    }

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
    val planDirty by mapBool("plan.dirty")
    val planOffline by mapBool("plan.offline")
    val planSyncing by mapBool("plan.syncInProgress")
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
            if (plan != null) {
                MapBridge.markReachable()
            }
            val nextItems = missionItems(plan)
            val nextFences = FenceBridge.polygons()
            val nextRally = FenceBridge.rally()
            val nextCircles = FenceBridge.circles()
            val nextSurveys = SurveyBridge.surveysFrom(plan)
            val nextProfile = terrainProfile(plan, SegmentBridge::forItem)
            withContext(Dispatchers.Main) {
                items = nextItems
                fences = nextFences
                rally = nextRally
                if (!selectionSurvives(selected, nextItems, nextFences, nextCircles, nextRally, nextSurveys)) {
                    selected = null
                }
                circles = nextCircles
                surveyList = nextSurveys
                profile = nextProfile
            }
        }
    }

    // An armed Load that stays armed is a trap: the next stray tap discards the
    // plan. It goes back to asking on its own.
    LaunchedEffect(loadArmed) {
        if (loadArmed) {
            delay(CONFIRM_TIMEOUT_MS)
            loadArmed = false
        }
    }

    LaunchedEffect(clearArmed) {
        if (clearArmed) {
            delay(CONFIRM_TIMEOUT_MS)
            clearArmed = false
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
                onCentre?.invoke(at.latitude, at.longitude)
            },
            bottomInsetPx = controlsHeightPx,
            fitRequest = fitRequest,
            onFitFailed = { onBridge("Fitting the plan") { false } },
        )

        // On the map rather than in the panel. It steers the map and nothing
        // else, and a whole row of the panel is a row the map does not get.
        FilterChip(
            selected = follow,
            onClick = { follow = !follow },
            label = { Text("Follow", style = MaterialTheme.typography.labelSmall) },
            modifier = Modifier.align(Alignment.TopEnd).padding(8.dp),
            colors = FilterChipDefaults.filterChipColors(
                containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
                selectedContainerColor =
                    MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.92f),
            ),
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
                        // What is known is that no read has come back. Why is a
                        // guess, and in the app's Plan tab the old guess — start
                        // the main app — is advice to do the thing already done.
                        "Waiting for QGroundControl"
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
            // The panel grows with what is selected, and it had grown past the
            // screen: the survey altitude field and Delete survey were rendering
            // below the fold with nothing to say so. Capping it and letting it
            // scroll means adding a control can never again make an existing one
            // unreachable.
            Column(
                Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    .heightIn(max = CONTROLS_MAX_HEIGHT)
                    .verticalScroll(rememberScrollState()),
            ) {
                TerrainProfileView(profile)

                // Wrapping beats scrolling here: a scrolled row cut a button off
                // mid-word at the right edge, which reads as a rendering fault
                // rather than an invitation to scroll.
                FlowRow(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    TextButton(onClick = {
                        val refusal = syncRefusal(
                            vehicleSyncState(planOffline, planSyncing), "download from",
                        )
                        when {
                            refusal != null -> say(refusal)
                            loadStep(planDirty, loadArmed) == LoadStep.Confirm -> loadArmed = true
                            else -> {
                                loadArmed = false
                                busy = "Downloading from vehicle"
                                scope.launch {
                                    val outcome = withContext(Dispatchers.Default) {
                                        uploadOutcome(PlanBridge.loadFromVehicle())
                                    }
                                    busy = downloadMessage(outcome)
                                    delay(FAILURE_MESSAGE_MS)
                                    busy = null
                                }
                            }
                        }
                    }) { Text(if (loadArmed) "Discard & download" else "Download") }

                    // Upload is the action that finishes the job — the mission
                    // reaching the aircraft — and it read as one of twelve
                    // identical choices, indistinguishable from Fit, which only
                    // moves the camera. One filled button says which one matters.
                    Button(onClick = {
                        val refusal = syncRefusal(
                            vehicleSyncState(planOffline, planSyncing), "upload to",
                        )
                        if (refusal != null) say(refusal) else {
                            busy = "Uploading to vehicle"
                            scope.launch {
                                val outcome = withContext(Dispatchers.Default) {
                                    uploadOutcome(PlanBridge.sendToVehicle())
                                }
                                busy = uploadMessage(outcome)
                                delay(FAILURE_MESSAGE_MS)
                                busy = null
                            }
                        }
                    }, contentPadding = PRIMARY_PADDING) { Text("Upload") }

                    GroupBreak()

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
                    GroupBreak()

                    TextButton(onClick = {
                        follow = false
                        fitRequest += 1
                    }) { Text("Fit") }

                    // Reads only. Making a vehicle active means writing a
                    // Vehicle* to vehicles.activeVehicle, and the bridge resolves
                    // that path to the vehicle itself rather than to the writable
                    // property, so the write cannot be expressed yet.
                    if (vehicleCount > 1) {
                        var vehicles by remember(vehicleCount, vehicleId) {
                            mutableStateOf<List<VehicleEntry>>(emptyList())
                        }
                        LaunchedEffect(vehicleCount, vehicleId) {
                            vehicles = withContext(Dispatchers.Default) {
                                VehicleBridge.entries(vehicleId)
                            }
                        }
                        vehicleSummary(vehicles)?.let {
                            Text(it, style = MaterialTheme.typography.labelSmall)
                        }
                    }

                    // Only offered when the host supplies one. Clearing the plan
                    // from in here would empty it behind a shell that still holds
                    // the opened document, leaving the next Save to write a blank
                    // plan over the user's file and report success.
                    onClear?.let { clear ->
                        TextButton(onClick = {
                            if (!clearArmed) {
                                clearArmed = true
                            } else {
                                clearArmed = false
                                selected = null
                                clear()
                            }
                        }) { Text(if (clearArmed) "Clear everything" else "Clear") }
                    }

                }

                // Actions for what is selected live on their own line. They used
                // to sit at the end of the row above, where they scrolled out of
                // sight and read as missing.
                val survey = selectedSurvey(selected, surveyList)
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
                                // Stepping by ten is fine for a nudge and hopeless for
                                // reaching a particular height, which is the usual reason
                                // to touch an altitude at all.
                                var typed by remember(item.index) {
                                    mutableStateOf(altitudeFieldText(item.altitude))
                                }
                                OutlinedTextField(
                                    value = typed,
                                    onValueChange = { typed = it },
                                    label = { Text("Alt m") },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(
                                        keyboardType = KeyboardType.Number,
                                        imeAction = ImeAction.Done,
                                    ),
                                    keyboardActions = KeyboardActions(
                                        onDone = {
                                            val metres = parsedAltitude(typed)
                                            if (metres == null) {
                                                say("Not an altitude")
                                            } else {
                                                onBridge("Setting altitude") {
                                                    PlanBridge.setAltitude(item.index, metres)
                                                }
                                            }
                                        },
                                    ),
                                    modifier = Modifier.width(120.dp),
                                    textStyle = MaterialTheme.typography.bodySmall,
                                )

                                TextButton(onClick = {
                                    onBridge { PlanBridge.setAltitude(item.index, item.altitude + 10.0) }
                                }) { Text("+10") }

                                TextButton(
                                    enabled = item.altitude >= 10.0,
                                    onClick = {
                                        onBridge { PlanBridge.setAltitude(item.index, item.altitude - 10.0) }
                                    },
                                ) { Text("-10") }
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
                            // Read once when the survey is picked. Polling it would
                            // add a call per survey per poll for a value that only
                            // changes when someone changes it.
                            var surveyAlt by remember(hit.item) { mutableStateOf("") }
                            LaunchedEffect(hit.item) {
                                val metres = withContext(Dispatchers.Default) {
                                    SurveyBridge.altitude(hit.item)
                                }
                                surveyAlt = altitudeFieldText(metres)
                            }
                            OutlinedTextField(
                                value = surveyAlt,
                                onValueChange = { surveyAlt = it },
                                label = { Text("Survey alt m") },
                                singleLine = true,
                                keyboardOptions = KeyboardOptions(
                                    keyboardType = KeyboardType.Number,
                                    imeAction = ImeAction.Done,
                                ),
                                keyboardActions = KeyboardActions(
                                    onDone = {
                                        val metres = parsedAltitude(surveyAlt)
                                        if (metres == null) {
                                            say("Not an altitude")
                                        } else {
                                            onBridge("Setting survey altitude") {
                                                SurveyBridge.setAltitude(hit.item, metres)
                                            }
                                        }
                                    },
                                ),
                                modifier = Modifier.width(150.dp),
                                textStyle = MaterialTheme.typography.bodySmall,
                            )
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
