package one.aircast.map

import androidx.annotation.DrawableRes
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.foundation.clickable
import androidx.compose.material3.ButtonDefaults


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
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import org.json.JSONObject
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
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
private const val PANEL_MAX_FRACTION = 0.5f
private val SIDE_PANEL_WIDTH = 380.dp
private const val SIDE_PANEL_MIN_WIDTH_DP = 840
internal const val SUMMARY_MAX_FRACTION = 0.74f

private const val WAITING_FOR_QGC = "Waiting for QGroundControl"

private fun sendPlan(
    scope: CoroutineScope,
    say: (String?) -> Unit,
    done: () -> Unit,
    pauseFirst: Boolean = false,
) {
    say("Uploading to vehicle")
    scope.launch {
        val outcome = withContext(Dispatchers.Default) {
            if (pauseFirst) PlanBridge.pauseVehicle()
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

class PlanBar(val upload: PlanUpload, val stats: List<PlanStat>, val warning: String?)

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
internal fun PlanMapContent(
    mapStyle: String,
    onCentre: ((Double, Double) -> Unit)? = null,
    itemPanel: (@Composable (Int, TrackPoint?, String?) -> Unit)? = null,
    header: (@Composable (PlanBar) -> Unit)? = null,
    routeSettings: (@Composable () -> Unit)? = null,
    fitKey: Int = 0,
    onTemplates: (() -> Unit)? = null,
) {
    var follow by remember { mutableStateOf(false) }
    var shownStyle by remember(mapStyle) { mutableStateOf(mapStyle) }
    var fitRequest by remember { mutableIntStateOf(0) }
    var edits by remember { mutableIntStateOf(0) }
    var fitOnly by remember { mutableStateOf<List<TrackPoint>?>(null) }
    var positioning by remember { mutableStateOf<Pair<MapHit, TrackPoint>?>(null) }
    var loadArmed by remember { mutableStateOf(false) }
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
    var panelOpen by remember { mutableStateOf(true) }
    var layer by remember { mutableStateOf(PlanLayer.Mission) }
    LaunchedEffect(selected) { layerOf(selected)?.let { layer = it } }

    BackHandler(enabled = selected != null) { selected = null }
    var busy by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val removeItem: (MissionItem) -> Unit = { item ->
        scope.launch {
            busy = "Removing #${item.sequence}"
            val outcome = withContext(Dispatchers.Default) { removeMissionItem(item.index) }
            selected = if (outcome.ok) selectionAfterRemove(item.index, allItems.size) else null
            busy = outcome.reason.takeIf { !outcome.ok }
            if (!outcome.ok) {
                delay(FAILURE_MESSAGE_MS)
                busy = null
            }
        }
    }
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var zoom by remember { mutableDoubleStateOf(0.0) }
    var controlsHeightPx by remember { mutableIntStateOf(0) }
    var headerPx by remember { mutableIntStateOf(0) }
    val sidePanel = LocalConfiguration.current.screenWidthDp >= SIDE_PANEL_MIN_WIDTH_DP
    val sidePanelPx = with(LocalDensity.current) { SIDE_PANEL_WIDTH.roundToPx() }.takeIf { sidePanel } ?: 0
    val railPx = with(LocalDensity.current) { (RAIL_WIDTH + 8.dp).roundToPx() }

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
                busy = outcome.reason
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
    val canUndo = remember(planStatus) { planStatus?.optBoolean("canUndo") == true }
    val homeSet = remember(planStatus) { planStatus?.optJSONObject("templates")?.optBoolean("homeSet") != false }
    fun addable(kind: String): Boolean = kindAllows(insertable, kind) || !homeSet
    fun addablePattern(kind: MissionKind): Boolean = kind.enabled || !homeSet
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
    fun focusItem(index: Int) {
        panelOpen = true
        selected = MapHit.Waypoint(index)
        items.firstOrNull { it.index == index }?.let { placed ->
            centreOn = TrackPoint(placed.latitude, placed.longitude)
            centreRequest += 1
            follow = false
        }
    }
    fun pickRow(row: ItemRow) = focusItem(row.index)
    fun addRallyAt(latitude: Double, longitude: Double) {
        val next = rally.size
        onBridge("Adding rally", then = { selected = MapHit.Rally(next) }) { FenceBridge.addRallyPoint(latitude, longitude) }
    }
    var centredOnEntry by remember { mutableStateOf(false) }
    val entryPoint = TrackPoint(latitude, longitude).takeIf { isPlottable(latitude, longitude) } ?: operator
    LaunchedEffect(entryPoint != null) {
        if (centersOnVehicleAtEntry(centredOnEntry, entryPoint != null, fitRequest)) {
            centredOnEntry = true
            centreOn = entryPoint
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
                if (!firstRead && drawn) centredOnEntry = true
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

    fun requestDownload() {
        val refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), "download from")
        when {
            refusal != null -> say(refusal)
            loadStep(planDirty, loadArmed) == LoadStep.Confirm -> loadArmed = true
            else -> download()
        }
    }

    val uploadText = uploadLabel(planOffline, planSyncing, planDirty, planHasItems)
    val uploadEnabled = planHasItems && !planOffline && !planSyncing
    val uploadEmphasised = uploadEnabled && !uploadBlocked
    val planUpload = PlanUpload(enabled = uploadEnabled, emphasised = uploadEmphasised, label = uploadText, shown = !planOffline, onClick = upload)
    Box(Modifier.fillMaxSize()) {
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
                    PlanLayer.Mission -> {
                        panelOpen = false
                        addMissionItem(KIND_WAYPOINT, "Adding a waypoint", TrackPoint(lat, lon), insertAfter(selected, allItems))
                    }
                    PlanLayer.Rally -> if (support.rally) addRallyAt(lat, lon)
                    PlanLayer.Fence -> Unit
                }
            },
            onBlankTap = { lat, lon ->
                val traced = tracing
                when {
                    traced != null -> tracing = traced.first to traced.second + TrackPoint(lat, lon)
                    tapCloses(selected, panelOpen) -> {
                        selected = null
                        panelOpen = false
                    }
                    layer == PlanLayer.Mission -> {
                        panelOpen = false
                        addMissionItem(KIND_WAYPOINT, "Adding a waypoint", TrackPoint(lat, lon), insertAfter(selected, allItems))
                    }
                    layer == PlanLayer.Rally && support.rally -> addRallyAt(lat, lon)
                    else -> selected = null
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
                    else -> {
                        panelOpen = true
                        selected = hit
                    }
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
            topInsetPx = headerPx,
            leftInsetPx = sidePanelPx + railPx,
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
            PositionDialog(at, positionTitle(hit), onDismiss = { positioning = null }) { moved ->
                positioning = null
                onBridge(done = movedText(hit, allItems)) { writeMove(hit, moved.latitude, moved.longitude, surveyList, rally, fences, allItems, circles) }
            }
        }

        val ready by MapBridge.bridgeReady.collectAsState()
        val mapStart = if (sidePanel) SIDE_PANEL_WIDTH else 0.dp
        val railStart = mapStart + 8.dp
        val headerHeight = with(LocalDensity.current) { headerPx.toDp() }
        header?.let { bar ->
            Box(Modifier.align(Alignment.TopStart).padding(start = mapStart).fillMaxWidth().onGloballyPositioned { headerPx = it.size.height }) {
                bar(PlanBar(planUpload, planStats(itemCount, allItems, missionSummaryView), terrainWarning(terrainHits.size, (collidingSimple + collidingPatterns).size)))
            }
        }
        val panelHeight = with(LocalDensity.current) { controlsHeightPx.toDp() }
        val listedInPanel = sidePanel && layer == PlanLayer.Mission && worthListing(allItems)
        Column(
            Modifier.align(Alignment.TopStart).padding(start = railStart + RAIL_WIDTH + 8.dp, top = headerHeight + 8.dp, end = 8.dp).fillMaxWidth(SUMMARY_MAX_FRACTION),
        ) {
            (busy ?: WAITING_FOR_QGC.takeIf { !ready })?.let { message ->
                Surface(
                    color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.94f),
                    shape = MaterialTheme.shapes.extraLarge,
                ) {
                    Text(message, Modifier.padding(horizontal = 16.dp, vertical = 10.dp), style = MaterialTheme.typography.bodyMedium)
                }
            }
            centre?.let { at ->
                ScaleBarView(at.latitude, zoom, Modifier.padding(start = 4.dp, top = 8.dp))
            }
        }

        PlanRail(Modifier.align(Alignment.TopStart).padding(start = railStart, top = headerHeight + 8.dp, bottom = if (sidePanel) 8.dp else panelHeight + 8.dp)) {
            PlanLayer.entries.forEach { option ->
                RailButton(option.icon, option.label, chosen = layer == option, labelled = true, onClick = {
                    panelOpen = true
                    layer = option
                    if (layerOf(selected) != option) selected = null
                })
            }
            RailDivider()
            when (layer) {
                PlanLayer.Mission -> {
                    RailButton(R.drawable.plan_add, "Waypoint", enabled = placeAt() != null, onClick = {
                        addMissionItem(KIND_WAYPOINT, "Adding a waypoint", placeAt(), insertAfter(selected, allItems))
                    })
                    RailButton(R.drawable.plan_grid, "Pattern", enabled = scanPatterns(insertable).isNotEmpty(), onClick = { patternWanted = scanPatterns(insertable) })
                    if (kindOffered(insertable, KIND_TAKEOFF) && takeoffMissing(items)) {
                        RailButton(R.drawable.plan_flight_takeoff, "Takeoff", enabled = addable(KIND_TAKEOFF), onClick = {
                            addMissionItem(KIND_TAKEOFF, "Adding a takeoff", placeAt(), BEFORE_THE_REST)
                        })
                    }
                    RailButton(R.drawable.plan_flight_land, kindLabel(insertable, KIND_LAND), enabled = itemCount > 0, onClick = {
                        addMissionItem(KIND_LAND, "Adding a landing", placeAt(), insertAfter(selected, allItems))
                    })
                }
                PlanLayer.Fence -> if (!support.fenceRefused) {
                    RailButton(R.drawable.plan_polygon, "Polygon", enabled = support.fence, onClick = {
                        val at = placeAt()
                        val next = fences.size
                        onBridge("Adding fence", then = { selected = MapHit.FenceVertex(next, 0) }) {
                            at?.let { fenceWindow(visible, it) }?.let { (topLeft, bottomRight) -> FenceBridge.addInclusionPolygon(topLeft, bottomRight) } ?: false
                        }
                    })
                    RailButton(R.drawable.plan_circle, "Circle", enabled = support.fence, onClick = {
                        val at = placeAt()
                        val next = circles.size
                        onBridge("Adding circle", then = { selected = MapHit.Circle(next) }) {
                            at?.let { fenceWindow(visible, it) }?.let { (topLeft, bottomRight) -> FenceBridge.addInclusionCircle(topLeft, bottomRight) } ?: false
                        }
                    })
                    RailButton(R.drawable.plan_home, "Breach", enabled = support.fence, chosen = breach != null, onClick = {
                        if (breach != null) {
                            editingBreach = true
                        } else {
                            val at = placeAt()
                            onBridge("Adding breach return point") { at != null && FenceBridge.setBreachReturn(at) }
                        }
                    })
                }
                PlanLayer.Rally -> RailButton(R.drawable.plan_add, "Add point", enabled = support.rally && placeAt() != null, onClick = {
                    placeAt()?.let { addRallyAt(it.latitude, it.longitude) }
                })
            }
        }

        PlanRail(Modifier.align(Alignment.TopEnd).padding(end = 8.dp, top = headerHeight + 8.dp, bottom = if (sidePanel) 8.dp else panelHeight + 8.dp)) {
            RailButton(R.drawable.plan_undo, "Undo", enabled = canUndo, onClick = { onBridge { invokeOk(PLAN_UNDO) } })
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
                following = follow,
                onFollow = { follow = !follow },
                anchor = { open -> RailButton(R.drawable.plan_my_location, "Centre", chosen = follow, onClick = open) },
            )
            MapTypeMenu { shownStyle = it }
        }

        if (loadArmed) {
            AlertDialog(
                onDismissRequest = { loadArmed = false },
                title = { Text("Load plan from vehicle?") },
                text = { Text(replaceWarning(allItems.count { it.index != HOME_ITEM })) },
                confirmButton = { TextButton(onClick = ::download) { Text("Replace") } },
                dismissButton = { TextButton(onClick = { loadArmed = false }) { Text("Keep mine") } },
            )
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

        if (patternWanted.isNotEmpty()) {
            AlertDialog(
                onDismissRequest = { patternWanted = emptyList() },
                title = { Text("Which pattern?") },
                text = {
                    Text(
                        patternWanted.firstOrNull { !addablePattern(it) && it.disabledReason.isNotBlank() }
                            ?.disabledReason
                            ?: "A pattern covers an area or a line with a camera run.",
                    )
                },
                confirmButton = {
                    Column {
                        patternWanted.forEach { kind ->
                            TextButton(
                                enabled = addablePattern(kind),
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
                    .then(if (sidePanel) Modifier.fillMaxHeight().padding(top = 12.dp) else Modifier.heightIn(max = (LocalConfiguration.current.screenHeightDp * PANEL_MAX_FRACTION).dp)),
            ) {
                val chosen = selected
                val chosenItem = (chosen as? MapHit.Waypoint)?.let { hit -> allItems.firstOrNull { it.index == hit.index } }
                Column(Modifier.fillMaxWidth().panelDrag { open -> panelOpen = open }) {
                if (!sidePanel) Box(
                    Modifier
                        .fillMaxWidth()
                        .height(24.dp)
                        .clickable(onClickLabel = if (panelOpen) "Fold the panel" else "Open the panel") { panelOpen = !panelOpen },
                    contentAlignment = Alignment.Center,
                ) {
                    Box(
                        Modifier
                            .size(width = 32.dp, height = 4.dp)
                            .background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f), CircleShape),
                    )
                }
                if (layer == PlanLayer.Mission && itemCount > 0 && layerOf(chosen) in setOf(null, PlanLayer.Mission)) {
                    WaypointStripBar(
                        rows = itemRows(allItems, surveyStatsMap),
                        altitudes = allItems.associate { it.index to it.altitudeText },
                        conflicts = collidingSimple + collidingPatterns,
                        selected = chosenItem?.index,
                        onPick = ::focusItem,
                        onList = { listOpen = true }.takeIf { !sidePanel },
                        profileShown = missionStatusShown.takeIf { profileShown(layer, profile) },
                        onProfile = { onBridge { setOk(SHOW_MISSION_ITEM_STATUS, settingJson((!missionStatusShown).toString())) } },
                    )
                }
                when {
                    chosenItem != null -> SelectionHeader(
                        title = "${sentenceCase(chosenItem.command.ifBlank { "Item" })} ${chosenItem.sequence}",
                        detail = (if (chosenItem.index in collidingSimple + collidingPatterns) TERRAIN_CONFLICT_HERE else null)
                            ?: sheetDetail(chosenItem, surveyStatsMap[chosenItem.index]).takeIf { chosenItem.complexPattern }?.ifBlank { null }
                            ?: addingAfterText(chosen, allItems).takeIf { !panelOpen },
                        warning = chosenItem.index in collidingSimple + collidingPatterns,
                        open = panelOpen,
                        onTitle = { panelOpen = !panelOpen },
                        onDelete = { removeItem(chosenItem) }.takeIf { chosenItem.index > HOME_ITEM },
                        onDone = { selected = null },
                    )
                    chosen != null -> SelectionHeader(
                        title = selectionTitle(chosen, allItems),
                        detail = selectionText(chosen, allItems, circles, fences),
                        warning = false,
                        open = panelOpen,
                        onTitle = { panelOpen = !panelOpen },
                        onDelete = null,
                        onDone = { selected = null },
                    )
                    layer == PlanLayer.Mission && itemCount == 0 -> EmptyMissionStrip(
                        homeSet = homeSet,
                        onTemplates = onTemplates,
                        onDownload = { requestDownload() }.takeIf { !planOffline },
                    )
                    layer != PlanLayer.Mission -> Text(layer.label, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(start = 12.dp, bottom = 4.dp))
                }
                }
                val panelScroll = remember(chosen) { ScrollState(0) }
                if (panelOpen || sidePanel) Column(Modifier.weight(1f, fill = false).verticalScroll(panelScroll)) {
                    if (chosen == null && layer == PlanLayer.Mission && missionStatusShown && profileShown(layer, profile) && itemCount > 0) {
                        TerrainProfileView(profile, elevationNotice, selectedSequence = selectedSequence) { sequence ->
                            allItems.firstOrNull { it.sequence == sequence }?.let { selected = MapHit.Waypoint(it.index) }
                        }
                    }
                    when {
                        chosenItem != null -> {
                            surveyTiles(chosenItem, surveyStatsMap[chosenItem.index]).takeIf { it.isNotEmpty() }?.let { tiles ->
                                Row(Modifier.padding(start = 12.dp, end = 12.dp, bottom = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    tiles.forEach { (label, value) -> StatTile(label, value) }
                                }
                            }
                            WaypointSettings(chosenItem, globalFrame) { label, work -> onBridge(label) { work() } }
                        }
                        chosen != null -> Unit
                        layer == PlanLayer.Mission -> routeSettings?.invoke()
                        layer == PlanLayer.Rally -> {
                            if (support.rallyRefused) PaletteNote(RALLY_NOT_SUPPORTED)
                            else if (rally.isEmpty()) PaletteNote(NO_RALLY_POINTS)
                            if (rally.isNotEmpty()) FenceHeading("Rally points")
                            rallyRows(rally).forEach { row ->
                                FenceListRow(
                                    row,
                                    chosen = false,
                                    onSelect = { selected = MapHit.Rally(row.index) },
                                ) {
                                    val count = rally.size
                                    onBridge("Removing ${row.title.lowercase()}", then = { selected = rallyAfterRemove(row.index, count) }) { FenceBridge.removeRallyPoint(row.index) }
                                }
                            }
                        }
                        else -> {
                            if (support.fenceRefused) PaletteNote(GEOFENCE_NOT_SUPPORTED)
                            else if (fences.isEmpty() && circles.isEmpty()) PaletteNote(NO_GEOFENCE)
                            val listed = if (support.fenceRefused) emptyList() else fenceRows(fences, circles)
                            listed.forEachIndexed { at, row ->
                                fenceHeading(row, listed.getOrNull(at - 1))?.let { FenceHeading(it) }
                                FenceListRow(
                                    row,
                                    chosen = false,
                                    onSelect = { selected = fenceRowHit(row) },
                                    onInclusion = row.inclusion?.let { { keep: Boolean -> onBridge("Changing ${row.title.lowercase()}") { FenceBridge.setPolygonInclusion(row.index, keep) } } },
                                ) {
                                    onBridge("Removing ${row.title.lowercase()}", then = { selected = fenceSelectionAfterRemove(row, selected) }) {
                                        if (row.circle) FenceBridge.deleteCircle(row.index) else FenceBridge.deletePolygon(row.index)
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

                    val survey = selectedSurvey(selected, surveyList)
                    val shapeFence = (selected as? MapHit.ShapeCentre)?.takeIf { it.fence }?.owner ?: (selected as? MapHit.ShapeRadius)?.takeIf { it.fence }?.owner
                    val fenceHit = selected as? MapHit.FenceVertex
                    val surveyHit = selected as? MapHit.SurveyVertex
                    val rallyHit = selected as? MapHit.Rally
                    val circleIndex = (selected as? MapHit.Circle)?.index
                        ?: (selected as? MapHit.CircleCentre)?.index
                    val circle = circleIndex?.let { index -> circles.firstOrNull { it.index == index } }

                    if (survey != null || fenceHit != null || shapeFence != null ||
                        rallyHit != null || circle != null || selectedLanding(selected, landingList) != null
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
                                        shapeCentreHit(target, fences, surveyList)?.let { centre ->
                                            TextButton(onClick = { positioning = centre }) { Text("Edit position\u2026") }
                                        }
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
                                }) { Text("Delete rally point") }
                            }

                        }
                    }

                    chosenItem?.let { item ->
                        itemPanel?.invoke(item.index, TrackPoint(item.latitude, item.longitude).takeIf { item.placed }, advancedDetail(item))
                    }
                }
            }
        }

        if (listOpen) {
            AircastSheet(onDismissRequest = { listOpen = false }) {
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

private const val SURVEY_FIT_INSET = 0.8

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
        RailButton(R.drawable.plan_layers, "Map", onClick = {
            scope.launch {
                listed = withContext(Dispatchers.Default) { mapTypes(runCatching { JSONObject(QGCBridge.get(MAP_TYPES_VIEW)) }.getOrNull()) }
                open = true
            }
        })
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
private fun PositionDialog(at: TrackPoint, title: String, onDismiss: () -> Unit, onMove: (TrackPoint) -> Unit) {
    var latitude by remember(at) { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.latitude)) }
    var longitude by remember(at) { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.longitude)) }
    val parsed = parsedCoordinate(latitude, longitude)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
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
    following: Boolean? = null,
    onFollow: () -> Unit = {},
    anchor: @Composable (open: () -> Unit) -> Unit = { open -> FilledTonalButton(onClick = open) { Text("Center map") } },
) {
    var open by remember { mutableStateOf(false) }
    var asking by remember { mutableStateOf(false) }
    val fleetJson by mapPath(VEHICLES_VIEW)
    val vehicle = remember(fleetJson) { vehicleChoices(fleetJson).choices.firstOrNull { it.active } }
        ?.takeIf { isPlottable(it.latitude, it.longitude) }
        ?.let { TrackPoint(it.latitude, it.longitude) }
    Box {
        anchor { open = true }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            following?.let { on ->
                DropdownMenuItem(
                    text = { Text("Follow vehicle") },
                    enabled = vehicle != null,
                    trailingIcon = { if (on) Icon(Icons.Filled.Check, contentDescription = null) },
                    onClick = { open = false; onFollow() },
                )
            }
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

internal enum class PlanLayer(val label: String, @DrawableRes val icon: Int) {
    Mission("Mission", R.drawable.plan_route),
    Fence("Fence", R.drawable.plan_shield),
    Rally("Rally", R.drawable.plan_flag),
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
