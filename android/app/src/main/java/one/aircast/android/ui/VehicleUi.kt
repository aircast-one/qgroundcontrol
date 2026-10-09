package one.aircast.android.ui

import one.aircast.map.aircast
import one.aircast.android.bridge.VehicleCommands
import androidx.compose.foundation.rememberScrollState

import androidx.compose.foundation.verticalScroll

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.Surface
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.CompositionLocalProvider
import one.aircast.map.AircastSpace
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
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
import one.aircast.map.AircastSheet
import androidx.compose.material3.Switch
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.alpha
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.material3.HorizontalDivider
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import androidx.compose.runtime.rememberUpdatedState
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
import one.aircast.map.TelemetryNumber
import one.aircast.map.optText

private const val MISSION_POPUP_DELAY_MS = 1000L
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


internal data class Instrument(val label: String, val reading: String, val id: String = "", val value: String = reading, val units: String = "", val raw: Double? = null, val defaultIcon: String = "")

internal fun displayFor(displays: Map<String, ValueDisplay>, instrument: Instrument): ValueDisplay =
    displays[instrument.id] ?: ValueDisplay(icon = instrument.defaultIcon)

internal fun rowWidth(count: Int): Int = when {
    count <= 4 -> count
    else -> (count + 1) / 2
}

internal fun operatorDistance(view: JSONObject?): List<Instrument> =
    view?.optText("distanceToVehicleText")
        ?.takeIf { it.isNotBlank() }
        ?.let { listOf(Instrument(label = "From you", reading = it, value = it.substringBeforeLast(' '), units = it.substringAfterLast(' ', ""))) }
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
                    defaultIcon = item.optText("defaultIcon"),
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
fun TelemetryRow(modifier: Modifier = Modifier, columns: Int? = null, valuesShown: Boolean = true, chooser: Boolean = true, compact: Boolean = false, stacked: Boolean = false) {
    val flyScreen = LocalFlyScreenState.current
    val context = LocalContext.current
    val classView by qgcPath(INSTRUMENTS_VIEW)
    val vehicleClass = instrumentVehicleClass(classView)
    var chosen by remember(vehicleClass) { mutableStateOf(readChosen(context, vehicleClass)) }
    var displays by remember(vehicleClass) { mutableStateOf(readDisplays(context, vehicleClass)) }
    var styling by remember { mutableStateOf<Instrument?>(null) }
    LaunchedEffect(vehicleClass) { flyScreen.layout.valueSize = readValueSize(context, vehicleClass) }
    val view by one.aircast.android.bridge.qgcPathHoldingLast(instrumentsPath(chosen, vehicleClass))
    val gcsJson by qgcPath(GCS_POSITION)
    val shown = remember(view, gcsJson, chosen) {
        (if (showsInstruments(chosen)) instruments(view) else emptyList()) + operatorDistance(gcsJson)
    }
    val stateJson by qgcPath(FLY_STATE)
    val stale = remember(stateJson) { flyState(stateJson)?.staleNotice.orEmpty() }
    val silent = stale.isNotBlank()

    val change: (List<String>) -> Unit = { next ->
        chosen = next
        writeChosen(context, vehicleClass, next)
        flyScreen.instrumentEdits++
        styling = null
    }
    var replacing by remember { mutableStateOf<Pair<Int, String>?>(null) }
    replacing?.let { (index, fromId) ->
        InstrumentSheet(
            chosen = chosen,
            title = "Change reading",
            onToggle = { path ->
                val carried = displays[fromId]
                if (carried != null && selectionId(path) !in displays) {
                    writeDisplay(context, vehicleClass, selectionId(path), carried)
                    displays = displays + (selectionId(path) to carried)
                }
                change(replacedInstrument(chosen, index, path))
                replacing = null
            },
            onDismiss = { replacing = null },
        )
    }

    styling?.let { instrument ->
        val index = chosen.indexOfFirst { selectionId(it) == instrument.id }.takeIf { it >= 0 }
        ValueDisplayDialog(
            label = instrument.label,
            initial = displayFor(displays, instrument),
            onDismiss = { styling = null },
            extra = {
                if (index != null) FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    TextButton(onClick = { replacing = index to instrument.id; styling = null }) { Text("Change reading") }
                    if (index > 0) TextButton(onClick = { change(movedInstrument(chosen, index, -1)) }) { Text("Move left") }
                    if (index < chosen.lastIndex) TextButton(onClick = { change(movedInstrument(chosen, index, 1)) }) { Text("Move right") }
                    TextButton(onClick = { change(removedInstrument(chosen, index)) }) { Text("Remove") }
                }
            },
            onDone = { display ->
                writeDisplay(context, vehicleClass, instrument.id, display)
                displays = displays + (instrument.id to display)
                flyScreen.instrumentEdits += 1
                styling = null
            },
        )
    }

    if (flyScreen.choosingReadings) {
        InstrumentSheet(
            chosen = chosen,
            onToggle = { name ->
                chosen = withInstrument(chosen, name)
                writeChosen(context, vehicleClass, chosen)
                flyScreen.instrumentEdits++
            },
            onDismiss = { flyScreen.choosingReadings = false },
        )
    }

    if (shown.isEmpty()) return

    if (compact) {
        val reading: @Composable (Instrument, TextStyle) -> Unit = { instrument, style ->
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1)) {
                Text(osdLabel(instrument.label), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.aircast.outdoorForeground.copy(alpha = OSD_LABEL_ALPHA), maxLines = 1, softWrap = false, modifier = Modifier.alignByBaseline())
                Text(
                    "${instrument.value}${instrument.units}",
                    style = style.copy(fontFeatureSettings = "tnum"),
                    maxLines = 1,
                    softWrap = false,
                    color = if (silent) MaterialTheme.colorScheme.onSurfaceVariant else displayColour(displayFor(displays, instrument), instrument.raw)?.let { Color(it) } ?: MaterialTheme.aircast.outdoorForeground,
                    modifier = Modifier.alignByBaseline(),
                )
            }
        }
        if (stacked) {
            val (speeds, places) = shown.partition { osdIsSpeed(it.label) }
            Column(modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
                if (speeds.isNotEmpty()) Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s4)) { speeds.map { reading(it, MaterialTheme.typography.labelLarge) } }
                if (places.isNotEmpty()) Row(horizontalArrangement = Arrangement.spacedBy(AircastSpace.s4)) { places.map { reading(it, MaterialTheme.typography.titleLarge) } }
            }
        } else {
            FlowRow(
                modifier,
                horizontalArrangement = Arrangement.spacedBy(AircastSpace.s4, Alignment.CenterHorizontally),
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                shown.map { reading(it, MaterialTheme.typography.titleMedium) }
            }
        }
        return
    }

    androidx.compose.animation.AnimatedVisibility(
        visible = valuesShown,
        enter = androidx.compose.animation.fadeIn() + androidx.compose.animation.expandVertically(),
        exit = androidx.compose.animation.fadeOut() + androidx.compose.animation.shrinkVertically(),
    ) {
    Column {
    if (silent) {
        Text(
            text = stale,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
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
            val display = displayFor(displays, instrument)
            Column(
                Modifier
                    .padding(horizontal = 12.dp, vertical = 6.dp)
                    .then(if (instrument.id.isBlank()) Modifier else Modifier.clickable { styling = instrument }),
            ) {
                ValueLabel(display, instrument.raw, instrument.label.uppercase(), MaterialTheme.colorScheme.onSurfaceVariant)
                Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(
                        instrument.value,
                        style = scaledNumber(flyScreen.layout.valueSize.scale),
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
        if (chooser) IconButton(onClick = { flyScreen.choosingReadings = true }) {
            Icon(
                painter = painterResource(R.drawable.ic_tune),
                contentDescription = "Readings",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
    }
    }
}

internal fun scaledNumber(scale: Float): TextStyle = TelemetryNumber.copy(
    fontSize = TelemetryNumber.fontSize * scale,
    lineHeight = if (TelemetryNumber.lineHeight.isSpecified) TelemetryNumber.lineHeight * scale else TelemetryNumber.lineHeight,
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun FlightActions(modifier: Modifier = Modifier, layout: FlyDeckLayout = FlyDeckLayout.Bottom, center: @Composable () -> Unit = {}) {
    val flyScreen = LocalFlyScreenState.current
    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }
    val readiness = remember(state) { guidedReadiness(state) }
    val available = state?.connected == true
    val armed = state?.armed == true
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    var sentName by remember { mutableStateOf<String?>(null) }
    var sentSnapshot by remember { mutableStateOf<String?>(null) }
    var refusal by flyScreen::refusal
    val scope = rememberCoroutineScope()
    var guidedValue by remember { mutableStateOf<OpenGuidedValue?>(null) }
    var showMore by remember { mutableStateOf(false) }
    var showGripper by remember { mutableStateOf(false) }
    var deckRest by remember { mutableStateOf<List<DeckEntry>>(emptyList()) }
    var deckShown by remember { mutableStateOf<Set<String>>(emptySet()) }
    var editingLoiter by remember { mutableStateOf<LoiterOffer?>(null) }
    val mapClickJson by qgcPath(MAP_CLICK_PATH)
    val loiter = remember(mapClickJson) { loiterOffer(mapClickJson) }
    val checklist = rememberPreflightChecklist()
    val preflightJson by qgcPath(PREFLIGHT)
    val useChecklist = remember(preflightJson) { preflightOffered(preflightJson) }
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val offers = remember(actionsJson) { guidedOffers(actionsJson) }
    val extras = remember(offers) { moreActions(offers) }
    val gripper = remember(extras) { gripperOffers(extras) }
    val resumeFrom = remember(actionsJson) { resumeFromSequence(actionsJson) }
    val automaticMissionPopups by qgcBool(settingControl("settings.flyViewSettings.enableAutomaticMissionPopups"))
    var missionReady by remember { mutableStateOf<Set<String>?>(null) }
    var popupDue by remember { mutableStateOf<GuidedOffer?>(null) }

    LaunchedEffect(offers) {
        if (offerWithdrawn(pending?.offerId, offers)) pending = null
        if (offerWithdrawn(guidedValue?.kind?.offerId, offers)) guidedValue = null
        val popup = missionReady?.let { autoMissionPopup(it, offers, automaticMissionPopups) }
        missionReady = AUTO_POPUP_ACTIONS.filter { offers[it]?.ready == true }.toSet()
        if (popup != null) popupDue = popup
    }

    val latestOffers by rememberUpdatedState(offers)
    LaunchedEffect(popupDue) {
        val due = popupDue ?: return@LaunchedEffect
        delay(MISSION_POPUP_DELAY_MS)
        popupDue = null
        val settled = latestOffers[due.id]?.takeIf { it.ready } ?: return@LaunchedEffect
        if (pending == null || popupReplacesOpenConfirm(settled.id)) {
            guidedCommand(settled.id, resumeFrom)?.let { command ->
                pending = GuidedAction(name = settled.title, confirm = settled.prompt, destructive = settled.destructive, offerId = settled.id, run = command)
            }
        }
    }

    PreflightChecklistReset(checklist, available)

    if (!available) {
        if (layout != FlyDeckLayout.Rail) Text("Connect a vehicle to enable flight controls.", modifier.padding(16.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        return
    }

    val deciding = pending != null || guidedValue != null || editingLoiter != null

    PreflightChecklist(checklist, deciding)
    OpenOnRequest("more") { showMore = true }

    val openValue: (GuidedValueKind) -> Unit = { kind ->
        scope.launch {
            val opened = openGuidedValue(kind)
            if (opened == null) refusal = kind.missingRange else guidedValue = opened
        }
    }
    val openAltitude: (Boolean) -> Unit = { pauses -> openValue(altitudeValue(pauses)) }

    val liveActions = actionsJson?.toString()
    val confirming = pending
    val showingSent = sentIsStillShowing(sentName, sentSnapshot, liveActions)

    val entries = flightDeckEntries(
        FlightDeckContext(
            offers = offers,
            armed = armed,
            scope = scope,
            confirm = { pending = it },
            openValue = openValue,
            report = { refusal = it },
            withdraw = { refusal = withdrawn(refusal, it) },
            openChecklist = checklist::open.takeIf { useChecklist },
            readiness = readiness,
        ),
    )
    val deck = deckIds(entries.map { it.id }.toSet(), armed)
    deckRest = entries.filter { entry -> deck.none { it.first == entry.id } }
    deckShown = deck.map { it.first }.toSet()
    LaunchedEffect(flyScreen.deckRequest) {
        when (val asked = flyScreen.deckRequest) {
            null -> Unit
            ARM_REQUEST -> entries.firstOrNull { it.id == ARM_REQUEST && it.enabled }?.onClick?.invoke()
                ?: armedStopOffer(offers, armed)?.let { pending = emergencyStopAction(it) }
                ?: run { refusal = deckRequestRefusal(offers[if (armed) "disarm" else "arm"]) }
            else -> entries.firstOrNull { it.id == asked && it.enabled }?.onClick?.invoke() ?: offers[asked]?.takeIf { it.ready }?.let { offer ->
                guidedCommand(offer.id, resumeFrom)?.let { command ->
                    pending = GuidedAction(offerId = offer.id, name = offer.title, confirm = offer.prompt, destructive = offer.destructive, run = command)
                }
            }
        }
        flyScreen.deckRequest = null
    }

    if (layout == FlyDeckLayout.Rail) {
        Box(modifier.fillMaxSize()) {
            if (pending == null && guidedValue == null && editingLoiter == null) {
                Column(
                    Modifier.align(Alignment.CenterStart).padding(start = AircastSpace.s3, bottom = RAIL_LIFT),
                    verticalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    deck.map { (id, primary) ->
                        entries.firstOrNull { it.id == id }?.let { entry -> RailDeckButton(entry, primary, labelled = armed) }
                    }
                    RailDeckButton(DeckEntry("more", "More", R.drawable.ic_more_vert, true) { showMore = true }, primary = false)
                }
            }
            if (refusal != null || confirming != null || showingSent) {
                Surface(
                    Modifier.align(Alignment.Center).widthIn(max = DECISION_CARD_WIDTH).padding(AircastSpace.s3),
                    shape = MaterialTheme.shapes.extraLarge,
                    color = MaterialTheme.colorScheme.surfaceContainerLow,
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        refusal?.let { message -> Text(message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error) }
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
                    }
                }
            }
        }
    } else Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {

        refusal?.let { message ->
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.fillMaxWidth(),
            )
        }

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

        Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { TelemetryRow(valuesShown = true, chooser = false, compact = true) }

        if (!deciding) readiness?.let { DeckReadiness(it) }

        if (pending == null) {
            val more = DeckEntry("more", "More", R.drawable.ic_more_vert, true) { showMore = true }
            val buttons = deck.mapNotNull { (id, primary) -> entries.firstOrNull { it.id == id }?.let { it to primary } } + (more to false)
            val leading = (buttons.size + 1) / 2
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                val button: @Composable (Pair<DeckEntry, Boolean>) -> Unit = { (entry, primary) ->
                    DeckButton(entry, primary, if (entry === more && deck.isNotEmpty()) Modifier.width(64.dp) else Modifier.weight(1f))
                }
                buttons.take(leading).map { button(it) }
                center()
                buttons.drop(leading).map { button(it) }
            }
        }
    }

    val rail = layout == FlyDeckLayout.Rail
    guidedValue?.let { open -> DecisionHost(rail) { GuidedValueFlow(open) { guidedValue = null } } }

    LaunchedEffect(loiter == null) {
        if (loiter == null) editingLoiter = null
    }
    editingLoiter?.let { offer ->
        DecisionHost(rail) { LoiterRadiusPanel(offer, mapClickUnits(mapClickJson), onRefused = { refusal = it }) { editingLoiter = null } }
    }

    if (showMore) {
        val checklistPast = checklistOffered(armed)
        MoreActionsSheet(
            tiles = listOfNotNull(armedStopOffer(offers, armed)?.let { offer -> MoreTile(STOP_MOTORS, R.drawable.ic_warning, true, warning = true) { pending = emergencyStopAction(offer) } }) +
                deckRest.filter { it.id != CHECKLIST }.map { MoreTile(it.label, it.icon, it.enabled, it.warning, it.onClick) } +
                listOfNotNull(
                    MoreTile("Checklist", R.drawable.ic_check_circle, checklistPast == null) { checklist.open() }
                        .takeIf { preflightOffered(preflightJson) },
                    loiter?.let { offer -> MoreTile(offer.title, R.drawable.ic_my_location, true) { editingLoiter = offer } },
                    MoreTile("Gripper", R.drawable.ic_download, gripper.any { it.ready }) { showGripper = true }.takeIf { gripper.isNotEmpty() },
                    MoreTile("Choose readings", R.drawable.ic_tune, true) { flyScreen.choosingReadings = true },
                    MoreTile("Edit layout", R.drawable.ic_edit, true) { flyScreen.layout.startEditing() }.takeIf { !armed },
                ) +
                extras.filter { it.id !in deckShown && it.id !in GRIPPER_ACTIONS }.map { offer ->
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

    LaunchedEffect(gripper.isEmpty()) {
        if (gripper.isEmpty()) showGripper = false
    }
    if (showGripper && gripper.isNotEmpty()) GripperPanel(gripper) { showGripper = false }


}

private val DECISION_CARD_WIDTH = 460.dp
private const val OSD_LABEL_ALPHA = 0.75f
internal const val STOP_MOTORS = "Stop motors"
private val RAIL_LIFT = 56.dp

private val OSD_LABELS = mapOf(
    "distance to home" to "D",
    "alt (rel)" to "H",
    "altitude" to "H",
    "ground speed" to "H.S",
    "climb rate" to "V.S",
    "air speed" to "A.S",
    "airspeed" to "A.S",
    "distance to operator" to "D.OP",
    "from you" to "D.OP",
    "heading" to "HDG",
    "distancetohome" to "D",
    "altituderelative" to "H",
    "groundspeed" to "H.S",
    "climbrate" to "V.S",
)

internal fun osdLabel(label: String): String = OSD_LABELS[label.trim().lowercase()] ?: label.uppercase()

internal fun osdIsSpeed(label: String): Boolean = osdLabel(label) in setOf("H.S", "V.S", "A.S")

@Composable
private fun DecisionHost(rail: Boolean, content: @Composable () -> Unit) {
    if (rail) {
        Box(Modifier.fillMaxSize().padding(AircastSpace.s3), contentAlignment = Alignment.Center) {
            Box(Modifier.widthIn(max = DECISION_CARD_WIDTH)) { content() }
        }
    } else {
        content()
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

private fun Modifier.longPress(key: Any?, action: () -> Unit): Modifier =
    if (key == null) this else pointerInput(key) {
        awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            val released = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) { waitForUpOrCancellation(PointerEventPass.Initial); true }
            if (released == null) {
                action()
                waitForUpOrCancellation(PointerEventPass.Initial)?.consume()
            }
        }
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun FlightModeMenu(expanded: Boolean, onDismiss: () -> Unit, onStatus: () -> Unit) {
    val navigation = LocalAppNavigation.current
    val flyScreen = LocalFlyScreenState.current
    val json by qgcPath(FLIGHT_MODES)
    val modes = remember(json) { flightModesView(json) }
    val flyJson by qgcPath(FLY_STATE)
    val fly = remember(flyJson) { flyState(flyJson) }
    val setupJson by qgcPath(SETUP)
    val hasModesPage = remember(setupJson) { setupComponents(setupJson).any { it.name == FLIGHT_MODES_PAGE } }
    val onRefusal: (String?) -> Unit = { flyScreen.refusal = it }
    val onWithdraw: (String) -> Unit = { flyScreen.refusal = withdrawn(flyScreen.refusal, it) }
    var showFolded by remember { mutableStateOf(false) }
    var editing by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf<FlightModeOption?>(null) }
    var settings by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    if (modes == null) return

    fun send(mode: FlightModeOption) {
        scope.launch {
            onRefusal(null)
            flyScreen.pendingMode = mode.name
            val before = withContext(Dispatchers.Default) { modeAck(Qgc.get(FLIGHT_MODES)) }
            withContext(Dispatchers.Default) { VehicleCommands.setFlightMode(mode.name) }
            val started = System.currentTimeMillis()
            var outcome: ModeOutcome = ModeOutcome.Pending
            while (outcome == ModeOutcome.Pending) {
                delay(200)
                outcome = withContext(Dispatchers.Default) {
                    modeOutcome(mode.name, before, modeAck(Qgc.get(FLIGHT_MODES)), flightModeNow() == mode.name, System.currentTimeMillis() - started)
                }
            }
            flyScreen.pendingMode = null
            (outcome as? ModeOutcome.Rejected)?.let { rejected ->
                onRefusal(rejected.text)
                delay(MODE_REJECTION_MS)
                onWithdraw(rejected.text)
            }
        }
    }

    fun toggleHidden(name: String) {
        val setting = modes.hiddenSetting ?: return
        scope.launch(Dispatchers.Default) {
            flightModesView(Qgc.get(FLIGHT_MODES))?.let { now -> Qgc.set(setting, hiddenModesAfter(now.hidden, name, name !in now.hidden)) }
        }
    }

    fun choose(mode: FlightModeOption) {
        onDismiss()
        showFolded = false
        if (mode.needsConfirm) confirming = mode else send(mode)
    }

    if (settings) {
        AircastSheet(onDismissRequest = { settings = false }) {
            ParameterForm(FLIGHT_MODE_SETTINGS_PAGE)
            if (hasModesPage && advancedUiShown()) {
                TextButton(
                    onClick = {
                        settings = false
                        navigation.setupPage = FLIGHT_MODES_PAGE
                    },
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
                ) { Text("Configure flight modes") }
            }
        }
    }

    confirming?.let { mode ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("Set flight mode") },
            text = { Text("Set the vehicle flight mode to ${mode.name}") },
            confirmButton = {
                TextButton(onClick = { confirming = null; send(mode) }) { Text("Confirm") }
            },
            dismissButton = {
                TextButton(onClick = { confirming = null }) { Text("Cancel") }
            },
        )
    }

    DropdownMenu(
        expanded = expanded,
        onDismissRequest = { onDismiss(); showFolded = false; editing = false },
        shape = MaterialTheme.shapes.large,
    ) {
        val modeRow: @Composable (FlightModeOption, Boolean) -> Unit = { mode, dimmed ->
            DropdownMenuItem(
                modifier = (if (mode.current) Modifier.background(MaterialTheme.colorScheme.secondaryContainer) else Modifier)
                    .alpha(if (dimmed && !mode.current) HIDDEN_MODE_ALPHA else 1f)
                    .longPress(modes.hiddenSetting?.takeIf { showFolded }?.let { setting -> mode.name to setting }) { toggleHidden(mode.name) },
                leadingIcon = { Icon(painterResource(flightModeIcon(mode.name)), null) },
                trailingIcon = if (mode.current) {
                    { Icon(Icons.Filled.Check, contentDescription = "Current mode", tint = MaterialTheme.colorScheme.primary) }
                } else null,
                text = {
                    Column(Modifier.widthIn(max = MODE_TEXT_WIDTH)) {
                        Text(mode.name, style = MaterialTheme.typography.titleSmall)
                        mode.summary.ifBlank { null }?.let {
                            Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                },
                enabled = modes.canSet,
                onClick = { if (!mode.current) choose(mode) else onDismiss() },
            )
        }
        modes.unknownModeNotice.ifBlank { null }?.let { notice ->
            Text(
                notice,
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp).widthIn(max = MODE_TEXT_WIDTH),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.aircast.warning,
            )
        }
        val setting = modes.hiddenSetting
        if (!showFolded) {
            val primary = primaryModes(modes.all)
            modes.all.firstOrNull { it.current && it !in primary }?.let { now -> modeRow(now, false) }
            primary.forEach { mode -> modeRow(mode, false) }
            HorizontalDivider()
            DropdownMenuItem(
                text = { Text(ALL_MODES) },
                trailingIcon = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) },
                onClick = { showFolded = true },
            )
        } else {
            Row(Modifier.padding(start = 16.dp, end = 4.dp).widthIn(max = MODE_TEXT_WIDTH + 16.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    modeHeading(modes) ?: ALL_MODES,
                    Modifier.weight(1f).padding(vertical = 8.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (setting != null) TextButton(onClick = { editing = !editing }) { Text(if (editing) "Done" else "Edit") }
            }
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
            } else {
                modes.all.forEachIndexed { index, mode ->
                    if (startsSection(modes.all, index)) HorizontalDivider()
                    modeRow(mode, mode.hidden)
                }
            }
            HorizontalDivider()
            DropdownMenuItem(
                text = { Text("Flight mode settings") },
                onClick = {
                    onDismiss()
                    settings = true
                },
            )
        }
        HorizontalDivider()
        val warning = readinessWarning(fly)
        DropdownMenuItem(
            text = { Text(VEHICLE_STATUS, style = MaterialTheme.typography.bodyMedium) },
            leadingIcon = {
                warning?.let { Icon(painterResource(R.drawable.ic_warning), null, tint = MaterialTheme.aircast.warning) }
                    ?: Icon(painterResource(R.drawable.ic_check_circle), null)
            },
            trailingIcon = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null) },
            onClick = { onDismiss(); onStatus() },
        )
    }
}

private val MODE_TEXT_WIDTH = 260.dp
internal const val ALL_MODES = "All modes"

private val POSITION_MODES = listOf("Position", "Position Hold", "PosHold", "Loiter", "Hold")
private val ALTITUDE_MODES = listOf("Altitude", "Altitude Hold", "AltHold")
private val MANUAL_MODES = listOf("Stabilized", "Stabilize", "Manual")
private val MISSION_MODES = listOf("Mission", "Auto")
private const val PRIMARY_MODE_COUNT = 4

internal fun primaryModes(all: List<FlightModeOption>): List<FlightModeOption> {
    val offered = all.filterNot { it.hidden }
    val picked = listOf(POSITION_MODES, ALTITUDE_MODES, MANUAL_MODES, MISSION_MODES).mapNotNull { names ->
        names.firstNotNullOfOrNull { name -> offered.firstOrNull { it.name.equals(name, ignoreCase = true) } }
    }
    return if (picked.size >= 2) picked else offered.take(PRIMARY_MODE_COUNT)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun InstrumentSheet(
    chosen: List<String>,
    title: String = "Instrument tiles",
    onToggle: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var groups by remember { mutableStateOf(emptyList<InstrumentGroup>()) }
    val shownAtOpen = remember { chosen }
    val connected = hasVehicle()
    LaunchedEffect(connected) {
        groups = withContext(Dispatchers.Default) {
            val catalogue = Qgc.get(INSTRUMENT_GROUPS)
            listOfNotNull(vehicleOwnGroup(catalogue)) + instrumentGroups(catalogue)
        }
    }

    AircastSheet(onDismissRequest = onDismiss) {
        Text(title, style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(horizontal = 16.dp))
        Text(instrumentChoiceNote(chosen), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp).padding(bottom = 8.dp))
        if (groups.isEmpty()) {
            FootNote(emptyCatalogueText(connected))
            return@AircastSheet
        }
        LazyColumn(Modifier.fillMaxWidth()) {
            shownFirst(groups, shownAtOpen).forEach { group ->
                item(key = "head${group.group}") { SectionHeader(sentenceCase(group.title)) }
                items(group.facts, key = { it.path }) { fact ->
                    ListItem(
                        headlineContent = { Text(sentenceCase(fact.label)) },
                        leadingContent = { Checkbox(checked = fact.path in chosen, onCheckedChange = null) },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                        modifier = Modifier.clickable { onToggle(fact.path) },
                    )
                }
            }
        }
    }
}


private val FLIGHT_MODE_ICONS = listOf(
    listOf("rtl", "return", "smart_rtl", "smartrtl") to R.drawable.ic_home,
    listOf("land", "precland") to R.drawable.ic_flight_land,
    listOf("takeoff") to R.drawable.ic_flight_takeoff,
    listOf("auto", "mission") to R.drawable.ic_route,
    listOf("guided", "offboard", "follow") to R.drawable.ic_location_on,
    listOf("althold", "altitude", "alt") to R.drawable.ic_height,
    listOf("loiter", "position", "poshold", "hold", "brake") to R.drawable.ic_my_location,
    listOf("stabilize", "stabilized", "manual", "acro", "sport", "drift") to R.drawable.ic_gamepad,
)

internal fun flightModeIcon(name: String): Int =
    name.lowercase().split(' ', '_', '-').let { words -> words.joinToString("") to words }.let { (joined, words) ->
        FLIGHT_MODE_ICONS.firstOrNull { (keys, _) -> keys.any { it == joined || it in words } }?.second ?: R.drawable.ic_flight
    }

@Composable
private fun DeckReadiness(readiness: Readiness) {
    Surface(
        color = if (readiness.blocks) MaterialTheme.colorScheme.errorContainer else MaterialTheme.aircast.warningContainer,
        contentColor = if (readiness.blocks) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onSurface,
        shape = MaterialTheme.shapes.medium,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(horizontal = 12.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Icon(painterResource(R.drawable.ic_warning), null, tint = if (readiness.blocks) MaterialTheme.colorScheme.error else MaterialTheme.aircast.warning, modifier = Modifier.size(20.dp))
            Text(readiness.text, style = MaterialTheme.typography.bodyMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

internal const val VEHICLE_STATUS = "Vehicle status"

internal class FlightDeckContext(
    val offers: Map<String, GuidedOffer>,
    val armed: Boolean,
    val scope: CoroutineScope,
    val confirm: (GuidedAction) -> Unit,
    val openValue: (GuidedValueKind) -> Unit,
    val report: (String?) -> Unit,
    val withdraw: (String) -> Unit,
    val openChecklist: (() -> Unit)?,
    val readiness: Readiness? = null,
)

internal fun flightDeckEntries(deck: FlightDeckContext): List<DeckEntry> = with(deck) {
    val armAction = offers[if (armed) "disarm" else "arm"]
    listOfNotNull(
        DeckEntry(
            id = "arm",
            label = armAction?.title ?: if (armed) "Disarm" else "Arm",
            icon = R.drawable.ic_bolt,
            enabled = armAction?.ready == true,
            warning = armed,
        ) {
            confirm(GuidedAction(
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
                    report = report,
                    withdraw = withdraw,
                    reached = { armedNow() == target },
                ) { VehicleCommands.setArmed(target) }
            })
        }.takeIf { armAction?.shown == true },
        DeckEntry(
            "takeoff",
            takeoffLabel(offers["takeoff"], readiness),
            R.drawable.ic_flight_takeoff,
            offers["takeoff"]?.ready == true,
            onHold = if (readiness != null) null else fun() {
                scope.launch {
                    report(withContext(Dispatchers.Default) {
                        val heightless = offers["takeoff"]?.carriesValue == false
                        val target = if (heightless) null else holdTakeoffHeight(guidedTakeoff(Qgc.get(GUIDED_TAKEOFF)))?.let { height -> guidedTakeoff(Qgc.get(guidedTakeoffPath(height))) }
                        when {
                            heightless -> VehicleCommands.takeoffRefusal()
                            target == null -> "This vehicle did not report a takeoff height range."
                            else -> VehicleCommands.takeoffRefusal(target.targetMeters)
                        }
                    })
                }
            },
        ) {
            if (offers["takeoff"]?.carriesValue == false) {
                confirm(GuidedAction(
                    offerId = "takeoff",
                    name = offers["takeoff"]?.title ?: "Takeoff",
                    confirm = offers["takeoff"]?.prompt?.ifBlank { null } ?: "Takeoff from ground and hold position.",
                    destructive = false,
                ) {
                    offMainDetached { VehicleCommands.takeoff() }
                })
                return@DeckEntry
            }
            openValue(takeoffValue(offers["takeoff"]))
        }.takeIf { offers["takeoff"]?.let { it.shown || it.reasonCode == NO_SIGNAL_CODE } == true },
        DeckEntry(PAUSE, offers[PAUSE]?.title ?: "Pause", R.drawable.ic_pause, offers[PAUSE]?.ready == true) {
            openValue(altitudeValue(true))
        }.takeIf { offers[PAUSE]?.shown == true },
        DeckEntry("rtl", "Return", R.drawable.ic_home, offers["rtl"]?.ready == true) {
            confirm(GuidedAction(
                offerId = "rtl",
                name = offers["rtl"]?.title ?: "Return",
                confirm = offers["rtl"]?.prompt?.ifBlank { null } ?: "Return to the launch position of the vehicle",
                destructive = false,
                option = offers["rtl"]?.option?.ifBlank { null }?.let { label ->
                    ConfirmOption(label) { smart -> offMainDetached { VehicleCommands.returnToLaunch(smart) } }
                },
            ) {
                offMainDetached { VehicleCommands.returnToLaunch(false) }
            })
        }.takeIf { offers["rtl"]?.shown == true },
        DeckEntry(
            "land",
            holdLabel(offers["land"]?.title ?: "Land"),
            R.drawable.ic_flight_land,
            offers["land"]?.ready == true,
            onHold = {
                scope.launch { report(withContext(Dispatchers.Default) { VehicleCommands.landRefusal() }) }
            },
        ) {
            confirm(GuidedAction(
                offerId = "land",
                name = offers["land"]?.title ?: "Land",
                confirm = offers["land"]?.prompt?.ifBlank { null }
                    ?: "Land the vehicle at the current position",
                destructive = false,
            ) {
                offMainDetached { VehicleCommands.land() }
            })
        }.takeIf { offers["land"]?.shown == true },
        DeckEntry("changeSpeed", "Speed", R.drawable.ic_speed, offers["changeSpeed"]?.ready == true) {
            openValue(speedValue(offers["changeSpeed"]))
        }.takeIf { offers["changeSpeed"]?.shown == true },
        DeckEntry("changeAltitude", "Altitude", R.drawable.ic_height, offers["changeAltitude"]?.ready == true) {
            openValue(altitudeValue(false))
        }.takeIf { offers["changeAltitude"]?.shown == true },
        DeckEntry(CHECKLIST, "Checklist", R.drawable.ic_check_circle, true) {
            openChecklist?.invoke()
        }.takeIf { openChecklist != null && checklistOffered(armed) == null },
    )
}
