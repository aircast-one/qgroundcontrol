package one.aircast.mapspike

import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.foundation.clickable
import androidx.compose.material3.ButtonDefaults
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.ui.semantics.semantics

import androidx.compose.ui.semantics.contentDescription

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.ui.platform.LocalContext
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import org.json.JSONObject
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.TextButton
import androidx.compose.runtime.produceState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInParent
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
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
private val SIDE_PANEL_WIDTH = 380.dp
private const val SIDE_PANEL_MIN_WIDTH_DP = 840
internal const val SUMMARY_MAX_FRACTION = 0.74f

private val PRIMARY_PADDING = PaddingValues(horizontal = 16.dp, vertical = 4.dp)

@Composable
private fun PlanUploadButton(
    emphasised: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
    contentPadding: PaddingValues,
    content: @Composable RowScope.() -> Unit,
) = when (emphasised) {
    true -> Button(onClick = onClick, enabled = enabled, contentPadding = contentPadding, content = content)
    false -> FilledTonalButton(onClick = onClick, enabled = enabled, contentPadding = contentPadding, content = content)
}

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
private fun FenceHeading(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.fillMaxWidth().padding(start = 4.dp, end = 4.dp, top = 8.dp, bottom = 4.dp),
    )
}

@Composable
private fun PaletteNote(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp),
    )
}

private const val FALLBACK_FENCE_DEGREES = 0.002

fun fenceWindow(visible: List<TrackPoint>, at: TrackPoint): Pair<TrackPoint, TrackPoint> =
    visible.takeIf { it.size == 4 }?.let { it[0] to it[2] }
        ?: (TrackPoint(at.latitude + FALLBACK_FENCE_DEGREES, at.longitude - FALLBACK_FENCE_DEGREES) to TrackPoint(at.latitude - FALLBACK_FENCE_DEGREES, at.longitude + FALLBACK_FENCE_DEGREES))

class PlanUpload(val enabled: Boolean, val emphasised: Boolean, val label: String, val shown: Boolean, val onClick: () -> Unit) {
    val done: Boolean get() = label == UPLOADED
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
internal fun MapSpikeScreen(
    mapStyle: String,
    onClear: (() -> Unit)? = null,
    onCentre: ((Double, Double) -> Unit)? = null,
    itemEditor: (@Composable (Int, TrackPoint?, () -> Unit) -> Unit)? = null,
    header: (@Composable (PlanUpload) -> Unit)? = null,
    fitKey: Int = 0,
    overlay: (@Composable BoxScope.() -> Unit)? = null,
    summaryHidden: Boolean = false,
) {
    var follow by remember { mutableStateOf(false) }
    var shownStyle by remember(mapStyle) { mutableStateOf(mapStyle) }
    var editingItem by remember { mutableStateOf<MissionItem?>(null) }
    var fitRequest by remember { mutableIntStateOf(0) }
    var edits by remember { mutableIntStateOf(0) }
    var fitOnly by remember { mutableStateOf<List<TrackPoint>?>(null) }
    var positioning by remember { mutableStateOf<Pair<MapHit, TrackPoint>?>(null) }
    var loadArmed by remember { mutableStateOf(false) }
    var clearArmed by remember { mutableStateOf(false) }
    var items by remember { mutableStateOf<List<MissionItem>>(emptyList()) }
    var allItems by remember { mutableStateOf<List<MissionItem>>(emptyList()) }
    var itemCount by remember { mutableIntStateOf(0) }
    var shape by remember { mutableStateOf<List<String>>(emptyList()) }
    var linkStartToHome by remember { mutableStateOf(false) }
    var fences by remember { mutableStateOf<List<FencePolygon>>(emptyList()) }
    var chosenCircles by remember { mutableStateOf(setOf<String>()) }
    var radiusFor by remember { mutableStateOf<ShapeTarget?>(null) }
    var rally by remember { mutableStateOf<List<RallyPoint>>(emptyList()) }
    var operator by remember { mutableStateOf<TrackPoint?>(null) }
    var circles by remember { mutableStateOf<List<FenceCircle>>(emptyList()) }
    var firmware by remember { mutableStateOf<FirmwareFence?>(null) }
    var breach by remember { mutableStateOf<BreachReturn?>(null) }
    var editingBreach by remember { mutableStateOf(false) }
    var surveyList by remember { mutableStateOf<List<Survey>>(emptyList()) }
    val circled = liveCircles(chosenCircles, fences, surveyList)
    var landingList by remember { mutableStateOf<List<LandingPattern>>(emptyList()) }
    var surveyStatsMap by remember { mutableStateOf<Map<Int, SurveyStats>>(emptyMap()) }
    var selected by remember { mutableStateOf<MapHit?>(null) }
    var layer by remember { mutableStateOf(PlanLayer.Mission) }
    LaunchedEffect(selected) { layerOf(selected)?.let { layer = it } }

    BackHandler(enabled = selected != null) { selected = null }
    var busy by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableDoubleStateOf(0.0) }
    var controlsHeightPx by remember { mutableIntStateOf(0) }
    val sidePanel = LocalConfiguration.current.screenWidthDp >= SIDE_PANEL_MIN_WIDTH_DP
    val sidePanelPx = with(LocalDensity.current) { SIDE_PANEL_WIDTH.roundToPx() }.takeIf { sidePanel } ?: 0
    var topOverlayPx by remember { mutableIntStateOf(0) }

