package one.aircast.android.ui

import androidx.compose.foundation.rememberScrollState

import androidx.compose.foundation.verticalScroll

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.width
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.unit.isSpecified
import androidx.compose.ui.text.TextStyle
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Fact
import kotlin.math.roundToInt
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.settingControl
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import one.aircast.android.bridge.qgcStrings
import one.aircast.mapspike.TelemetryNumber
import one.aircast.mapspike.optText

private const val GCS_POSITION = "view.gcsPosition"


internal data class ConfirmOption(val label: String, val run: (Boolean) -> Unit)

internal data class GuidedAction(
    val name: String,
    val confirm: String,
    val destructive: Boolean,
    val option: ConfirmOption? = null,
    val offerId: String? = null,
    val run: () -> Unit,
)

internal const val CHECKLIST_PASSED = 1
internal const val CHECKLIST_FAILED = 2

internal fun checklistStateValue(passed: Boolean): Int = if (passed) CHECKLIST_PASSED else CHECKLIST_FAILED

internal fun offerWithdrawn(offerId: String?, offers: Map<String, GuidedOffer>): Boolean =
    offerId != null && offers[offerId]?.shown != true


internal data class Instrument(val label: String, val reading: String, val id: String = "", val value: String = reading, val units: String = "", val raw: Double? = null)

internal fun rowWidth(count: Int): Int = when {
    count <= 4 -> count
    else -> (count + 1) / 2
}

internal fun operatorDistance(view: JSONObject?): List<Instrument> =
    view?.optText("distanceToVehicleText")
        ?.takeIf { it.isNotBlank() }
        ?.let { listOf(Instrument(label = "From you", reading = it)) }
        ?: emptyList()

internal const val AWAITING_READING = "\u2014"

internal fun instrumentReading(item: JSONObject): String? {
    if (!item.optBoolean("missing")) {
        val units = item.optText("units")
        val value = item.optText("value")
        return if (units.isBlank()) value else "$value $units"
    }
    return AWAITING_READING.takeIf { item.optText("missingReason") == "notReported" }
}

internal fun instruments(view: JSONObject?): List<Instrument> {
    val items = view?.optJSONArray("items") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { item ->
            instrumentReading(item)?.let {
                Instrument(
                    label = item.optText("label"),
                    reading = it,
                    id = item.optText("id"),
                    value = if (item.optBoolean("missing")) it else item.optText("value"),
                    units = if (item.optBoolean("missing")) "" else item.optText("units"),
                    raw = if (item.isNull("raw")) null else item.optDouble("raw").takeIf { r -> !r.isNaN() },
                )
            }
        }
    }
}

