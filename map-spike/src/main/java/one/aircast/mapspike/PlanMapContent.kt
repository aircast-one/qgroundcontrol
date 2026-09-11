package one.aircast.mapspike

import androidx.activity.compose.BackHandler
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
import androidx.compose.material3.AlertDialog
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
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInParent
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.mavlink.qgroundcontrol.QGCBridge
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.withContext

private const val PLAN_POLL_MS = 700L
private const val FAILURE_MESSAGE_MS = 2500L
private const val CONFIRM_TIMEOUT_MS = 5000L
private val CONTROLS_MAX_HEIGHT = 320.dp
private val PRIMARY_PADDING = PaddingValues(horizontal = 16.dp, vertical = 4.dp)

private fun sendPlan(
    scope: CoroutineScope,
    say: (String?) -> Unit,
    done: () -> Unit,
    pauseFirst: Boolean = false,
) {
    say("Uploading to vehicle")
    scope.launch {
        val outcome = withContext(Dispatchers.Default) {
            if (pauseFirst) {
                QGCBridge.invoke("vehicle.pauseVehicle", "[]")
            }
            uploadOutcome(PlanBridge.sendToVehicle())
        }
        say(uploadMessage(outcome))
        delay(FAILURE_MESSAGE_MS)
        done()
    }
}

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
    var fitRequest by remember { mutableIntStateOf(0) }
    var loadArmed by remember { mutableStateOf(false) }
    var clearArmed by remember { mutableStateOf(false) }
    var items by remember { mutableStateOf<List<MissionItem>>(emptyList()) }
    var itemCount by remember { mutableIntStateOf(0) }
    var shape by remember { mutableStateOf<List<String>>(emptyList()) }
    var linkStartToHome by remember { mutableStateOf(false) }
    var fences by remember { mutableStateOf<List<FencePolygon>>(emptyList()) }
    var rally by remember { mutableStateOf<List<RallyPoint>>(emptyList()) }
    var circles by remember { mutableStateOf<List<FenceCircle>>(emptyList()) }
    var surveyList by remember { mutableStateOf<List<Survey>>(emptyList()) }
    var selected by remember { mutableStateOf<MapHit?>(null) }

    BackHandler(enabled = selected != null) { selected = null }
    var busy by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableDoubleStateOf(0.0) }
    var controlsHeightPx by remember { mutableIntStateOf(0) }
    var topOverlayPx by remember { mutableIntStateOf(0) }

    fun say(message: String) {
        busy = message
        scope.launch {
            delay(FAILURE_MESSAGE_MS)
            busy = null
        }
    }

    fun addMissionItem(kindId: String, label: String, at: TrackPoint?, index: Int = AT_END) {
        scope.launch {
            if (at == null) {
                busy = "Move the map to where this should go"
                delay(FAILURE_MESSAGE_MS)
                busy = null
                return@launch
            }
            busy = label
            val outcome = withContext(Dispatchers.Default) {
                insertMissionItem(kindId, at.latitude, at.longitude, index)
            }
            if (outcome.ok) {
                busy = null
            } else {
                busy = outcome.reason
                delay(FAILURE_MESSAGE_MS)
                busy = null
            }
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
    val planHasItems by mapBool("plan.containsItems")
    val planOffline by mapBool("plan.offline")
    val planSyncing by mapBool("plan.syncInProgress")
    var uploadAsk by remember { mutableStateOf<UploadGate?>(null) }
    val missionSummaryView by mapPath("view.missionSummary")
    val terrainView by mapPath(TERRAIN_VIEW)
    val profile = remember(terrainView) { terrainProfile(terrainView) }
    val mode by mapString("vehicle.flightMode")
    val vehicleId by mapInt("vehicle.id")
    val vehicleCount by mapCount("vehicles.vehicles")

    fun placeAt(): TrackPoint? = when {
        isPlottable(latitude, longitude) -> TrackPoint(latitude, longitude)
        else -> centre?.takeIf { isPlottable(it.latitude, it.longitude) }
    }

    var fittedToPlan by remember { mutableStateOf(false) }
    val planIsDrawn = items.isNotEmpty() || surveyList.isNotEmpty() ||
        fences.isNotEmpty() || circles.isNotEmpty() || rally.isNotEmpty()

    LaunchedEffect(planIsDrawn, isPlottable(latitude, longitude)) {
        if (isPlottable(latitude, longitude)) {
            fittedToPlan = true
        } else if (planIsDrawn && !fittedToPlan) {
            fittedToPlan = true
            fitRequest += 1
        }
    }

    suspend fun refresh() {
        withContext(Dispatchers.Default) {
            val plan = PlanBridge.rawItems()
            if (plan != null) {
                MapBridge.markReachable()
            }
            val nextItems = missionItems(plan)
            val nextItemCount = planItemCount(plan)
            val nextShape = planShape(plan)
            val nextLink = linksStartToHome(plan)
            val fenceView = FenceBridge.read()
            val nextFences = fencePolygons(fenceView)
            val nextRally = rallyPoints(fenceView)
            val nextCircles = fenceCircles(fenceView)
            val nextSurveys = SurveyBridge.surveysFrom(plan)
            withContext(Dispatchers.Main) {
                items = nextItems
                itemCount = nextItemCount
                shape = nextShape
                linkStartToHome = nextLink
                fences = nextFences
                rally = nextRally
                if (!selectionSurvives(selected, nextItems, nextFences, nextCircles, nextRally, nextSurveys)) {
                    selected = null
                }
                circles = nextCircles
                surveyList = nextSurveys
            }
        }
    }

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
            linkStartToHome = linkStartToHome,
            fencePolygons = fences,
            fenceCircles = circles,
            rallyPoints = rally,
            surveys = surveyList,
            editable = true,
            onAdd = { lat, lon ->
                addMissionItem("waypoint", "Adding a waypoint", TrackPoint(lat, lon))
            },
            onMove = { hit, lat, lon ->
                onBridge {
                    when (hit) {
                        is MapHit.Waypoint -> PlanBridge.moveItem(hit.index, lat, lon)
                        is MapHit.FenceVertex -> FenceBridge.adjustVertex(hit.polygon, hit.vertex, lat, lon)
                        is MapHit.SurveyVertex -> surveyList.firstOrNull { it.index == hit.item }
                            ?.let { SurveyBridge.adjustAreaVertex(it, hit.vertex, lat, lon) } == true
                        is MapHit.Rally -> FenceBridge.moveRallyPoint(hit.index, lat, lon)
                        is MapHit.CircleCentre -> FenceBridge.moveCircle(hit.index, lat, lon)
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
            topInsetPx = topOverlayPx,
            fitRequest = fitRequest,
            onFitFailed = { onBridge("Fitting the plan") { false } },
        )

        FilterChip(
            selected = follow,
            onClick = { follow = !follow },
            label = { Text("Follow", style = MaterialTheme.typography.labelSmall) },
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(8.dp)
                .onGloballyPositioned { topOverlayPx = it.size.height + it.positionInParent().y.toInt() },
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
                        "Waiting for QGroundControl"
                    } else {
                        planSummary(
                            itemCount, shape, items, fences, circles, rally, surveyList,
                            missionSummaryText(missionSummaryView), selected,
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
            Column(
                Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    .heightIn(max = CONTROLS_MAX_HEIGHT)
                    .verticalScroll(rememberScrollState()),
            ) {
                TerrainProfileView(profile)

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
                            loadStep(planDirty, planHasItems, loadArmed) == LoadStep.Confirm -> loadArmed = true
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

                    Button(onClick = {
                        val refusal = syncRefusal(
                            vehicleSyncState(planOffline, planSyncing), "upload to",
                        )
                        if (refusal != null) {
                            say(refusal)
                        } else {
                            scope.launch {
                                val gate = withContext(Dispatchers.Default) { freshUploadGate() }
                                when (val step = uploadStep(gate)) {
                                    is UploadStep.Refuse -> say(step.reason)
                                    is UploadStep.Confirm -> uploadAsk = step.gate
                                    UploadStep.Send -> sendPlan(scope, say = { busy = it }, done = { busy = null })
                                }
                            }
                        }
                    }, contentPadding = PRIMARY_PADDING) { Text("Upload") }

                    uploadAsk?.let { gate ->
                        AlertDialog(
                            onDismissRequest = { uploadAsk = null },
                            title = { Text(gate.heading.ifBlank { "Upload this plan?" }) },
                            text = { Text(gate.refusal) },
                            confirmButton = {
                                TextButton(onClick = {
                                    val pauses = gate.pausesFirst
                                    uploadAsk = null
                                    sendPlan(
                                        scope,
                                        say = { busy = it },
                                        done = { busy = null },
                                        pauseFirst = pauses,
                                    )
                                }) { Text(gate.proceedTitle.ifBlank { "Upload" }) }
                            },
                            dismissButton = {
                                TextButton(onClick = { uploadAsk = null }) { Text("Cancel") }
                            },
                        )
                    }

                    if (vehicleCount > 1) {
                        var vehicles by remember(vehicleCount, vehicleId) {
                            mutableStateOf<List<VehicleEntry>>(emptyList())
                        }
                        LaunchedEffect(vehicleCount, vehicleId) {
                            vehicles = withContext(Dispatchers.Default) {
                                VehicleBridge.entries()
                            }
                        }
                        vehicles.filterNot { it.active }.forEach { entry ->
                            TextButton(onClick = {
                                scope.launch {
                                    val switched = withContext(Dispatchers.Default) {
                                        VehicleBridge.askFor(entry.id)
                                    }
                                    busy = if (switched) {
                                        "Asked for vehicle ${entry.id}"
                                    } else {
                                        VehicleBridge.lastRefusal ?: "Could not switch vehicle"
                                    }
                                    delay(FAILURE_MESSAGE_MS)
                                    busy = null
                                }
                            }) { Text("Vehicle ${entry.id}") }
                        }
                    }

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
                        addMissionItem("survey", "Adding survey", at)
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
                    TextButton(
                        onClick = {
                            val at = placeAt()
                            addMissionItem(
                                "takeoff",
                                "Adding a takeoff",
                                at,
                                if (takeoffMissing(items)) BEFORE_THE_REST else AT_END,
                            )
                        },
                    ) { Text("Takeoff") }
                    TextButton(
                        onClick = {
                            val at = placeAt()
                            addMissionItem("land", "Adding a landing", at)
                        },
                    ) { Text("Land") }
                    GroupBreak()

                    TextButton(onClick = {
                        follow = false
                        fitRequest += 1
                    }) { Text("Fit") }

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
                        survey?.takeIf { it.kind == KIND_SURVEY }?.let {
                            TextButton(onClick = {
                                onBridge("Rotating grid") {
                                    SurveyBridge.rotateGrid(it.index)
                                }
                            }) { Text("Rotate") }
                        }

                        waypoint?.let { item ->
                            if (!item.altitude.isNaN()) {
                                var typed by remember(item.index, item.altitude) {
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

                            if (item.index > HOME_ITEM) {
                                TextButton(onClick = {
                                    scope.launch {
                                        busy = "Removing #${item.index}"
                                        val outcome = withContext(Dispatchers.Default) {
                                            removeMissionItem(item.index)
                                        }
                                        selected = null
                                        if (outcome.ok) {
                                            busy = null
                                        } else {
                                            busy = outcome.reason
                                            delay(FAILURE_MESSAGE_MS)
                                            busy = null
                                        }
                                    }
                                }) { Text("Delete #${item.index}") }
                            }
                        }

                        fenceHit?.let { hit ->
                            if (cornerRemovable(fences.firstOrNull { it.index == hit.polygon })) {
                                TextButton(onClick = {
                                    onBridge("Removing corner") {
                                        FenceBridge.removeVertex(hit.polygon, hit.vertex)
                                    }
                                    selected = null
                                }) { Text("Remove corner") }
                            }
                            TextButton(onClick = {
                                onBridge { FenceBridge.deletePolygon(hit.polygon) }
                                selected = null
                            }) { Text("Delete fence") }
                        }

                        surveyHit?.let { hit ->
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