    val kindsView by mapPath("view.missionKinds")
    val insertable by remember { derivedStateOf { missionKinds(kindsView) } }

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
                outcome.index?.let { added ->
                    withContext(Dispatchers.Default) {
                        if (kindId == KIND_LAND) {
                            placeLandingIfUnplaced(added, at.latitude, at.longitude)
                        } else if (kindId == KIND_TAKEOFF) {
                            placeTakeoff(added, at.latitude, at.longitude)
                        }
                    }
                    edits += 1
                    selected = MapHit.Waypoint(added)
                }
            } else {
                busy = outcome.reason.takeIf { it != blockedReason(insertable) }
                delay(FAILURE_MESSAGE_MS)
                busy = null
            }
        }
    }

    fun onRefusal(label: String? = null, work: () -> String?) {
        busy = label
        scope.launch {
            val refusal = withContext(Dispatchers.Default) { work() }
            busy = refusal
            if (refusal != null) {
                delay(FAILURE_MESSAGE_MS)
            }
            busy = null
        }
    }

    fun onBridge(label: String? = null, done: String? = null, then: (() -> Unit)? = null, work: () -> Boolean) {
        busy = label
        scope.launch {
            val ok = withContext(Dispatchers.Default) { work() }
            if (ok) {
                then?.let {
                    edits += 1
                    it()
                }
                busy = done
                if (done != null) {
                    delay(FAILURE_MESSAGE_MS)
                    busy = null
                }
            } else {
                busy = "${label ?: "That"} did not work"
                delay(FAILURE_MESSAGE_MS)
                busy = null
            }
        }
    }

    val planStatus by mapPath("view.plan")
    val planHasItems = remember(planStatus) { planStatus?.optBoolean("containsItems") == true }
    val globalFrame = remember(planStatus) { planStatus?.takeIf { it.has("globalAltitudeFrame") && !it.isNull("globalAltitudeFrame") }?.optInt("globalAltitudeFrame") }
    val planOffline = remember(planStatus) { planStatus?.optBoolean("offline") == true }
    val planDirty = remember(planStatus) { planStatus?.optBoolean("dirty") == true }
    val planSyncing = remember(planStatus) {
        planStatus?.optJSONObject("sync")?.optText("state") == "busy"
    }
    val support = planSupport(planStatus)
    var uploadAsk by remember { mutableStateOf<UploadGate?>(null) }
    val vehiclesJson by mapPath(VEHICLES_VIEW)
    val flown = remember(vehiclesJson) { vehicleChoices(vehiclesJson).active }
    val latitude = flown?.latitude ?: Double.NaN
    val longitude = flown?.longitude ?: Double.NaN
    var patternWanted by remember { mutableStateOf<List<MissionKind>>(emptyList()) }
    val missionSummaryView by mapPath("view.missionSummary")
    val terrainView by mapPath(TERRAIN_VIEW)
    val profile = remember(terrainView) { terrainProfile(terrainView) }
    val terrainHits = remember(terrainView) { collisionLegs(terrainView) }
    val collidingPatterns = remember(terrainView) { collidingItems(terrainView) }
    val collidingSimple = remember(terrainView) { collidingItems(terrainView, "collidingSimpleItems") }
    val elevationProviderJson by mapPath(ELEVATION_PROVIDER)
    val elevationNotice = elevationProviderJson?.optText("value").orEmpty()
    val missionStatusJson by mapPath("$SHOW_MISSION_ITEM_STATUS.rawValue")
    val missionStatusShown = missionItemStatusShown(missionStatusJson)

    fun placeAt(): TrackPoint? =
        centre?.takeIf { isPlottable(it.latitude, it.longitude) }
            ?: TrackPoint(latitude, longitude).takeIf { isPlottable(latitude, longitude) }

    var visible by remember { mutableStateOf<List<TrackPoint>>(emptyList()) }
    val context = LocalContext.current
    var importInto by remember { mutableStateOf<ShapeTarget?>(null) }
    val polygonFile = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenMultipleDocuments(),
    ) { uris ->
        val target = importInto
        importInto = null
        if (uris.isNotEmpty() && target != null) {
            scope.launch {
                withContext(Dispatchers.Default) { importShapeFiles(context, uris, target) }?.let { say(it) }
            }
        }
    }
    var tracing by remember { mutableStateOf<Pair<ShapeTarget, List<TrackPoint>>?>(null) }
    var firstRead by remember { mutableStateOf(true) }
    var listOpen by remember { mutableStateOf(false) }
    var centreRequest by remember { mutableIntStateOf(0) }
    var centreOn by remember { mutableStateOf<TrackPoint?>(null) }
    fun pickRow(row: ItemRow) {
        selected = MapHit.Waypoint(row.index)
        items.firstOrNull { it.index == row.index }?.let { placed ->
            centreOn = TrackPoint(placed.latitude, placed.longitude)
            centreRequest += 1
            follow = false
        }
    }
    var centredOnEntry by remember { mutableStateOf(false) }
    LaunchedEffect(isPlottable(latitude, longitude)) {
        if (centersOnVehicleAtEntry(centredOnEntry, isPlottable(latitude, longitude), fitRequest)) {
            centredOnEntry = true
            centreOn = TrackPoint(latitude, longitude)
            centreRequest += 1
        }
    }

    LaunchedEffect(fitKey) {
        if (fitKey != 0) firstRead = true
    }

    val selectedSequence = selectionSequence(selected, allItems)

    LaunchedEffect(Unit) {
        PlanFocus.requests.collect { index ->
            index?.let {
                selected = MapHit.Waypoint(it)
                PlanFocus.requests.value = null
            }
        }
    }

    LaunchedEffect(selected, selectedSequence) {
        val sequence = selectedSequence ?: 0.takeIf { selected == null } ?: return@LaunchedEffect
        withContext(Dispatchers.Default) { PlanBridge.selectSequence(sequence) }
    }

    suspend fun refresh() {
        val readAt = edits
        withContext(Dispatchers.Default) {
            val plan = PlanBridge.rawItems()
            if (plan != null) {
                MapBridge.markReachable()
            }
            val nextAll = allMissionItems(plan)
            val nextItems = nextAll.filter { it.placed }
            val nextItemCount = planItemCount(plan)
            val nextShape = planShape(plan)
            val nextLink = linksStartToHome(plan)
            val fenceView = FenceBridge.read()
            val nextFences = fencePolygons(fenceView)
            val nextRally = rallyPoints(fenceView)
            val nextOperator = operatorPoint(OperatorBridge.read())
            val nextCircles = fenceCircles(fenceView)
            val nextFirmware = firmwareFence(fenceView)
            val nextBreach = breachReturn(fenceView)
            val nextSurveys = SurveyBridge.surveysFrom(plan)
            val nextLandings = landingPatterns(nextAll)
            val nextStats = surveyStatsFor(nextAll)
            val drawn = planIsDrawn(nextItems, nextSurveys, nextFences, nextCircles, nextRally)
            withContext(Dispatchers.Main) {
                if (fitsPlanOnEntry(firstRead, plan != null, drawn)) {
                    follow = false
                    fitRequest += 1
                }
                firstRead = stillFirstRead(firstRead, plan != null)
                if (!firstRead) centredOnEntry = true
                allItems = nextAll
                items = nextItems
                itemCount = nextItemCount
                shape = nextShape
                linkStartToHome = nextLink
                fences = nextFences
                rally = nextRally
                operator = nextOperator
                if (readAt == edits && !selectionSurvives(
                        selected, nextAll, nextFences, nextCircles, nextRally, nextSurveys, nextLandings,
                        breach = nextBreach != null,
                    )
                ) {
                    selected = null
                }
                circles = nextCircles
                firmware = nextFirmware
                breach = nextBreach
                surveyList = nextSurveys
                landingList = nextLandings
                surveyStatsMap = nextStats
            }
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

    val uploadBlocked = syncRefusal(vehicleSyncState(planOffline, planSyncing), "upload to") != null
    val upload: () -> Unit = {
        val refusal = syncRefusal(
            vehicleSyncState(planOffline, planSyncing), "upload to",
        )
        if (refusal != null) {
            say(refusal)
        } else {
            scope.launch {
                val view = withContext(Dispatchers.Default) { freshPlanView() }
                when (val step = uploadStep(uploadGate(view), notReadyToSend(view))) {
                    is UploadStep.Refuse -> {
                        PlanFocus.notReady(view)
                        say(step.reason)
                    }
                    is UploadStep.Confirm -> uploadAsk = step.gate
                    UploadStep.Send -> sendPlan(scope, say = { busy = it }, done = { busy = null })
                }
            }
        }
    }

    Column(Modifier.fillMaxSize()) {
    val uploadText = uploadLabel(planOffline, planSyncing, planDirty, planHasItems)
    val uploadEnabled = planHasItems && !planOffline && !planSyncing
    header?.invoke(PlanUpload(enabled = uploadEnabled, emphasised = !uploadBlocked, label = uploadText, shown = !planOffline, onClick = upload))
    Box(Modifier.fillMaxWidth().weight(1f)) {
        VehicleMap(
            modifier = Modifier.fillMaxSize(),
            mapStyle = shownStyle,
            follow = follow,
            missionItems = items.map { it.copy(terrainCollision = it.index in collidingSimple) },
            linkStartToHome = linkStartToHome,
            fencePolygons = fences.map { if (ownerOf(selected) == "p${it.index}") it else it.copy(editable = null) },
            fenceCircles = circles,
            firmwareFence = firmware,
            breachReturn = breach?.point,
            rallyPoints = rally,
            operator = operator,
            surveys = surveyList.map { if (ownerOf(selected) == "m${it.index}") it else it.copy(editable = null) }.map { it.copy(collides = it.index in collidingPatterns) },
            landings = landingList.map { it.copy(collides = it.index in collidingPatterns) },
            editable = true,
            circledShapes = circled,
            collisionLegs = terrainHits,
            onAdd = { lat, lon ->
                tracing?.let { (target, points) ->
                    tracing = target to points + TrackPoint(lat, lon)
                    return@VehicleMap
                }
                when (layer) {
                    PlanLayer.Mission -> addMissionItem(
                        KIND_WAYPOINT, "Adding a waypoint", TrackPoint(lat, lon),
                        insertAfter(selected, allItems),
                    )
                    PlanLayer.Rally -> if (support.rally) {
                        val next = rally.size
                        onBridge("Adding rally", then = { selected = MapHit.Rally(next) }) { FenceBridge.addRallyPoint(lat, lon) }
                    }
                    PlanLayer.Fence -> Unit
                }
            },
            canDrag = { hit -> dragAllowed(hit, selected, layer) },
            onMove = { hit, lat, lon ->
                val generation = moveGeneration()
                onBridge { writeDragStep(generation, hit, lat, lon, surveyList, rally, fences, allItems, circles) }
            },
            onWaypointSelected = { hit ->
                if (hit != null && actsOnTap(hit) && !dragAllowed(hit, selected, layer)) Unit
                else when (hit) {
                    is MapHit.Midpoint -> if (hit.path == MISSION_SPLIT_PATH) {
                        addMissionItem(KIND_WAYPOINT, "Adding a waypoint", legSplit(allItems.filter { it.placed }, hit.segment), hit.segment)
                    } else onBridge("Adding a corner") {
                        // qtpaths: plan.geoFenceController.polygons.0.splitPolygonSegment, plan.missionController.visualItems.0.surveyAreaPolygon.splitPolygonSegment, plan.missionController.visualItems.0.corridorPolyline.splitSegment
                        invokeOk("${hit.path}.${hit.invokable}", "[${hit.segment}]")
                    }
                    MapHit.BreachReturn -> editingBreach = true
                    is MapHit.LoiterRotation -> allItems.firstOrNull { it.index == hit.index }?.let { item ->
                        onBridge(done = movedText(hit, allItems)) { PlanBridge.setLoiterRadius(item.index, -item.loiterRadius) }
                    }
                    is MapHit.LoiterRadius -> selected = MapHit.Waypoint(hit.index)
                    is MapHit.CircleRadius -> selected = MapHit.Circle(hit.index)
                    else -> selected = hit
                }
            },
            onMoved = { hit, lat, lon ->
                onBridge(done = movedText(hit, allItems)) { writeMove(hit, lat, lon, surveyList, rally, fences, allItems, circles) }
            },
            selectedWaypoint = (selected as? MapHit.Waypoint)?.index,
            onViewChanged = { visible = it },
            tracePoints = tracing?.second.orEmpty(),
            traceLine = tracing?.first?.line == true,
            onCentreChanged = { at, level ->
                centre = at
                zoom = level
                onCentre?.invoke(at.latitude, at.longitude)
            },
            bottomInsetPx = controlsHeightPx,
            topInsetPx = topOverlayPx,
            leftInsetPx = sidePanelPx,
            fitRequest = fitRequest,
            fitOnly = fitOnly,
            onFitFailed = { onBridge("Fitting the plan") { false } },
            centreRequest = centreRequest,
            centreOn = centreOn,
        )

        radiusFor?.let { target ->
            val vertices = shapeVertices(target, fences, surveyList)
            val shape = shapeEditable(target, fences, surveyList)
            RadiusDialog(
                circleRadius(vertices) ?: 0.0,
                shape?.distanceUnit ?: "m",
                shape?.metresPerUnit ?: 1.0,
                onDismiss = { radiusFor = null },
            ) { radius ->
                radiusFor = null
                circleAround(vertices, radius)?.let { ring ->
                    onBridge("Changed the circle radius") { replaceShape(target, ring) }
                }
            }
        }

        positioning?.let { (hit, at) ->
            PositionDialog(at, onDismiss = { positioning = null }) { moved ->
                positioning = null
                onBridge(done = movedText(hit, allItems)) { writeMove(hit, moved.latitude, moved.longitude, surveyList, rally, fences, allItems, circles) }
            }
        }

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

        overlay?.let { content ->
            val inset = with(LocalDensity.current) { controlsHeightPx.toDp() }
            Box(Modifier.fillMaxSize().padding(start = if (sidePanel) SIDE_PANEL_WIDTH else 0.dp, bottom = if (sidePanel) 0.dp else inset), content = content)
        }

        val listedInPanel = sidePanel && layer == PlanLayer.Mission && worthListing(allItems)
        if (busy != null || !summaryHidden) Column(
            Modifier.align(Alignment.TopStart).padding(start = if (sidePanel) SIDE_PANEL_WIDTH + 8.dp else 8.dp, top = 8.dp, end = 8.dp, bottom = 8.dp).fillMaxWidth(SUMMARY_MAX_FRACTION),
        ) {
            if (busy != null || !listedInPanel) Surface(
                color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.94f),
                shape = MaterialTheme.shapes.extraLarge,
                onClick = { listOpen = true },
                enabled = worthListing(allItems) && !sidePanel,
            ) {
                val ready by MapBridge.bridgeReady.collectAsState()
                Row(
                    Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        busy ?: if (!ready) {
                            "Waiting for QGroundControl"
                        } else {
                            planSummary(
                                itemCount, shape, items, fences, circles, rally, surveyList,
                                missionSummaryText(missionSummaryView), selected, planOffline,
                                canAddByHand = kindAllows(insertable, KIND_WAYPOINT),
                                canPlaceByButton = placeAt() != null,
                            )
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f, fill = false),
                    )
                    if (worthListing(allItems) && !sidePanel) {
                        Icon(
                            Icons.AutoMirrored.Filled.List,
                            contentDescription = "Show the plan as a list",
                            Modifier.size(18.dp),
                        )
                    }
                }
            }

            centre?.let { at ->
                ScaleBarView(at.latitude, zoom, Modifier.padding(start = 4.dp, top = 8.dp))
            }
        }

        val density = LocalDensity.current
        val waypointAt = placeAt()
        if (waypointAt != null && kindAllows(insertable, KIND_WAYPOINT) && tracing == null && !summaryHidden) {
            ExtendedFloatingActionButton(
                onClick = { addMissionItem(KIND_WAYPOINT, "Adding a waypoint", placeAt(), insertAfter(selected, allItems)) },
                icon = { Icon(Icons.Default.Add, null) },
                text = { Text("Add waypoint") },
                containerColor = MaterialTheme.colorScheme.primaryContainer,
                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 16.dp, bottom = with(density) { controlsHeightPx.toDp() } + 16.dp)
                    .semantics { contentDescription = "Add waypoint" },
            )
        }

        Surface(
            if (sidePanel) {
                Modifier.align(Alignment.TopStart).width(SIDE_PANEL_WIDTH).fillMaxHeight()
            } else {
                Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                    .onGloballyPositioned { controlsHeightPx = it.size.height }
            },
            shape = if (sidePanel) RectangleShape else RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
            color = MaterialTheme.colorScheme.surfaceContainerLow,
        ) {
            Column(
                Modifier.padding(horizontal = 12.dp)
                    .then(if (sidePanel) Modifier.fillMaxHeight().padding(top = 12.dp) else Modifier.heightIn(max = CONTROLS_MAX_HEIGHT))
                    .verticalScroll(rememberScrollState()),
            ) {
                if (!sidePanel) Box(
                    Modifier
                        .align(Alignment.CenterHorizontally)
                        .padding(vertical = 10.dp)
                        .size(width = 32.dp, height = 4.dp)
                        .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f), CircleShape),
                )
                (selected as? MapHit.Waypoint)
                    ?.let { hit -> allItems.firstOrNull { it.index == hit.index } }
                    ?.let { item ->
                        Row(
                            Modifier.fillMaxWidth().padding(start = 12.dp, end = 4.dp, bottom = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text(
                                    "${sentenceCase(item.command.ifBlank { "Item" })} ${sequenceLabel(item)}",
                                    style = MaterialTheme.typography.titleLarge,
                                )
                                listOfNotNull(itemPlace(item, allItems), sheetDetail(item, surveyStatsMap[item.index]).ifBlank { null }).joinToString(" \u00b7 ").ifBlank { null }?.let {
                                    Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                            }
                            Button(onClick = { selected = null }, contentPadding = ButtonDefaults.ButtonWithIconContentPadding) {
                                Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(ButtonDefaults.IconSize))
                                Spacer(Modifier.width(ButtonDefaults.IconSpacing))
                                Text("Done")
                            }
                        }
                        surveyTiles(item, surveyStatsMap[item.index]).takeIf { it.isNotEmpty() }?.let { tiles ->
                            Row(Modifier.padding(start = 12.dp, end = 12.dp, bottom = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                tiles.forEach { (label, value) -> StatTile(label, value) }
                            }
                        }
                    }
                SingleChoiceSegmentedButtonRow(Modifier.padding(bottom = 8.dp)) {
                    PlanLayer.entries.forEach { option ->
                        SegmentedButton(
                            selected = layer == option,
                            onClick = { layer = option },
                            shape = SegmentedButtonDefaults.itemShape(option.ordinal, PlanLayer.entries.size),
                        ) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text(option.label)
                                layerSubtitle(option, itemCount, rally.size).ifBlank { null }?.let {
                                    Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                            }
                        }
                    }
                }
                if (listedInPanel) {
                    val rows = itemRows(allItems, surveyStatsMap)
                    ItemListHeading(rows, missionSummaryText(missionSummaryView), Modifier.padding(horizontal = 12.dp, vertical = 4.dp))
                    rows.forEach { row ->
                        ItemRowView(row, selected = (selected as? MapHit.Waypoint)?.index == row.index) { pickRow(row) }
                    }
                }
                if (layer == PlanLayer.Rally) {
                    if (!support.rally && support.reason.isNotBlank()) PaletteNote(support.reason)
                    if (rally.isEmpty()) PaletteNote(RALLY_HELP)
                    rallyRows(rally).forEach { row ->
                        FenceListRow(
                            row,
                            chosen = (selected as? MapHit.Rally)?.index == row.index,
                            onSelect = { selected = MapHit.Rally(row.index) },
                        ) {
                            val count = rally.size
                            onBridge("Removing ${row.title.lowercase()}", then = { selected = rallyAfterRemove(row.index, count) }) { FenceBridge.removeRallyPoint(row.index) }
                        }
                    }
                }
                if (layer == PlanLayer.Fence) {
                    if (!support.fence && support.reason.isNotBlank()) PaletteNote(support.reason)
                    if (fences.isEmpty() && circles.isEmpty()) PaletteNote(NO_GEOFENCE)
                    val listed = fenceRows(fences, circles)
                    listed.forEachIndexed { at, row ->
                        fenceHeading(row, listed.getOrNull(at - 1))?.let { FenceHeading(it) }
                        FenceListRow(
                            row,
                            chosen = rowSelected(row, selected),
                            onSelect = { selected = fenceRowHit(row) },
                            onInclusion = row.inclusion?.let { { keep: Boolean -> onBridge("Changing ${row.title.lowercase()}") { FenceBridge.setPolygonInclusion(row.index, keep) } } },
                        ) {
                            onBridge("Removing ${row.title.lowercase()}", then = { selected = fenceSelectionAfterRemove(row, selected) }) {
                                if (row.circle) FenceBridge.deleteCircle(row.index) else FenceBridge.deletePolygon(row.index)
                            }
                        }
                    }
                }
                FlowRow(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    fun download() {
                        loadArmed = false
                        busy = "Downloading from vehicle"
                        selected = null
                        scope.launch {
                            val outcome = withContext(Dispatchers.Default) {
                                uploadOutcome(PlanBridge.loadFromVehicle())
                            }
                            busy = downloadMessage(outcome)
                            delay(FAILURE_MESSAGE_MS)
                            busy = null
                        }
                    }
                    FilledTonalButton(onClick = {
                        val refusal = syncRefusal(
                            vehicleSyncState(planOffline, planSyncing), "download from",
                        )
                        when {
                            refusal != null -> say(refusal)
                            loadStep(planDirty, loadArmed) == LoadStep.Confirm -> loadArmed = true
                            else -> download()
                        }
                    }) { Text("Download") }
                    if (loadArmed) {
                        AlertDialog(
                            onDismissRequest = { loadArmed = false },
                            title = { Text("Load plan from vehicle?") },
                            text = { Text(replaceWarning(allItems.count { it.index != HOME_ITEM })) },
                            confirmButton = { TextButton(onClick = ::download) { Text("Replace") } },
                            dismissButton = { TextButton(onClick = { loadArmed = false }) { Text("Keep mine") } },
                        )
                    }

                    if (header == null && !planOffline) {
                        PlanUploadButton(emphasised = !uploadBlocked, enabled = uploadEnabled, onClick = upload, contentPadding = PRIMARY_PADDING) { Text(uploadText) }
                    }

                    uploadAsk?.let { gate ->
                        AlertDialog(
                            onDismissRequest = { uploadAsk = null },
                            title = { Text(uploadHeading(gate, vehicleChoices(vehiclesJson))) },
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

                    if (layer == PlanLayer.Fence) FilledTonalButton(enabled = support.fence, onClick = {
                        val at = placeAt()
                        val next = fences.size
                        onBridge("Adding fence", then = { selected = MapHit.FenceVertex(next, 0) }) {
                            at?.let { fenceWindow(visible, it) }?.let { (topLeft, bottomRight) -> FenceBridge.addInclusionPolygon(topLeft, bottomRight) } ?: false
                        }
                    }) { Text("Polygon fence") }

                    if (layer == PlanLayer.Mission) addingAfterText(selected, allItems)?.let {
                        PaletteNote(it)
                    }

                    if (layer == PlanLayer.Mission) FilledTonalButton(
                        enabled = kindAllows(insertable, KIND_SURVEY),
                        onClick = { patternWanted = scanPatterns(insertable) },
                    ) { Text("Pattern") }

                    if (patternWanted.isNotEmpty()) {
                        AlertDialog(
                            onDismissRequest = { patternWanted = emptyList() },
                            title = { Text("Which pattern?") },
                            text = {
                                Text(
                                    patternWanted.firstOrNull { !it.enabled && it.disabledReason.isNotBlank() }
                                        ?.disabledReason
                                        ?: "A pattern covers an area or a line with a camera run.",
                                )
                            },
                            confirmButton = {
                                Column {
                                    patternWanted.forEach { kind ->
                                        TextButton(
                                            enabled = kind.enabled,
                                            onClick = {
                                                patternWanted = emptyList()
                                                addMissionItem(
                                                    kind.id,
                                                    "Adding ${kind.label.lowercase()}",
                                                    placeAt(),
                                                    insertAfter(selected, allItems),
                                                )
                                            },
                                        ) { Text(kind.label) }
                                    }
                                }
                            },
                            dismissButton = {
                                TextButton(onClick = { patternWanted = emptyList() }) { Text("Cancel") }
                            },
                        )
                    }

                    if (layer == PlanLayer.Fence) FilledTonalButton(enabled = support.fence, onClick = {
                        val at = placeAt()
                        val next = circles.size
                        onBridge("Adding circle", then = { selected = MapHit.Circle(next) }) {
                            at?.let { fenceWindow(visible, it) }?.let { (topLeft, bottomRight) -> FenceBridge.addInclusionCircle(topLeft, bottomRight) } ?: false
                        }
                    }) { Text("Circular fence") }

                    if (layer == PlanLayer.Fence) FilledTonalButton(enabled = support.fence, onClick = {
                        if (breach != null) {
                            editingBreach = true
                        } else {
                            val at = placeAt()
                            onBridge("Adding breach return point") {
                                at != null && FenceBridge.setBreachReturn(at)
                            }
                        }
                    }) { Text(if (breach == null) "Add breach return point" else "Breach return point") }

                    breach?.takeIf { editingBreach }?.let { current ->
                        BreachReturnDialog(
                            breach = current,
                            onDismiss = { editingBreach = false },
                            onAltitude = { shown ->
                                editingBreach = false
                                onBridge("Setting breach return altitude") { FenceBridge.setBreachAltitude(current.altitudePath, shown) }
                            },
                            onRemove = {
                                editingBreach = false
                                onBridge("Removing breach return point") { FenceBridge.clearBreachReturn() }
                            },
                        )
                    }

                    if (layer == PlanLayer.Rally) FilledTonalButton(enabled = support.rally, onClick = {
                        val at = placeAt()
                        val next = rally.size
                        onBridge("Adding rally", then = { selected = MapHit.Rally(next) }) {
                            at != null && FenceBridge.addRallyPoint(at.latitude, at.longitude)
                        }
                    }) { Text("Add rally point") }
                    if (layer == PlanLayer.Mission && kindOffered(insertable, KIND_TAKEOFF)) FilledTonalButton(
                        enabled = kindAllows(insertable, KIND_TAKEOFF),
                        onClick = {
                            val at = placeAt()
                            addMissionItem(
                                "takeoff",
                                "Adding a takeoff",
                                at,
                                if (takeoffMissing(items)) {
                                    BEFORE_THE_REST
                                } else {
                                    insertAfter(selected, allItems)
                                },
                            )
                        },
                    ) { Text("Takeoff") }
                    if (layer == PlanLayer.Mission) FilledTonalButton(
                        enabled = kindAllows(insertable, KIND_LAND),
                        onClick = {
                            val at = placeAt()
                            addMissionItem(KIND_LAND, "Adding a landing", at, insertAfter(selected, allItems))
                        },
                    ) { Text(kindLabel(insertable, KIND_LAND)) }

                    if (layer == PlanLayer.Mission) blockedReason(insertable)?.let {
                        PaletteNote(it)
                    }

                    CenterMenu(
                        launch = allItems.firstOrNull { it.sequence == 0 }?.let { TrackPoint(it.latitude, it.longitude) },
                        myLocation = operator,
                        onFit = { points ->
                            follow = false
                            fitOnly = points
                            fitRequest += 1
                        },
                        onCentre = { point ->
                            follow = false
                            centreOn = point
                            centreRequest += 1
                        },
                        missionPoints = missionFitPoints(allItems),
                    )

                    MapTypeMenu { shownStyle = it }

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

                tracing?.let { (target, points) ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(traceCaption(points.size, target.minimum), style = MaterialTheme.typography.labelSmall)
                        TextButton(enabled = points.size >= target.minimum, onClick = {
                            tracing = null
                            onBridge("Tracing shape") { replaceShape(target, points) }
                        }) { Text("Done") }
                        TextButton(onClick = { tracing = null }) { Text("Cancel") }
                    }
                }

                var cameraMenuFor by remember { mutableStateOf<Int?>(null) }
                val survey = selectedSurvey(selected, surveyList)
                val waypoint = (selected as? MapHit.Waypoint)
                    ?.let { hit -> allItems.firstOrNull { it.index == hit.index } }
                val shapeFence = (selected as? MapHit.ShapeCentre)?.takeIf { it.fence }?.owner ?: (selected as? MapHit.ShapeRadius)?.takeIf { it.fence }?.owner
                val fenceHit = selected as? MapHit.FenceVertex
                val surveyHit = selected as? MapHit.SurveyVertex
                val rallyHit = selected as? MapHit.Rally
                val circleIndex = (selected as? MapHit.Circle)?.index
                    ?: (selected as? MapHit.CircleCentre)?.index
                val circle = circleIndex?.let { index -> circles.firstOrNull { it.index == index } }

                if (survey != null || waypoint != null || fenceHit != null || shapeFence != null ||
                    rallyHit != null || circle != null
                ) {
                    FlowRow(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        selectedFence(selected, fences, circles)?.let { fence ->
                            fence.flip?.let { flip ->
                                TextButton(onClick = {
                                    onBridge(
                                        if (fence.keepsIn) "Making it keep-out" else "Making it keep-in",
                                    ) { flip() }
                                }) { Text(if (fence.keepsIn) "Make keep-out" else "Make keep-in") }
                            }
                        }

                        fenceDetail(selected, fences, circles)?.let {
                            PaletteNote(it)
                        }

                        waypoint?.let { item ->
                            if (itemEditor != null) {
                                FilledTonalButton(onClick = { editingItem = item }) { Text("Edit item") }
                            }
                        }

                        landingText(selectedLanding(selected, landingList))?.let {
                            PaletteNote(it)
                        }

                        survey?.let { cameraText(surveyStatsMap[it.index]) }?.let {
                            PaletteNote(it)
                        }

                        layersText(survey)?.let {
                            PaletteNote(it)
                        }

                        survey?.takeIf { it.kind == KIND_SURVEY }?.let { grid ->
                            var angle by remember(grid.index) { mutableStateOf<Float?>(null) }
                            LaunchedEffect(grid.index) {
                                angle = withContext(Dispatchers.Default) { gridAngleShown(SurveyBridge.gridAngle(grid.index)) }
                            }
                            angle?.let { shown ->
                                Column(Modifier.width(220.dp)) {
                                    Text("Angle ${shown.toInt()}°", style = MaterialTheme.typography.labelSmall)
                                    Slider(
                                        value = shown,
                                        onValueChange = { angle = Math.round(it).toFloat() },
                                        onValueChangeFinished = { angle?.let { chosen -> onBridge("Setting the grid angle") { SurveyBridge.setGridAngle(grid.index, chosen.toDouble()) } } },
                                        valueRange = 0f..GRID_ANGLE_MAX,
                                    )
                                }
                            }
                        }

                        survey?.takeIf { visible.size == it.area.size }?.let {
                            TextButton(onClick = {
                                onBridge("Sizing the area", done = "Area sized to the view") {
                                    fitSurveyArea(it, insetRing(visible, SURVEY_FIT_INSET))
                                }
                            }) { Text("Size to view") }
                        }

                        waypoint?.let { legText(it) }?.let {
                            PaletteNote(it)
                        }

                        waypoint?.let { item ->
                            if (!item.altitude.isNaN()) {
                                var typed by remember(item.index, item.altitude) {
                                    mutableStateOf(altitudeFieldText(item.altitude, WAYPOINT_ALTITUDE_DECIMALS))
                                }
                                OutlinedTextField(
                                    value = typed,
                                    onValueChange = { typed = it },
                                    label = { Text(altitudeFieldLabel(item)) },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(
                                        keyboardType = KeyboardType.Text,
                                        imeAction = ImeAction.Done,
                                    ),
                                    keyboardActions = KeyboardActions(
                                        onDone = {
                                            val shown = parsedAltitude(typed)
                                            if (shown == null) {
                                                say("Not an altitude")
                                            } else {
                                                onBridge("Setting altitude") {
                                                    PlanBridge.setAltitude(item.index, shown)
                                                }
                                            }
                                        },
                                    ),
                                    modifier = Modifier.width(altitudeFieldWidth(item)),
                                    textStyle = MaterialTheme.typography.bodySmall,
                                )

                                FilledTonalButton(onClick = {
                                    onBridge { PlanBridge.setAltitude(item.index, item.altitude + 10.0) }
                                }) { Text("+10") }

                                FilledTonalButton(
                                    onClick = {
                                        onBridge { PlanBridge.setAltitude(item.index, item.altitude - 10.0) }
                                    },
                                ) { Text("-10") }

                                WaypointSpeedField(item.index, onWrite = { label, work -> onBridge(label) { work() } }, onRefused = { say(it) })

                                if (itemReferenceShown(globalFrame)) AltitudeModePicker(
                                    item = item,
                                    onPick = { raw ->
                                        onBridge("Setting the altitude frame") {
                                            PlanBridge.setAltitudeMode(item.index, raw)
                                        }
                                    },
                                    modifier = Modifier.fillMaxWidth(),
                                    globalFrameMixed = itemReferenceSelectable(globalFrame),
                                )
                            }

                            var cameraRevision by remember(item.index) { mutableStateOf(0) }
                            val camera by produceState<JSONObject?>(null, item.index, cameraRevision) {
                                value = withContext(Dispatchers.Default) {
                                    ItemCameraBridge.read(item.index)
                                }
                            }
                            val cameraPicker = cameraChoices(camera)
                                ?.let { it.labels.getOrNull(it.chosen) }
                            itemCameraTextBeside(camera, cameraPicker)?.let {
                                PaletteNote(it)
                            }
                            cameraChoices(camera)?.let { choices ->
                                Box {
                                    OutlinedTextField(
                                        value = choices.labels.getOrElse(choices.chosen) { "…" },
                                        onValueChange = {},
                                        readOnly = true,
                                        label = { Text("At this point") },
                                        trailingIcon = { Icon(Icons.Default.ArrowDropDown, null) },
                                        singleLine = true,
                                        textStyle = MaterialTheme.typography.bodySmall,
                                        modifier = Modifier.width(AT_THIS_POINT_WIDTH),
                                    )
                                    Box(Modifier.matchParentSize().clickable { cameraMenuFor = item.index })
                                    DropdownMenu(
                                        expanded = cameraMenuFor == item.index,
                                        onDismissRequest = { cameraMenuFor = null },
                                    ) {
                                        choices.labels.forEachIndexed { at, label ->
                                            DropdownMenuItem(
                                                text = { Text(label) },
                                                onClick = {
                                                    cameraMenuFor = null
                                                    onBridge("Setting the camera action") {
                                                        ItemCameraBridge.chooseAction(item.index, at)
                                                    }
                                                    cameraRevision += 1
                                                },
                                            )
                                        }
                                    }
                                }
                            }
                            WaypointHoldField(item.index, onWrite = { label, work -> onBridge(label) { work() } }, onRefused = { say(it) })
                            cameraChoices(camera)?.let {
                                cameraExtras(camera)?.let { extras ->
                                    CameraSectionExtras(extras) { member, value ->
                                        onBridge("Setting the camera") { ItemCameraBridge.set(item.index, member, value) }
                                        cameraRevision += 1
                                    }
                                }
                                itemCameraNote(camera)?.let { PaletteNote(it) }
                            }
                            if (item.index > HOME_ITEM) {
                                TextButton(onClick = {
                                    scope.launch {
                                        busy = "Removing #${item.sequence}"
                                        val outcome = withContext(Dispatchers.Default) {
                                            removeMissionItem(item.index)
                                        }
                                        selected = if (outcome.ok) selectionAfterRemove(item.index, allItems.size) else null
                                        if (outcome.ok) {
                                            busy = null
                                        } else {
                                            busy = outcome.reason
                                            delay(FAILURE_MESSAGE_MS)
                                            busy = null
                                        }
                                    }
                                }, colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error)) { Text(deleteLabel(item)) }
                            }
                        }

                        listOfNotNull(fenceHit, surveyHit).firstOrNull()?.let { hit ->
                            cornerPosition(hit, fences, surveyList)?.let { at ->
                                TextButton(onClick = { positioning = hit to at }) { Text("Edit position") }
                            }
                        }

                        shapeTarget(fenceHit?.polygon ?: shapeFence, survey?.takeIf { surveyHit != null || selected is MapHit.ShapeCentre || selected is MapHit.ShapeRadius })?.let { target ->
                            shapeEditable(target, fences, surveyList)?.let { shape ->
                                Text(shapeCaption(shape, target.path in circled), style = MaterialTheme.typography.labelSmall)
                            }
                            if (target.line) {
                                TextButton(enabled = visible.size == 4, onClick = {
                                    onBridge("Drawing line") { replaceShape(target, defaultLine(visible)) }
                                }) { Text("Line") }
                            } else {
                                TextButton(enabled = visible.size == 4, onClick = {
                                    chosenCircles = chosenCircles - target.path
                                    onBridge("Drawing rectangle") { replaceShape(target, defaultRectangle(visible)) }
                                }) { Text("Rectangle") }
                                TextButton(enabled = visible.size == 4, onClick = {
                                    chosenCircles = chosenCircles + target.path
                                    onBridge("Drawing circle") { replaceShape(target, defaultCircle(visible)) }
                                }) { Text("Circle") }
                                if (target.path in circled) {
                                    TextButton(onClick = { radiusFor = target }) { Text("Set radius\u2026") }
                                }
                            }
                            TextButton(onClick = { tracing = target to emptyList() }) { Text("Trace") }
                            TextButton(onClick = {
                                chosenCircles = chosenCircles - target.path
                                importInto = target
                                polygonFile.launch(arrayOf("*/*"))
                            }) { Text("Import\u2026") }
                        }

                        fenceHit?.let { hit ->
                            if (cornerRemovable(fences.firstOrNull { it.index == hit.polygon })) {
                                TextButton(onClick = {
                                    onBridge("Removing corner") {
                                        FenceBridge.removeVertex(hit.polygon, hit.vertex)
                                    }
                                    selected = null
                                }) { Text("Remove vertex") }
                            }
                            TextButton(onClick = {
                                onBridge { FenceBridge.deletePolygon(hit.polygon) }
                                selected = null
                            }) { Text("Delete fence") }
                        }

                        surveyHit?.let { hit ->
                            survey?.takeIf { it.editable?.canRemoveVertex == true }?.let { shape ->
                                TextButton(onClick = {
                                    onBridge("Removing corner") { SurveyBridge.removeVertex(shape, hit.vertex) }
                                    selected = null
                                }) { Text("Remove vertex") }
                            }
                            var surveyAlt by remember(hit.item) { mutableStateOf("") }
                            var surveyUnit by remember(hit.item) { mutableStateOf("m") }
                            LaunchedEffect(hit.item) {
                                val shown = withContext(Dispatchers.Default) {
                                    SurveyBridge.altitude(hit.item)
                                }
                                val unit = withContext(Dispatchers.Default) {
                                    SurveyBridge.altitudeUnits(hit.item)
                                }
                                surveyAlt = altitudeFieldText(shown, SURFACE_DISTANCE_DECIMALS)
                                surveyUnit = unit.ifBlank { "m" }
                            }
                            OutlinedTextField(
                                value = surveyAlt,
                                onValueChange = { surveyAlt = it },
                                label = { Text("Above surface $surveyUnit") },
                                singleLine = true,
                                keyboardOptions = KeyboardOptions(
                                    keyboardType = KeyboardType.Decimal,
                                    imeAction = ImeAction.Done,
                                ),
                                keyboardActions = KeyboardActions(
                                    onDone = {
                                        val shown = parsedSurfaceDistance(surveyAlt, metresPerUnit(surveyUnit))
                                        if (shown == null) {
                                            say("Not an altitude")
                                        } else {
                                            onBridge("Setting survey altitude") {
                                                SurveyBridge.setAltitude(hit.item, shown)
                                            }
                                        }
                                    },
                                ),
                                modifier = Modifier.width(150.dp),
                                textStyle = MaterialTheme.typography.bodySmall,
                            )
                        }

                        (surveyHit?.item ?: (selected as? MapHit.ShapeCentre)?.takeIf { !it.fence }?.owner)?.let { item ->
                            TextButton(onClick = {
                                onRefusal { PlanBridge.removeItemRefusal(item) }
                                selected = null
                            }) { Text("Delete ${patternName(item, allItems)}") }
                        }

                        circle?.let { it ->
                            val bigger = grownRadius(it)
                            val smaller = shrunkRadius(it)
                            var typedCircleRadius by remember(it.index, it.radius) { mutableStateOf(trimmedRadius(it.radius)) }

                            OutlinedTextField(
                                value = typedCircleRadius,
                                onValueChange = { typedCircleRadius = it },
                                label = { Text("Radius") },
                                suffix = { Text(it.radiusUnits) },
                                singleLine = true,
                                keyboardOptions = KeyboardOptions(
                                    keyboardType = KeyboardType.Decimal,
                                    imeAction = ImeAction.Done,
                                ),
                                keyboardActions = KeyboardActions(
                                    onDone = {
                                        val wanted = typedRadius(typedCircleRadius, it)
                                        if (wanted == null) say("Not a radius this fence accepts") else onBridge { FenceBridge.setCircleRadius(it.index, wanted) }
                                    },
                                ),
                                modifier = Modifier.width(110.dp),
                                textStyle = MaterialTheme.typography.bodySmall,
                            )

                            TextButton(
                                enabled = bigger != null,
                                onClick = {
                                    bigger?.let { wanted ->
                                        onBridge { FenceBridge.setCircleRadius(it.index, wanted) }
                                    }
                                },
                            ) { Text("Bigger") }

                            TextButton(
                                enabled = smaller != null,
                                onClick = {
                                    smaller?.let { wanted ->
                                        onBridge { FenceBridge.setCircleRadius(it.index, wanted) }
                                    }
                                },
                            ) { Text("Smaller") }

                            TextButton(onClick = {
                                onBridge { FenceBridge.deleteCircle(it.index) }
                                selected = null
                            }) { Text("Delete circle") }
                        }

                        rallyHit?.let { hit ->
                            rally.firstOrNull { it.index == hit.index }?.let { point ->
                                listOf(
                                    Triple("Latitude", point.latitude, LATITUDE_LIMIT),
                                    Triple("Longitude", point.longitude, LONGITUDE_LIMIT),
                                ).forEach { (label, value, limit) ->
                                    var typed by remember(point.index, label, value) { mutableStateOf(value.toString()) }
                                    OutlinedTextField(
                                        value = typed,
                                        onValueChange = { typed = it },
                                        label = { Text(label) },
                                        singleLine = true,
                                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text, imeAction = ImeAction.Done),
                                        keyboardActions = KeyboardActions(
                                            onDone = {
                                                val entered = parsedCoordinate(typed, limit)
                                                if (entered == null) {
                                                    say("Not a $label")
                                                } else {
                                                    val (latitude, longitude) = if (label == "Latitude") entered to point.longitude else point.latitude to entered
                                                    onBridge("Moving rally point") {
                                                        FenceBridge.moveRallyPoint(point.index, latitude, longitude, point.altitudeMetres)
                                                    }
                                                }
                                            },
                                        ),
                                        modifier = Modifier.width(150.dp),
                                        textStyle = MaterialTheme.typography.bodySmall,
                                    )
                                }
                            }
                            rally.firstOrNull { it.index == hit.index }
                                ?.takeIf { rallyAltitudeIsEditable(it) }
                                ?.let { point ->
                                    var typed by remember(point.index, point.altitude) {
                                        mutableStateOf(altitudeFieldText(point.altitude, RALLY_ALTITUDE_DECIMALS))
                                    }
                                    OutlinedTextField(
                                        value = typed,
                                        onValueChange = { typed = it },
                                        label = { Text(rallyAltitudeLabel(point)) },
                                        singleLine = true,
                                        keyboardOptions = KeyboardOptions(
                                            keyboardType = KeyboardType.Text,
                                            imeAction = ImeAction.Done,
                                        ),
                                        keyboardActions = KeyboardActions(
                                            onDone = {
                                                val shown = parsedAltitude(typed)
                                                if (shown == null) {
                                                    say("Not an altitude")
                                                } else {
                                                    onBridge("Setting altitude") {
                                                        FenceBridge.setRallyAltitude(point.altitudePath, shown)
                                                    }
                                                }
                                            },
                                        ),
                                        modifier = Modifier.width(120.dp),
                                        textStyle = MaterialTheme.typography.bodySmall,
                                    )
                                }

                            TextButton(onClick = {
                                val count = rally.size
                                onBridge(then = { selected = rallyAfterRemove(hit.index, count) }) { FenceBridge.removeRallyPoint(hit.index) }
                            }) { Text("Delete rally") }
                        }

                    }
                }

                if (profileShown(layer, profile)) {
                    FilterChip(
                        selected = missionStatusShown,
                        onClick = { onBridge { setOk(SHOW_MISSION_ITEM_STATUS, settingJson((!missionStatusShown).toString())) } },
                        label = { Text("Terrain profile") },
                    )
                }
                if (missionStatusShown && profileShown(layer, profile)) {
                    TerrainProfileView(profile, elevationNotice, selectedSequence = selectedSequence) { sequence ->
                        allItems.firstOrNull { it.sequence == sequence }?.let { selected = MapHit.Waypoint(it.index) }
                    }
                }
            }
        }

        if (listOpen) {
            ModalBottomSheet(onDismissRequest = { listOpen = false }) {
                val rows = itemRows(allItems, surveyStatsMap)
                ItemListHeading(rows, missionSummaryText(missionSummaryView), Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
                LazyColumn(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
                    items(rows, key = { it.index }) { row ->
                        ItemRowView(row, selected = (selected as? MapHit.Waypoint)?.index == row.index) {
                            pickRow(row)
                            listOpen = false
                        }
                    }
                }
            }
        }
    }
    }
    editingItem?.let { item ->
        itemEditor?.invoke(item.index, TrackPoint(item.latitude, item.longitude).takeIf { item.placed }) { editingItem = null }
    }
}