@Composable
fun VehicleTitle() {
    val json by qgcPath(FLY_STATE)
    val fly = remember(json) { flyState(json) }
    val communicationLost = fly?.contactLost == true

    Column {
        Text("Aircast", style = MaterialTheme.typography.titleMedium)
        Text(
            text = vehicleSubtitle(fly),
            style = MaterialTheme.typography.bodySmall,
            fontWeight = if (communicationLost) FontWeight.Bold else FontWeight.Normal,
            color = if (communicationLost) {
                MaterialTheme.colorScheme.error
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun TelemetryRow(modifier: Modifier = Modifier, columns: Int? = null) {
    val context = LocalContext.current
    val classView by qgcPath(INSTRUMENTS_VIEW)
    val vehicleClass = instrumentVehicleClass(classView)
    var chosen by remember(vehicleClass) { mutableStateOf(readChosen(context, vehicleClass)) }
    var choosing by remember { mutableStateOf(false) }
    var displays by remember { mutableStateOf(readDisplays(context)) }
    var styling by remember { mutableStateOf<Instrument?>(null) }
    LaunchedEffect(Unit) { OverlayLayout.valueSize = readValueSize(context) }
    val view by qgcPath(instrumentsPath(chosen))
    val gcsJson by qgcPath(GCS_POSITION)
    val shown = remember(view, gcsJson, chosen) {
        (if (showsInstruments(chosen)) instruments(view) else emptyList()) + operatorDistance(gcsJson)
    }
    val stateJson by qgcPath(FLY_STATE)
    val stale = remember(stateJson) { flyState(stateJson)?.staleNotice.orEmpty() }
    val silent = stale.isNotBlank()

    if (shown.isEmpty()) return

    if (silent) {
        Text(
            text = stale,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        )
    }

    styling?.let { instrument ->
        ValueDisplayDialog(
            label = instrument.label,
            initial = displays[instrument.id] ?: ValueDisplay(),
            onDismiss = { styling = null },
            onDone = { display ->
                writeDisplay(context, instrument.id, display)
                displays = displays + (instrument.id to display)
                styling = null
            },
        )
    }

    if (choosing) {
        InstrumentSheet(
            chosen = chosen,
            onToggle = { name ->
                chosen = withInstrument(chosen, name)
                writeChosen(context, vehicleClass, chosen)
            },
            onDismiss = { choosing = false },
        )
    }

    FlowRow(
        modifier
            .fillMaxWidth()
            .alpha(if (silent) 0.45f else 1f),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalArrangement = Arrangement.spacedBy(6.dp),
        maxItemsInEachRow = columns ?: rowWidth(shown.size + 1),
    ) {
        shown.forEach { instrument ->
            val display = displays[instrument.id] ?: ValueDisplay()
            Column(
                Modifier
                    .padding(horizontal = 12.dp, vertical = 6.dp)
                    .then(if (instrument.id.isBlank()) Modifier else Modifier.clickable { styling = instrument }),
            ) {
                ValueLabel(display, instrument.raw, instrument.label.uppercase(), MaterialTheme.colorScheme.onSurfaceVariant)
                Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(
                        instrument.value,
                        style = scaledNumber(OverlayLayout.valueSize.scale),
                        color = displayColour(display, instrument.raw)?.let { Color(it) } ?: MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.alignByBaseline(),
                    )
                    if (display.showUnits && instrument.units.isNotBlank()) Text(
                        instrument.units,
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.alignByBaseline(),
                    )
                }
            }
        }
        IconButton(onClick = { choosing = true }) {
            Icon(
                painter = painterResource(R.drawable.ic_tune),
                contentDescription = "Readings",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

internal fun scaledNumber(scale: Float): TextStyle = TelemetryNumber.copy(
    fontSize = TelemetryNumber.fontSize * scale,
    lineHeight = if (TelemetryNumber.lineHeight.isSpecified) TelemetryNumber.lineHeight * scale else TelemetryNumber.lineHeight,
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun FlightActions(modifier: Modifier = Modifier, layout: FlyDeckLayout = FlyDeckLayout.Bottom) {
    val simple = layout == FlyDeckLayout.Simple
    val side = layout == FlyDeckLayout.Side
    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val available = state?.connected == true
    val armed = state?.armed == true
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    var sentName by remember { mutableStateOf<String?>(null) }
    var sentSnapshot by remember { mutableStateOf<String?>(null) }
    var refusal by FlyRefusal::text
    val scope = rememberCoroutineScope()
    var takeoffTarget by remember { mutableStateOf<Double?>(null) }
    var takeoffSettled by remember { mutableStateOf<Double?>(null) }
    var takeoffRange by remember { mutableStateOf<GuidedTakeoff?>(null) }
    var altitudeTarget by remember { mutableStateOf<Double?>(null) }
    var altitudeSettled by remember { mutableStateOf<Double?>(null) }
    var altitudeRange by remember { mutableStateOf<GuidedAltitude?>(null) }
    var speedTarget by remember { mutableStateOf<Double?>(null) }
    var speedSettled by remember { mutableStateOf<Double?>(null) }
    var speedRange by remember { mutableStateOf<GuidedSpeed?>(null) }
    var altitudePauses by remember { mutableStateOf(false) }
    var showMore by remember { mutableStateOf(false) }
    var deckRest by remember { mutableStateOf<List<DeckEntry>>(emptyList()) }
    var deckShown by remember { mutableStateOf<Set<String>>(emptySet()) }
    var editingLoiter by remember { mutableStateOf<LoiterOffer?>(null) }
    val mapClickJson by qgcPath(MAP_CLICK_PATH)
    val loiter = remember(mapClickJson) { loiterOffer(mapClickJson) }
    var showChecklist by remember { mutableStateOf(false) }
    val enforceChecklist by qgcBool(settingControl("settings.appSettings.enforceChecklist"))
    var popupShownFor by remember { mutableStateOf<Int?>(null) }
    var checklistTicked by rememberSaveable { mutableStateOf(setOf<String>()) }
    val preflightJson by qgcPath(PREFLIGHT)
    val useChecklist = remember(preflightJson) { preflightOffered(preflightJson) }
    val checks = remember(preflightJson) { preflight(preflightJson) }
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val offers = remember(actionsJson) { guidedOffers(actionsJson) }
    val extras = remember(offers) { moreActions(offers) }
    val resumeFrom = remember(actionsJson) { resumeFromSequence(actionsJson) }
    val automaticMissionPopups by qgcBool(settingControl("settings.flyViewSettings.enableAutomaticMissionPopups"))
    var missionReady by remember { mutableStateOf<Set<String>?>(null) }

    LaunchedEffect(offers) {
        if (offerWithdrawn(pending?.offerId, offers)) pending = null
        if (speedTarget != null && offerWithdrawn("changeSpeed", offers)) speedTarget = null
        if (takeoffTarget != null && offerWithdrawn("takeoff", offers)) takeoffTarget = null
        if (altitudeTarget != null && offerWithdrawn(if (altitudePauses) PAUSE else "changeAltitude", offers)) altitudeTarget = null
        val popup = missionReady?.let { autoMissionPopup(it, offers, automaticMissionPopups) }
        missionReady = AUTO_POPUP_ACTIONS.filter { offers[it]?.ready == true }.toSet()
        if (popup != null && pending == null) {
            guidedCommand(popup.id, resumeFrom)?.let { command ->
                pending = GuidedAction(name = popup.title, confirm = popup.prompt, destructive = popup.destructive, offerId = popup.id, run = command)
            }
        }
    }

    if (!available) {
        Text("Connect a vehicle to enable flight controls.", modifier.padding(16.dp))
        return
    }

    val vehiclesJson by qgcPath(one.aircast.mapspike.VEHICLES_VIEW)
    val vehicleId = remember(vehiclesJson) { activeVehicleId(vehiclesJson) }
    var checklistStateSent by remember { mutableStateOf<Boolean?>(null) }
    LaunchedEffect(vehicleId) {
        checklistTicked = emptySet()
        checklistStateSent = null
    }
    val checklistPassed = checklistIsComplete(checks, checklistTicked)
    LaunchedEffect(checklistPassed, vehicleId) {
        if (vehicleId == null || checklistStateSent == checklistPassed) return@LaunchedEffect
        if (checklistStateSent == null && !checklistPassed) {
            checklistStateSent = false
            return@LaunchedEffect
        }
        checklistStateSent = checklistPassed
        withContext(Dispatchers.Default) { Qgc.set("vehicle.checkListState", checklistStateValue(checklistPassed)) }
    }

    val deciding = pending != null || speedTarget != null ||
        takeoffTarget != null || altitudeTarget != null

    LaunchedEffect(vehicleId, deciding) {
        val id = vehicleId ?: return@LaunchedEffect
        if (popupShownFor == id || deciding) return@LaunchedEffect
        delay(CHECKLIST_POPUP_DELAY_MS)
        val complete = withContext(Dispatchers.Default) {
            checklistIsComplete(preflight(Qgc.get(PREFLIGHT)), checklistTicked)
        }
        if (checklistPopupIsDue(true, useChecklist, enforceChecklist, complete, deciding)) {
            popupShownFor = id
            showChecklist = true
        }
    }

    val openAltitude: (Boolean) -> Unit = { pauses ->
        altitudePauses = pauses
        scope.launch {
            val fresh = withContext(Dispatchers.Default) { guidedAltitude(Qgc.get(GUIDED_ALTITUDE)) }
            if (altitudeRangeUsable(fresh)) {
                altitudeRange = fresh
                altitudeTarget = fresh?.current
                altitudeSettled = fresh?.current
            } else {
                refusal = "This vehicle did not report an altitude range."
            }
        }
    }

    Column(modifier.then(if (side) Modifier.verticalScroll(rememberScrollState()) else Modifier).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {

        refusal?.let { message ->
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        val liveActions = actionsJson?.toString()
        val confirming = pending
        val showingSent = sentIsStillShowing(sentName, sentSnapshot, liveActions)

        if (confirming != null) {
            ConfirmTrack(
                action = confirming,
                onSent = {
                    sentName = confirming.name
                    sentSnapshot = liveActions
                    pending = null
                },
                onCancel = { pending = null },
            )
        } else if (showingSent) {
            SentNotice(sentName.orEmpty(), onDismiss = { sentName = null })
        }

        val armAction = offers[if (armed) "disarm" else "arm"]
        val entries = listOfNotNull(
            DeckEntry(
                id = "arm",
                label = armAction?.title ?: if (armed) "Disarm" else "Arm",
                icon = R.drawable.ic_bolt,
                enabled = armAction?.ready == true,
                warning = armed,
            ) {
                pending = GuidedAction(
                    offerId = if (armed) "disarm" else "arm",
                    name = armAction?.title ?: if (armed) "Disarm" else "Arm",
                    confirm = armAction?.prompt?.ifBlank { null } ?: if (armed) {
                        "Disarm the vehicle"
                    } else {
                        "Arm the vehicle."
                    },
                    destructive = armAction?.destructive ?: true,
                ) {
                    val target = !armed
                    scope.attemptCommand(
                        action = if (target) "Arm" else "Disarm",
                        report = { refusal = it },
                        withdraw = { refusal = withdrawn(refusal, it) },
                        reached = { armedNow() == target },
                    ) { Qgc.set("vehicle.armed", target) }
                }
            }.takeIf { armAction?.shown == true },
            DeckEntry("takeoff", offers["takeoff"]?.title ?: "Takeoff", R.drawable.ic_flight_takeoff, offers["takeoff"]?.ready == true) {
                scope.launch {
                    val fresh = withContext(Dispatchers.Default) {
                        guidedTakeoff(Qgc.get(GUIDED_TAKEOFF))
                    }
                    if (!takeoffRangeUsable(fresh)) {
                        refusal = "This vehicle did not report a takeoff height range."
                        return@launch
                    }
                    takeoffRange = fresh
                    takeoffTarget = fresh?.initial
                    takeoffSettled = fresh?.initial
                }
            }.takeIf { offers["takeoff"]?.shown == true },
            DeckEntry(PAUSE, offers[PAUSE]?.title ?: "Pause", R.drawable.ic_pause, offers[PAUSE]?.ready == true) {
                openAltitude(true)
            }.takeIf { offers[PAUSE]?.shown == true },
            DeckEntry("rtl", "Return", R.drawable.ic_home, offers["rtl"]?.ready == true) {
                pending = GuidedAction(
                    offerId = "rtl",
                    name = offers["rtl"]?.title ?: "Return",
                    confirm = offers["rtl"]?.prompt?.ifBlank { null } ?: "Return to the launch position of the vehicle",
                    destructive = false,
                    option = offers["rtl"]?.option?.ifBlank { null }?.let { label ->
                        ConfirmOption(label) { smart -> offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", smart) } }
                    },
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeRTL", false) }
                }
            }.takeIf { offers["rtl"]?.shown == true },
            DeckEntry("land", offers["land"]?.title ?: "Land", R.drawable.ic_flight_land, offers["land"]?.ready == true) {
                pending = GuidedAction(
                    offerId = "land",
                    name = offers["land"]?.title ?: "Land",
                    confirm = offers["land"]?.prompt?.ifBlank { null }
                        ?: "Land the vehicle at the current position",
                    destructive = false,
                ) {
                    offMainDetached { Qgc.invoke("vehicle.guidedModeLand") }
                }
            }.takeIf { offers["land"]?.shown == true },
            DeckEntry("changeSpeed", "Speed", R.drawable.ic_speed, offers["changeSpeed"]?.ready == true) {
                scope.launch {
                    val fresh = withContext(Dispatchers.Default) {
                        guidedSpeed(Qgc.get(GUIDED_SPEED))
                    }
                    if (!speedRangeUsable(fresh)) {
                        refusal = "This vehicle did not report a speed range."
                        return@launch
                    }
                    speedRange = fresh
                    speedTarget = fresh?.initial
                    speedSettled = fresh?.initial
                }
            }.takeIf { offers["changeSpeed"]?.shown == true },
            DeckEntry("changeAltitude", "Altitude", R.drawable.ic_height, offers["changeAltitude"]?.ready == true) {
                openAltitude(false)
            }.takeIf { offers["changeAltitude"]?.shown == true },
            DeckEntry(CHECKLIST, "Checklist", R.drawable.ic_check_circle, true) {
                showChecklist = true
            }.takeIf { useChecklist && checklistOffered(armed) == null },
        )
        val deck = deckIds(entries.map { it.id }.toSet(), armed)
        deckRest = entries.filter { entry -> deck.none { it.first == entry.id } }
        deckShown = deck.map { it.first }.toSet()
        LaunchedEffect(DeckRequest.action) {
            when (val asked = DeckRequest.action) {
                null -> Unit
                ARM_REQUEST -> entries.firstOrNull { it.id == ARM_REQUEST && it.enabled }?.onClick?.invoke()
                    ?: run { refusal = deckRequestRefusal(offers[if (armed) "disarm" else "arm"]) }
                else -> offers[asked]?.takeIf { it.ready }?.let { offer ->
                    guidedCommand(offer.id, resumeFrom)?.let { command ->
                        pending = GuidedAction(offerId = offer.id, name = offer.title, confirm = offer.prompt, destructive = offer.destructive, run = command)
                    }
                }
            }
            DeckRequest.action = null
        }

        if (!simple) TelemetryRow(columns = if (side) 1 else null)

        if (side) Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            deck.forEach { (id, primary) ->
                entries.firstOrNull { it.id == id }?.let { entry -> DeckButton(entry, primary, Modifier.fillMaxWidth()) }
            }
            DeckButton(DeckEntry("more", "More", R.drawable.ic_more_vert, true) { showMore = true }, primary = false, modifier = Modifier.fillMaxWidth())
        } else if (simple) SimpleDeck(deck, entries) { showMore = true } else Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            deck.forEach { (id, primary) ->
                entries.firstOrNull { it.id == id }?.let { entry ->
                    DeckButton(entry, primary, Modifier.weight(1f))
                }
            }
            DeckButton(
                DeckEntry("more", "More", R.drawable.ic_more_vert, true) { showMore = true },
                primary = false,
                modifier = if (deck.isEmpty()) Modifier.weight(1f) else Modifier.width(64.dp),
            )
        }
    }

    speedTarget?.let { target ->
        var probe by remember(speedTarget != null) { mutableStateOf<GuidedSpeed?>(null) }
        LaunchedEffect(speedSettled) {
            val at = speedSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) { guidedSpeed(Qgc.get(guidedSpeedPath(at))) }
        }
        GuidedValuePanel(
            title = speedRange?.label ?: "Speed",
            sentence = probe?.sentence ?: "",
            commitLabel = "Set",
            commitEnabled = probe != null,
            onCommit = {
                speedTarget = null
                offMainDetached {
                    val fresh = guidedSpeed(Qgc.get(guidedSpeedPath(target)))
                    val method = fresh?.command
                    if (method != null) {
                        // qtpaths: vehicle.guidedModeChangeGroundSpeedMetersSecond, vehicle.guidedModeChangeEquivalentAirspeedMetersSecond
                        Qgc.invoke("vehicle.$method", fresh.targetMetersSecond)
                    }
                }
            },
            onCancel = { speedTarget = null },
        ) {
            speedRange?.let { range -> guidedBounds(range.minimum, range.maximum)?.let { range to it } }?.let { (range, bounds) ->
                GuidedStepper(target, range.label, range.unit, bounds.first, bounds.second) { stepped ->
                    speedTarget = stepped
                    speedSettled = stepped
                }
            }
            Slider(
                value = target.toFloat(),
                onValueChange = { speedTarget = guidedRounded(it.toDouble(), speedRange?.unit.orEmpty()) },
                onValueChangeFinished = { speedSettled = speedTarget },
                valueRange = (speedRange?.minimum ?: 0.0).toFloat()..
                    (speedRange?.maximum ?: 0.0).toFloat(),
            )
            rangeLabel(speedRange?.minimum, speedRange?.maximum, speedRange?.unit.orEmpty())
                ?.let { RangeHint(it) }
        }
    }

    takeoffTarget?.let { target ->
        var probe by remember(takeoffTarget != null) { mutableStateOf<GuidedTakeoff?>(null) }
        LaunchedEffect(takeoffSettled) {
            val at = takeoffSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) { guidedTakeoff(Qgc.get(guidedTakeoffPath(at))) }
        }
        GuidedValuePanel(
            title = takeoffRange?.label?.ifBlank { null } ?: "Takeoff",
            sentence = probe?.sentence ?: "",
            commitLabel = "Take off",
            commitEnabled = probe != null,
            onCommit = {
                takeoffTarget = null
                offMainDetached {
                    val fresh = guidedTakeoff(Qgc.get(guidedTakeoffPath(target)))
                    if (fresh != null) {
                        Qgc.invoke("vehicle.guidedModeTakeoff", fresh.targetMeters)
                    }
                }
            },
            onCancel = { takeoffTarget = null },
        ) {
            takeoffRange?.let { range -> guidedBounds(range.minimum, range.maximum)?.let { range to it } }?.let { (range, bounds) ->
                GuidedStepper(target, range.label, range.unit, bounds.first, bounds.second) { stepped ->
                    takeoffTarget = stepped
                    takeoffSettled = stepped
                }
            }
            Slider(
                value = target.toFloat(),
                onValueChange = { takeoffTarget = guidedRounded(it.toDouble(), takeoffRange?.unit.orEmpty()) },
                onValueChangeFinished = { takeoffSettled = takeoffTarget },
                valueRange = (takeoffRange?.minimum ?: 0.0).toFloat()..
                    (takeoffRange?.maximum ?: 0.0).toFloat(),
            )
            rangeLabel(takeoffRange?.minimum, takeoffRange?.maximum, takeoffRange?.unit.orEmpty())
                ?.let { RangeHint(it) }
        }
    }

    altitudeTarget?.let { target ->
        var probe by remember(altitudeTarget != null) { mutableStateOf<GuidedAltitude?>(null) }
        LaunchedEffect(altitudeSettled) {
            val at = altitudeSettled ?: return@LaunchedEffect
            probe = withContext(Dispatchers.Default) {
                guidedAltitude(Qgc.get(guidedAltitudePath(at, altitudePauses)))
            }
        }
        GuidedValuePanel(
            title = if (altitudePauses) "Pause" else "Change Altitude",
            sentence = probe?.sentence ?: "",
            commitLabel = if (altitudePauses) "Pause" else "Change",
            commitEnabled = probe?.sends == true,
            onCommit = {
                altitudeTarget = null
                val pauses = altitudePauses
                offMainDetached {
                    val fresh = guidedAltitude(Qgc.get(guidedAltitudePath(target, pauses)))
                    fresh?.let { altitudeCommandArgs(it, pauses) }?.let { args ->
                        Qgc.invoke("vehicle.guidedModeChangeAltitude", *args.toTypedArray())
                    }
                }
            },
            onCancel = { altitudeTarget = null },
        ) {
            altitudeRange?.let { range -> guidedBounds(range.minimum, range.maximum)?.let { range to it } }?.let { (range, bounds) ->
                GuidedStepper(target, range.label, range.unit, bounds.first, bounds.second) { stepped ->
                    altitudeTarget = stepped
                    altitudeSettled = stepped
                }
            }
            Slider(
                value = target.toFloat(),
                onValueChange = { altitudeTarget = guidedRounded(it.toDouble(), altitudeRange?.unit.orEmpty()) },
                onValueChangeFinished = { altitudeSettled = altitudeTarget },
                valueRange = (altitudeRange?.minimum ?: 0.0).toFloat()..
                    (altitudeRange?.maximum ?: 0.0).toFloat(),
            )
            rangeLabel(altitudeRange?.minimum, altitudeRange?.maximum, altitudeRange?.unit.orEmpty())
                ?.let { RangeHint(it) }
        }
    }

    editingLoiter?.let { offer ->
        LoiterRadiusSheet(offer, mapClickUnits(mapClickJson)) { editingLoiter = null }
    }

    if (showMore) {
        val checklistPast = checklistOffered(armed)
        MoreActionsSheet(
            tiles = deckRest.filter { it.id != CHECKLIST }.map { MoreTile(it.label, it.icon, it.enabled, it.warning, it.onClick) } +
                listOfNotNull(
                    MoreTile("Checklist", R.drawable.ic_check_circle, checklistPast == null) { showChecklist = true }
                        .takeIf { preflightOffered(preflightJson) },
                    loiter?.let { offer -> MoreTile(offer.title, R.drawable.ic_my_location, true) { editingLoiter = offer } },
                ) +
                extras.filter { it.id !in deckShown }.map { offer ->
                    MoreTile(offer.title, guidedIcon(offer.id), offer.ready, offer.destructive) {
                        if (offer.id == PAUSE) {
                            openAltitude(true)
                        } else {
                            guidedCommand(offer.id, resumeFrom)?.let { command ->
                                pending = GuidedAction(
                                    offerId = offer.id,
                                    name = offer.title,
                                    confirm = offer.prompt,
                                    destructive = offer.destructive,
                                    run = command,
                                )
                            }
                        }
                    }
                },
            onDismiss = { showMore = false },
        ) {
            FlyViewMavlinkActions { showMore = false }
        }
    }

    if (showChecklist) {
        AlertDialog(
            onDismissRequest = { showChecklist = false },
            title = { Text("Pre-Flight Checklist") },
            text = {
                PreflightScreen(
                    modifier = Modifier.fillMaxWidth(),
                    ticked = checklistTicked,
                    onTicked = { checklistTicked = it },
                )
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { showChecklist = false }) { Text("Close") } },
        )
    }

}

@Composable
internal fun RangeHint(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun Offered(offer: GuidedOffer?, content: @Composable () -> Unit) {
    if (offer?.shown == true) {
        content()
    }
}

private fun armedNow(): Boolean = flyState(Qgc.get(FLY_STATE))?.armed == true

private fun flightModeNow(): String = flyState(Qgc.get(FLY_STATE))?.mode.orEmpty()

private fun CoroutineScope.attemptCommand(
    action: String,
    report: (String?) -> Unit,
    withdraw: (String) -> Unit,
    reached: () -> Boolean,
    call: () -> Unit,
) {
    launch {
        report(null)
        withContext(Dispatchers.Default) { call() }
        val confirmed = withTimeoutOrNull(COMMAND_SETTLE_MS) {
            while (!withContext(Dispatchers.Default) { reached() }) {
                delay(200)
            }
            true
        } == true
        val refused = commandRefusal(action, confirmed) ?: return@launch
        report(refused)
        while (!withContext(Dispatchers.Default) { reached() }) {
            delay(LATE_CONFIRM_POLL_MS)
        }
        withdraw(refused)
    }
}

internal const val COMMAND_SETTLE_MS = 4000L
internal const val LATE_CONFIRM_POLL_MS = 500L

internal fun withdrawn(shown: String?, late: String): String? = if (shown == late) null else shown

internal fun commandRefusal(action: String, confirmed: Boolean): String? =
    if (confirmed) null else "$action was not confirmed by the aircraft."

internal const val FLIGHT_MODE_SETTINGS_PAGE = "Flight Mode Settings"

internal object FlyRefusal {
    var text by mutableStateOf<String?>(null)
}

internal object FlightModePending {
    var mode by mutableStateOf<String?>(null)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun FlightModeMenu(expanded: Boolean, onDismiss: () -> Unit, onStatus: () -> Unit) {
    val json by qgcPath(FLIGHT_MODES)
    val modes = remember(json) { flightModesView(json) }
    val onRefusal: (String?) -> Unit = { FlyRefusal.text = it }
    val onWithdraw: (String) -> Unit = { FlyRefusal.text = withdrawn(FlyRefusal.text, it) }
    var showFolded by remember { mutableStateOf(false) }
    var editing by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf<FlightModeOption?>(null) }
    var settings by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    if (modes == null) return

    fun send(mode: FlightModeOption) {
        scope.launch {
            onRefusal(null)
            FlightModePending.mode = mode.name
            val before = withContext(Dispatchers.Default) { modeAck(Qgc.get(FLIGHT_MODES)) }
            withContext(Dispatchers.Default) { Qgc.set("vehicle.flightMode", mode.name) }
            val started = System.currentTimeMillis()
            var outcome: ModeOutcome = ModeOutcome.Pending
            while (outcome == ModeOutcome.Pending) {
                delay(200)
                outcome = withContext(Dispatchers.Default) {
                    modeOutcome(mode.name, before, modeAck(Qgc.get(FLIGHT_MODES)), flightModeNow() == mode.name, System.currentTimeMillis() - started)
                }
            }
            FlightModePending.mode = null
            (outcome as? ModeOutcome.Rejected)?.let { rejected ->
                onRefusal(rejected.text)
                delay(MODE_REJECTION_MS)
                onWithdraw(rejected.text)
            }
        }
    }

    fun choose(mode: FlightModeOption) {
        onDismiss()
        showFolded = false
        if (mode.needsConfirm) confirming = mode else send(mode)
    }

    if (settings) {
        ModalBottomSheet(onDismissRequest = { settings = false }) {
            ParameterForm(FLIGHT_MODE_SETTINGS_PAGE)
        }
    }

    confirming?.let { mode ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("Switch to ${mode.name}?") },
            text = { Text(mode.summary.ifBlank { "This mode changes how the aircraft responds." }) },
            confirmButton = {
                TextButton(onClick = { confirming = null; send(mode) }) { Text("Switch") }
            },
            dismissButton = {
                TextButton(onClick = { confirming = null }) { Text("Cancel") }
            },
        )
    }

    DropdownMenu(
        expanded = expanded,
        onDismissRequest = { onDismiss(); showFolded = false; editing = false },
        shape = MaterialTheme.shapes.small,
    ) {
        modeHeading(modes)?.let { heading ->
            Text(
                heading,
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp).widthIn(max = 280.dp),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        val setting = modes.hiddenSetting
        if (editing && setting != null) {
            modes.all.forEach { mode ->
                DropdownMenuItem(
                    text = { Text(mode.name, style = MaterialTheme.typography.bodyMedium) },
                    trailingIcon = { Switch(checked = !mode.hidden, onCheckedChange = null) },
                    onClick = {
                        val value = hiddenModesAfter(modes.hidden, mode.name, !mode.hidden)
                        scope.launch(Dispatchers.Default) { Qgc.set(setting, value) }
                    },
                )
            }
        }
        val shown = when {
            editing && setting != null -> emptyList()
            showFolded -> modes.all
            else -> modes.everyday
        }
        shown.forEach { mode ->
            DropdownMenuItem(
                modifier = if (mode.current) Modifier.background(MaterialTheme.colorScheme.secondaryContainer) else Modifier,
                text = {
                    Column(Modifier.widthIn(max = 280.dp)) {
                        Text(mode.name, style = MaterialTheme.typography.titleSmall)
                        mode.summary.ifBlank { null }?.let {
                            Text(
                                it,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                },
                enabled = modes.canSet,
                onClick = { choose(mode) },
            )
        }
        if (modes.folded.isNotEmpty() && !showFolded && !editing) {
            DropdownMenuItem(
                text = { Text("More modes") },
                onClick = { showFolded = true },
            )
        }
        DropdownMenuItem(
            text = { Text("Flight Mode Settings") },
            onClick = {
                onDismiss()
                settings = true
            },
        )
        DropdownMenuItem(
            text = { Text("Vehicle status") },
            onClick = {
                onDismiss()
                onStatus()
            },
        )
        if (modes.hiddenSetting != null) {
            DropdownMenuItem(
                text = { Text("Edit Displayed Flight Modes") },
                trailingIcon = { Switch(checked = editing, onCheckedChange = null) },
                onClick = { editing = !editing },
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun InstrumentSheet(
    chosen: List<String>,
    onToggle: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var groups by remember { mutableStateOf(emptyList<InstrumentGroup>()) }
    val connected = hasVehicle()
    LaunchedEffect(connected) {
        groups = withContext(Dispatchers.Default) {
            val catalogue = Qgc.get(INSTRUMENT_GROUPS)
            listOfNotNull(vehicleOwnGroup(catalogue)) + instrumentGroups(catalogue)
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        SectionHeader("Readings on the flight screen")
        FootNote(instrumentChoiceNote(chosen))
        if (groups.isEmpty()) {
            FootNote(emptyCatalogueText(connected))
            return@ModalBottomSheet
        }
        LazyColumn(Modifier.fillMaxWidth()) {
            groups.forEach { group ->
                item(key = "head${group.group}") { SectionHeader(group.title) }
                items(group.facts, key = { it.path }) { fact ->
                    val picked = fact.path in chosen
                    val choosable = picked || chosen.size < MOST_INSTRUMENTS
                    ListItem(
                        headlineContent = { Text(fact.label) },
                        trailingContent = {
                            Checkbox(checked = picked, onCheckedChange = null, enabled = choosable)
                        },
                        colors = ListItemDefaults.colors(
                            headlineColor = when {
                                choosable -> MaterialTheme.colorScheme.onSurface
                                else -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                            },
                        ),
                        modifier = Modifier.clickable(enabled = choosable) { onToggle(fact.path) },
                    )
                }
            }
        }
    }
}