private const val DETAIL_SHARE = 2f

private const val SURVEY_FIT_INSET = 0.8

private const val SHORT_ALTITUDE_LABEL = 10

private fun altitudeFieldWidth(item: MissionItem) =
    if (altitudeFieldLabel(item).length > SHORT_ALTITUDE_LABEL) 180.dp else 120.dp

private val ITEM_MARKER_SIZE = 40.dp

@Composable
private fun ItemListHeading(rows: List<ItemRow>, summary: String, modifier: Modifier) {
    Column(modifier) {
        Text(itemCountText(rows.count { it.index != HOME_ITEM }), style = MaterialTheme.typography.titleMedium)
        summary.ifBlank { null }?.let {
            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun ItemRowView(row: ItemRow, selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        color = if (selected) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surfaceContainerLow,
        contentColor = if (selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurface,
    ) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = 72.dp).padding(start = 16.dp, end = 12.dp, top = 8.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                if (row.readyForSave) Modifier.size(ITEM_MARKER_SIZE).background(Color(android.graphics.Color.parseColor(row.colour)), CircleShape)
                else Modifier.size(ITEM_MARKER_SIZE).border(1.dp, MaterialTheme.aircast.warning, CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                Text(row.number, style = MaterialTheme.typography.labelLarge, color = if (row.readyForSave) MaterialTheme.colorScheme.surface else MaterialTheme.aircast.warning, maxLines = 1)
            }
            Column(Modifier.weight(1f)) {
                Text(sentenceCase(row.name), style = MaterialTheme.typography.titleMedium, maxLines = 1)
                row.detail.ifBlank { null }?.let {
                    Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2)
                }
            }
            if (selected) Text("Selected", style = MaterialTheme.typography.labelMedium)
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

internal const val MAP_TYPES_VIEW = "view.mapTypes"

internal data class MapTypes(val current: String, val types: List<String>, val path: String)

internal fun mapTypes(view: JSONObject?): MapTypes? = view?.takeIf { it.has("types") }?.let {
    val types = it.optJSONArray("types")
    MapTypes(it.optText("current"), (0 until (types?.length() ?: 0)).map { at -> types!!.optString(at) }, it.optText("path"))
}

@Composable
private fun MapTypeMenu(onStyle: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    var listed by remember { mutableStateOf<MapTypes?>(null) }
    val scope = rememberCoroutineScope()
    Box {
        FilledTonalButton(onClick = {
            scope.launch {
                listed = withContext(Dispatchers.Default) { mapTypes(runCatching { JSONObject(QGCBridge.get(MAP_TYPES_VIEW)) }.getOrNull()) }
                open = true
            }
        }) { Text("Map") }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            listed?.types?.forEach { type ->
                DropdownMenuItem(
                    text = { Text(type) },
                    trailingIcon = { if (type == listed?.current) Text("✓") },
                    onClick = {
                        open = false
                        val path = listed?.path ?: return@DropdownMenuItem
                        scope.launch {
                            val style = withContext(Dispatchers.Default) {
                                setOk(path, settingJson(JSONObject.quote(type)))
                                qgcRasterStyle(currentMapType())
                            }
                            onStyle(style)
                        }
                    },
                )
            }
        }
    }
}

fun missionFitPoints(items: List<MissionItem>): List<TrackPoint> =
    items.filter { isPlottable(it.latitude, it.longitude) }.map { TrackPoint(it.latitude, it.longitude) }

fun parsedCoordinate(latitude: String, longitude: String): TrackPoint? {
    val lat = parsedCoordinate(latitude, LATITUDE_LIMIT) ?: return null
    val lon = parsedCoordinate(longitude, LONGITUDE_LIMIT) ?: return null
    return TrackPoint(lat, lon)
}

fun cornerPosition(hit: MapHit, fences: List<FencePolygon>, surveys: List<Survey>): TrackPoint? = when (hit) {
    is MapHit.FenceVertex -> fences.firstOrNull { it.index == hit.polygon }?.vertices?.getOrNull(hit.vertex)
    is MapHit.SurveyVertex -> surveys.firstOrNull { it.index == hit.item }?.area?.getOrNull(hit.vertex)
    else -> null
}

@Composable
private fun RadiusDialog(radius: Double, unit: String, metresPerUnit: Double, onDismiss: () -> Unit, onSet: (Double) -> Unit) {
    var text by remember(radius, metresPerUnit) { mutableStateOf(String.format(java.util.Locale.US, "%.1f", radius / metresPerUnit)) }
    val parsed = circleRadiusMetres(text, metresPerUnit)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Set radius") },
        text = {
            OutlinedTextField(value = text, onValueChange = { text = it }, label = { Text("Radius ($unit)") }, singleLine = true)
        },
        confirmButton = { TextButton(enabled = parsed != null, onClick = { parsed?.let(onSet) }) { Text("Set") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun PositionDialog(at: TrackPoint, onDismiss: () -> Unit, onMove: (TrackPoint) -> Unit) {
    var latitude by remember(at) { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.latitude)) }
    var longitude by remember(at) { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.longitude)) }
    val parsed = parsedCoordinate(latitude, longitude)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Edit position") },
        text = {
            Column {
                OutlinedTextField(value = latitude, onValueChange = { latitude = it }, label = { Text("Latitude") }, singleLine = true)
                OutlinedTextField(value = longitude, onValueChange = { longitude = it }, label = { Text("Longitude") }, singleLine = true)
            }
        },
        confirmButton = { TextButton(enabled = parsed != null, onClick = { parsed?.let(onMove) }) { Text("Move") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
fun CenterMenu(
    launch: TrackPoint?,
    myLocation: TrackPoint?,
    onCentre: (TrackPoint) -> Unit,
    launchLabel: String = "Launch",
    missionPoints: List<TrackPoint>? = null,
    onFit: (List<TrackPoint>?) -> Unit = {},
) {
    var open by remember { mutableStateOf(false) }
    var asking by remember { mutableStateOf(false) }
    val fleetJson by mapPath(VEHICLES_VIEW)
    val vehicle = remember(fleetJson) { vehicleChoices(fleetJson).choices.firstOrNull { it.active } }
        ?.takeIf { isPlottable(it.latitude, it.longitude) }
        ?.let { TrackPoint(it.latitude, it.longitude) }
    Box {
        FilledTonalButton(onClick = { open = true }) { Text("Center") }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            missionPoints?.let { points ->
                DropdownMenuItem(text = { Text("Mission") }, onClick = { open = false; onFit(points) })
                DropdownMenuItem(text = { Text("All items") }, onClick = { open = false; onFit(null) })
            }
            DropdownMenuItem(text = { Text(launchLabel) }, enabled = launch != null, onClick = { open = false; launch?.let(onCentre) })
            DropdownMenuItem(text = { Text("Vehicle") }, enabled = vehicle != null, onClick = { open = false; vehicle?.let(onCentre) })
            DropdownMenuItem(text = { Text("My location") }, enabled = myLocation != null, onClick = { open = false; myLocation?.let(onCentre) })
            DropdownMenuItem(text = { Text("Coordinates…") }, onClick = { open = false; asking = true })
        }
    }
    if (asking) {
        var latitude by remember { mutableStateOf("") }
        var longitude by remember { mutableStateOf("") }
        val parsed = parsedCoordinate(latitude, longitude)
        AlertDialog(
            onDismissRequest = { asking = false },
            title = { Text("Center map on coordinate") },
            text = {
                Column {
                    OutlinedTextField(value = latitude, onValueChange = { latitude = it }, label = { Text("Latitude") }, singleLine = true)
                    OutlinedTextField(value = longitude, onValueChange = { longitude = it }, label = { Text("Longitude") }, singleLine = true)
                }
            },
            confirmButton = {
                TextButton(enabled = parsed != null, onClick = {
                    asking = false
                    parsed?.let(onCentre)
                }) { Text("Center") }
            },
            dismissButton = { TextButton(onClick = { asking = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun StatTile(label: String, value: String) {
    val unit = value.substringAfterLast(' ', "").takeIf { value.contains(' ') }.orEmpty()
    val number = value.removeSuffix(unit).trim()
    Column(Modifier.padding(end = 20.dp, top = 4.dp, bottom = 4.dp)) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(number, style = MaterialTheme.typography.titleLarge)
            if (unit.isNotBlank()) Text(unit, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

internal enum class PlanLayer(val label: String) { Mission("Mission"), Fence("Fence"), Rally("Rally") }

internal fun layerSubtitle(layer: PlanLayer, missionItems: Int, rallyPoints: Int): String = when (layer) {
    PlanLayer.Mission -> "$missionItems items"
    PlanLayer.Rally -> "$rallyPoints points"
    PlanLayer.Fence -> ""
}

internal fun ownerOf(hit: MapHit?): String? = when (hit) {
    is MapHit.Waypoint -> "m${hit.index}"
    is MapHit.SurveyVertex -> "m${hit.item}"
    is MapHit.LandingPlace -> "m${hit.index}"
    is MapHit.LoiterRadius -> "m${hit.index}"
    is MapHit.LoiterRotation -> "m${hit.index}"
    is MapHit.ShapeCentre -> if (hit.fence) "p${hit.owner}" else "m${hit.owner}"
    is MapHit.ShapeRadius -> if (hit.fence) "p${hit.owner}" else "m${hit.owner}"
    is MapHit.FenceVertex -> "p${hit.polygon}"
    is MapHit.Circle -> "c${hit.index}"
    is MapHit.CircleCentre -> "c${hit.index}"
    is MapHit.CircleRadius -> "c${hit.index}"
    is MapHit.Rally -> "r${hit.index}"
    MapHit.BreachReturn -> "breach"
    else -> null
}

internal fun dragAllowed(hit: MapHit, selected: MapHit?, layer: PlanLayer): Boolean = when (hit) {
    is MapHit.Midpoint -> midpointOwner(hit.path)?.let { it == ownerOf(selected) } ?: (selected is MapHit.Waypoint)
    MapHit.BreachReturn -> layer == PlanLayer.Fence
    else -> layerOf(hit) == layer && ownerOf(hit) != null && ownerOf(hit) == ownerOf(selected)
}

internal fun profileShown(layer: PlanLayer, profile: TerrainProfile): Boolean = layer == PlanLayer.Mission && profile.points.isNotEmpty()

internal fun actsOnTap(hit: MapHit): Boolean = hit is MapHit.Midpoint || hit is MapHit.LoiterRotation || hit == MapHit.BreachReturn

internal fun midpointOwner(path: String): String? = when {
    path.startsWith("$FENCE_POLYGONS.") -> "p${path.removePrefix("$FENCE_POLYGONS.").substringBefore('.')}"
    path.startsWith("$PLAN_ITEMS.") -> "m${path.removePrefix("$PLAN_ITEMS.").substringBefore('.')}"
    else -> null
}

internal fun layerOf(hit: MapHit?): PlanLayer? = when (hit) {
    null -> null
    is MapHit.Rally -> PlanLayer.Rally
    is MapHit.FenceVertex, is MapHit.Circle, is MapHit.CircleCentre, is MapHit.CircleRadius, MapHit.BreachReturn -> PlanLayer.Fence
    is MapHit.ShapeCentre -> if (hit.fence) PlanLayer.Fence else PlanLayer.Mission
    is MapHit.ShapeRadius -> if (hit.fence) PlanLayer.Fence else PlanLayer.Mission
    is MapHit.Midpoint -> null
    else -> PlanLayer.Mission
}

@Composable
private fun FenceListRow(
    row: FenceRow,
    chosen: Boolean,
    onSelect: () -> Unit,
    onInclusion: ((Boolean) -> Unit)? = null,
    onRemove: () -> Unit,
) {
    Row(
        Modifier.fillMaxWidth()
            .background(if (chosen) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent, MaterialTheme.shapes.small)
            .clickable(onClick = onSelect)
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(row.title, style = MaterialTheme.typography.bodyLarge)
            if (row.detail.isNotBlank()) Text(row.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (row.radius.isNotBlank()) Text(row.radius, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(horizontal = 8.dp))
        if (onInclusion != null && row.inclusion != null) {
            Switch(checked = row.inclusion, onCheckedChange = onInclusion, modifier = Modifier.padding(horizontal = 8.dp))
        }
        IconButton(onClick = onRemove) { Icon(Icons.Filled.Close, contentDescription = "Remove ${row.title}") }
    }
}

private val AT_THIS_POINT_WIDTH = 242.dp
