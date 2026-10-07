package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.BoxWithConstraints
import one.aircast.android.R

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import one.aircast.android.bridge.qgcPath
import androidx.compose.ui.Alignment
import one.aircast.map.optText
import one.aircast.map.aircast
import androidx.compose.material3.OutlinedButton


internal data class FirmwareLine(val summary: String, val vehicleType: String)

internal fun firmwareLine(view: JSONObject?): FirmwareLine =
    FirmwareLine(view?.optText("summary").orEmpty(), view?.optText("vehicleType").orEmpty())

internal data class SetupComponent(
    val index: Int,
    val name: String,
    val known: String? = null,
    val className: String = "",
    val needsAttention: Boolean,
    val blockedReason: String? = null,
    val prerequisite: String? = null,
)

internal fun setupIcon(known: String?, className: String = ""): Int = when (known) {
    "radio", "joystick" -> R.drawable.ic_gamepad
    "flightModes" -> R.drawable.ic_toggle_on
    "sensors" -> R.drawable.ic_sensors
    "safety" -> R.drawable.ic_shield
    "power" -> R.drawable.ic_bolt
    "esc" -> R.drawable.ic_tune
    else -> COMPONENT_ICONS.entries.firstOrNull { (token, _) -> token in className }?.value ?: R.drawable.ic_build
}

internal fun setupSubtitle(vehicle: String, firmware: String): String =
    listOf(firmware, vehicle).filter { it.isNotBlank() }.joinToString(" · ")

internal fun readinessNote(readiness: SetupReadiness?, listed: Boolean): String? =
    readiness?.takeIf { it.setupComplete != true && !listed }?.let { listOf(it.headline, it.detail).filter(String::isNotBlank).joinToString(". ") }?.ifBlank { null }

internal fun parameterCountText(count: Int): String? =
    count.takeIf { it > 0 }?.let { "%,d parameters".format(java.util.Locale.US, it) }

internal fun attentionAction(className: String): String =
    if (listOf("Sensors", "Radio").any { it in className }) "Calibrate" else "Set up"

internal fun setupNote(className: String): String =
    COMPONENT_NOTES.entries.firstOrNull { (token, _) -> token in className }?.value.orEmpty()

private val COMPONENT_NOTES = linkedMapOf(
    "Failsafe" to "What it does when the battery, radio or link fails",
    "FlightSafety" to "Return altitude, landing speed and geofence",
    "Safety" to "Return altitude, landing speed and geofence",
    "FlightModes" to "Which mode each switch position selects",
    "Airframe" to "The frame type the autopilot flies",
    "SubFrame" to "The frame type the autopilot flies",
    "Gimbal" to "Camera mount axes and their limits",
    "Joystick" to "A gamepad as the stick instead of a radio",
    "Logging" to "What the autopilot records and when",
    "Motor" to "Spin each motor to check order and direction",
    "Power" to "Battery monitor, capacity and voltage",
    "Radio" to "Calibrate the sticks and switches",
    "RemoteSupport" to "Share telemetry with a support engineer",
    "Scripting" to "Lua scripts running on the autopilot",
    "Sensors" to "Compass, accelerometer and level",
    "AdvancedTuning" to "Every rate and filter, per axis",
    "Tuning" to "How it responds to the sticks",
    "Actuator" to "Outputs and what drives them",
    "Servo" to "Outputs and what drives them",
    "Follow" to "Follow a target or this phone",
    "Camera" to "Camera trigger and gimbal",
)

private val COMPONENT_ICONS = linkedMapOf(
    "Failsafe" to R.drawable.ic_warning,
    "Airframe" to R.drawable.ic_flight,
    "SubFrame" to R.drawable.ic_flight,
    "Gimbal" to R.drawable.ic_photo_camera,
    "Logging" to R.drawable.ic_description,
    "Motor" to R.drawable.ic_speed,
    "RemoteSupport" to R.drawable.ic_link,
    "Scripting" to R.drawable.ic_terminal,
    "Tuning" to R.drawable.ic_tune,
    "Actuator" to R.drawable.ic_speed,
    "Servo" to R.drawable.ic_tune,
    "Follow" to R.drawable.ic_my_location,
    "Airspeed" to R.drawable.ic_speed,
    "Lights" to R.drawable.ic_bolt,
    "ESP8266" to R.drawable.ic_wifi,
    "Syslink" to R.drawable.ic_wifi,
)

internal fun prerequisiteText(first: String, wanted: String): String = "${sentenceCase(first)} has to be set up before ${sentenceCase(wanted)}."

internal data class ParameterWait(val title: String, val body: String, val downloadOffered: Boolean = false)

internal const val PARAMETER_REFRESH = "parameterTools.refresh"

internal const val PARAMETERS_STOPPED = "Setup needs them. Disconnect and connect the link to ask again."

internal fun parameterWait(view: JSONObject?): ParameterWait? {
    if (view == null || view.optBoolean("parametersReady")) return null
    return when (val reason = view.optText("parametersReason")) {
        "" -> null
        "noVehicle" -> null
        "loading" -> ParameterWait("Loading parameters from the vehicle.", "")
        "skipped" -> ParameterWait(view.optText("parametersText"), "", downloadOffered = true)
        else -> ParameterWait(
            view.optText("parametersText").ifBlank {
                "This vehicle has not sent its parameters ($reason)."
            },
            PARAMETERS_STOPPED,
        )
    }
}

internal fun parametersIncomplete(view: JSONObject?): String? =
    view?.takeIf { it.optText("parametersReason") == "incomplete" }?.let { "Parameters Incomplete. ${it.optText("parametersText")}" }

internal const val SETUP_PARAMETERS_PAGE = "Parameters"
internal const val SETUP_OVERVIEW_PAGE = ""
private const val NAVIGATION_SETTLE_MS = 500L
internal const val SETUP_SUMMARY = "view.setupSummary"
internal const val SETUP_SUMMARY_PAGE = "Summary"
private const val SETUP_SUMMARY_POLL_MS = 2000L

internal fun setupSummaries(view: JSONObject?): Map<String, List<SummaryLine>> {
    val listed = view?.optJSONArray("components") ?: return emptyMap()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it) }.associate { component ->
        val rows = component.optJSONArray("rows")
        component.optText("name") to (0 until (rows?.length() ?: 0)).mapNotNull { rows?.optJSONObject(it) }.map { SummaryLine(it.optText("label"), it.optText("value"), it.optBoolean("warn")) }
    }
}

internal fun setupMatches(name: String, search: String): Boolean =
    search.isBlank() || name.lowercase().contains(search.trim().lowercase())

internal fun remainingSetup(components: List<SetupComponent>): List<SetupComponent> =
    components.filterNot { it.needsAttention }

internal fun setupComponents(view: JSONObject?): List<SetupComponent> {
    val listed = view?.optJSONArray("components") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        val element = listed.optJSONObject(index) ?: return@mapNotNull null
        val name = element.optText("name").takeIf { it.isNotBlank() } ?: return@mapNotNull null
        SetupComponent(
            index = index,
            name = name,
            known = element.optText("known").takeIf { !element.isNull("known") && it.isNotBlank() },
            className = element.optText("className"),
            needsAttention = element.optBoolean("needsAttention"),
            blockedReason = element.optText("blockedReason")
                .takeIf { !element.isNull("blockedReason") && it.isNotBlank() },
            prerequisite = element.optText("prerequisite").takeIf { !element.isNull("prerequisite") && it.isNotBlank() },
        )
    }
}

private val OWN_SCREEN_HEADS = setOf(SENSORS, RADIO, REMOTE_SUPPORT, MOTORS, FLIGHT_MODES_PAGE)

@Composable
private fun SectionHits(titles: List<String>, onOpen: (String) -> Unit) {
    titles.forEach { title ->
        androidx.compose.material3.TextButton(onClick = { onOpen(title) }, modifier = Modifier.padding(start = 56.dp)) { Text(sentenceCase(title)) }
    }
}

internal fun disabledWhile(reason: String): String = "Disabled while the vehicle is $reason"

internal fun Modifier.swallowTouches(): Modifier = pointerInput(Unit) {
    awaitPointerEventScope {
        while (true) {
            awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial).changes.forEach { it.consume() }
        }
    }
}

@Composable
private fun SetupNotice(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

internal fun reportsOpening(prerequisite: String?, page: SetupPage?): Boolean =
    prerequisite != null || (page?.parameterSections != true && page?.screen != NOT_SUPPORTED_SCREEN)

@Composable
fun SetupScreen(modifier: Modifier = Modifier) {
    val navigation = LocalAppNavigation.current
    val setupJson by qgcPath(SETUP)
    val setup = remember(setupJson) { setupReadiness(setupJson) }
    val hasVehicle = setup?.connected == true
    val firmwareJson by qgcPath(FIRMWARE)
    val line = remember(firmwareJson) { firmwareLine(firmwareJson) }
    val vehicleType = line.vehicleType
    var opened by remember { mutableStateOf<Pair<SetupComponent, String?>?>(null) }
    val openComponent = opened?.first
    val openSection = opened?.second
    var parametersOpen by remember { mutableStateOf(false) }
    val advanced = advancedUiShown()
    var setupSearch by remember { mutableStateOf("") }
    var parametersSearch by remember { mutableStateOf("") }

    BackHandler(enabled = openComponent != null) { opened = null }

    val components = remember(setupJson) { setupComponents(setupJson) }
    val searching = setupSearch.isNotBlank()
    val formSections by produceState(emptyMap<String, List<ParameterRows>>(), searching, components) {
        value = if (!searching) emptyMap() else withContext(Dispatchers.Default) {
            components.filter { setupPage(setupJson, it.name)?.parameterSections == true && headPage(it) !in OWN_SCREEN_HEADS }
                .associate { component -> component.name to readPage(component.name) }
        }
    }
    val sectionHits = formSections.mapValues { (_, rows) -> rows.filter { sectionMatches(it, setupSearch) }.map { it.title } }
    val searchHit: (SetupComponent) -> Boolean = { setupMatches(it.name, setupSearch) || sectionHits[it.name].orEmpty().isNotEmpty() }
    val openFromList: (SetupComponent, String?) -> Unit = { component, section ->
        parametersOpen = false
        opened = component to section
    }
    var summaries by remember { mutableStateOf(emptyMap<String, List<SummaryLine>>()) }
    LaunchedEffect(hasVehicle) {
        if (!hasVehicle) summaries = emptyMap()
    }

    LaunchedEffect(navigation.setupPage, components, hasVehicle) {
        navigation.setupPage?.takeIf { hasVehicle && components.isNotEmpty() }?.let { requested ->
            when (requested) {
                SETUP_PARAMETERS_PAGE -> {
                    parametersSearch = ""
                    parametersOpen = true
                }
                else -> components.firstOrNull { it.name == requested }?.let { opened = it to null }
            }
            delay(NAVIGATION_SETTLE_MS)
            navigation.setupPage = null
        }
    }

    LaunchedEffect(hasVehicle) {
        if (!hasVehicle) {
            opened = null
        }
    }

    if (!hasVehicle) {
        Column(modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            EmptyState(R.drawable.ic_build, NO_VEHICLE_HEADLINE, NO_VEHICLE_TEXT)
            OutlinedButton(onClick = { navigation.settingsPage = "Connections" }) { Text("Set up connection") }
        }
        return
    }

    parameterWait(setupJson)?.let { waiting ->
        Column(modifier.fillMaxSize()) {
            SetupNotice(waiting.title)
            if (waiting.body.isNotBlank()) {
                Text(
                    text = waiting.body,
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 24.dp),
                )
            }
            if (waiting.downloadOffered) {
                androidx.compose.material3.Button(
                    onClick = { one.aircast.android.bridge.offMainDetached { Qgc.invoke(PARAMETER_REFRESH) } },
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                ) { Text("Download parameters") }
            }
        }
        return
    }

    val parametersPage: (@Composable (Modifier) -> Unit)? = if (parametersOpen) {
        { pane ->
            Column(pane.fillMaxSize()) {
                PageTopBar("Parameters", "Back to Setup") { parametersOpen = false }
                ParametersScreen(Modifier.weight(1f), initialSearch = parametersSearch)
            }
        }
    } else {
        null
    }

    val componentPage: (@Composable (Modifier) -> Unit)? = openComponent?.takeIf { headCanOpen(setupPage(setupJson, it.name), it.name) }?.let { open -> { pane ->
        Column(pane.fillMaxSize()) {
            PageTopBar(sentenceCase(open.name), "Back to Setup") { opened = null }
            val nativePage = setupPage(setupJson, open.name)
            val blocked = open.blockedReason
            val first = open.prerequisite
            val reportsLookups = reportsOpening(first, nativePage)
            LaunchedEffect(open.name, reportsLookups) {
                if (reportsLookups) withContext(Dispatchers.Default) { Qgc.invoke(SETUP_PAGE_OPENED, open.name) }
            }
            val page: @Composable (Modifier) -> Unit = { area -> Column(area) { when {
                first != null -> androidx.compose.foundation.layout.Column(Modifier.weight(1f).fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                    val firstComponent = setupComponents(setupJson).firstOrNull { it.name == first }
                    EmptyState(setupIcon(firstComponent?.known, firstComponent?.className.orEmpty()), "${sentenceCase(first)} first", prerequisiteText(first, open.name))
                    androidx.compose.material3.Button(onClick = {
                        opened = setupComponents(setupJson).firstOrNull { it.name == first }?.let { it to null }
                    }) { Text("Set up ${sentenceCase(first)}") }
                }
                headPage(open) == SENSORS -> SensorsScreen(Modifier.weight(1f))
                headPage(open) == RADIO -> RadioScreen(Modifier.weight(1f))
                headPage(open) == REMOTE_SUPPORT -> RemoteSupportScreen(Modifier.weight(1f))
                nativePage?.screen == APM_SUB_MOTORS_SCREEN -> ApmSubMotorsScreen(Modifier.weight(1f))
                headPage(open) == MOTORS -> MotorsScreen(Modifier.weight(1f))
                headPage(open) == FLIGHT_MODES_PAGE -> FlightModesSetup(Modifier.weight(1f))
                nativePage?.screen == PX4_TUNING_SCREEN -> Px4TuningScreen(Modifier.weight(1f))
                nativePage?.screen == PX4_AIRFRAME_SCREEN -> Px4AirframeScreen(Modifier.weight(1f))
                nativePage?.screen == ACTUATORS_SCREEN -> ActuatorsScreen(Modifier.weight(1f))
                nativePage?.screen == APM_SERVOS_SCREEN -> ApmServosScreen(Modifier.weight(1f))
                nativePage?.screen == APM_FOLLOW_SCREEN -> ApmFollowScreen(Modifier.weight(1f))
                nativePage?.screen == SCRIPTING_SCREEN -> ScriptingScreen(Modifier.weight(1f))
                nativePage?.screen == JOYSTICK_SCREEN -> JoystickScreen(Modifier.weight(1f))
                nativePage?.screen == ESP_BRIDGE_SCREEN -> EspBridgeScreen(Modifier.weight(1f))
                nativePage?.screen == SYSLINK_SCREEN -> SyslinkScreen(Modifier.weight(1f))
                nativePage?.screen == APM_SUB_FRAME_SCREEN -> ApmSubFrameScreen(Modifier.weight(1f))
                nativePage?.screen == APM_AIRFRAME_SCREEN -> ApmAirframeScreen(Modifier.weight(1f))
                nativePage?.screen == OPTICAL_FLOW_SCREEN -> OpticalFlowScreen(Modifier.weight(1f))
                nativePage?.screen == NOT_SUPPORTED_SCREEN -> SetupNotice("Not supported", Modifier.weight(1f))
                nativePage?.parameterSections == true -> {
                    if (open.known == "power") PowerLiveCard()
                    ParameterForm(open.name, Modifier.weight(1f), section = openSection)
                }
                else -> SetupNotice(
                    "${open.name} is set up on the desktop.",
                    Modifier.weight(1f),
                )
            } } }
            when (blocked) {
                null -> page(Modifier.weight(1f))
                else -> {
                    Text(
                        disabledWhile(blocked),
                        color = MaterialTheme.aircast.warning,
                        fontWeight = androidx.compose.ui.text.font.FontWeight.Bold,
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
                    )
                    Box(Modifier.weight(1f).fillMaxWidth()) {
                        page(Modifier.fillMaxSize())
                        Box(Modifier.matchParentSize().background(Color.Black.copy(alpha = 0.5f)).swallowTouches())
                    }
                }
            }
        }
    } }

    val firmware = line.summary
    val needSetup = components.filter { it.needsAttention && searchHit(it) }

    val overview: @Composable (Modifier) -> Unit = { pane ->
    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) { Qgc.invoke(SETUP_PAGE_OPENED, SETUP_SUMMARY_PAGE) }
        while (true) {
            summaries = withContext(Dispatchers.Default) { setupSummaries(Qgc.get(SETUP_SUMMARY)) }
            delay(SETUP_SUMMARY_POLL_MS)
        }
    }
    val parametersAreReady = remember(setupJson) { parametersReady(setupJson) }
    val parameterCount by produceState(0, parametersAreReady) {
        value = if (parametersAreReady) withContext(Dispatchers.Default) { parameterNames().size } else 0
    }
    LazyColumn(pane.fillMaxSize()) {
        item(key = "verdict") {
            val readiness = setup
            ReadinessHeader(
                vehicle = vehicleType.ifBlank { "Vehicle" },
                firmware = firmware,
                vehicleId = readiness?.vehicleId,
                note = readinessNote(readiness, components.any { it.needsAttention }),
            )
        }

        item(key = "search") {
            SearchPill(
                value = setupSearch,
                onValueChange = { setupSearch = it },
                placeholder = "Search",
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = {
                    if (advanced && setupSearch.isNotBlank()) {
                        parametersSearch = setupSearch.trim()
                        parametersOpen = true
                    }
                }),
            )
        }

        if (needSetup.isNotEmpty()) {
            item(key = "attention") { SectionHeader("Needs attention") }
            items(needSetup, key = { "a${it.index}" }) { component ->
                val blocked = component.blockedReason
                val page = setupPage(setupJson, component.name)
                Column {
                SetupRow(
                    title = sentenceCase(component.name),
                    status = blocked?.let { "Not while $it" }
                        ?: attentionAction(component.className),
                    state = if (blocked != null) SetupState.Unavailable else SetupState.NeedsAttention,
                    onClick = if (headCanOpen(page, component.name)) {
                        { openFromList(component, null) }
                    } else {
                        null
                    },
                    selected = component == openComponent,
                    summary = summaries[component.name].orEmpty(),
                    icon = setupIcon(component.known, component.className),
                    subtitle = setupNote(component.className).takeIf { summaries[component.name].isNullOrEmpty() }.orEmpty(),
                )
                SectionHits(sectionHits[component.name].orEmpty()) { openFromList(component, it) }
                }
            }
        }

        val remaining = remainingSetup(components).filter(searchHit)
        if (components.isEmpty()) {
            item(key = "empty") {
                parametersIncomplete(setupJson)?.let { SetupNotice(it) } ?: EmptyState(R.drawable.ic_build, NOTHING_TO_CONFIGURE, NOTHING_TO_CONFIGURE_TEXT)
            }
        } else if (remaining.isNotEmpty()) {
            item(key = "allheader") { SectionHeader("Ready") }
            items(remaining, key = { it.index }) { component ->
                val page = setupPage(setupJson, component.name)
                val openable = headCanOpen(page, headPage(component))
                val blocked = component.blockedReason
                Column {
                SetupRow(
                    title = sentenceCase(component.name),
                    status = when {
                        blocked != null -> "Not while $blocked"
                        component.needsAttention -> attentionAction(component.className)
                        !openable -> "On desktop"
                        else -> ""
                    },
                    state = when {
                        blocked != null -> SetupState.Unavailable
                        component.needsAttention -> SetupState.NeedsAttention
                        !openable -> SetupState.Unavailable
                        else -> SetupState.Neutral
                    },
                    onClick = if (openable) {
                        { openFromList(component, null) }
                    } else {
                        null
                    },
                    selected = component == openComponent,
                    summary = summaries[component.name].orEmpty(),
                    icon = setupIcon(component.known, component.className),
                    subtitle = setupNote(component.className).takeIf { summaries[component.name].isNullOrEmpty() }.orEmpty(),
                )
                SectionHits(sectionHits[component.name].orEmpty()) { openFromList(component, it) }
                }
            }
        }

        if (advanced && setupMatches("Parameters", setupSearch)) item(key = "parameters") {
            SectionHeader("Advanced")
            SetupRow(
                title = "Parameters",
                subtitle = parameterCountText(parameterCount) ?: "Every setting the vehicle has",
                state = SetupState.Neutral,
                icon = R.drawable.ic_tune,
                onClick = {
                    parametersSearch = ""
                    opened = null
                    parametersOpen = true
                },
                selected = parametersOpen,
            )
        }

    } }

    val shownDetail = parametersPage ?: componentPage
    val latestDetail by androidx.compose.runtime.rememberUpdatedState(shownDetail)
    val detailKey = if (parametersOpen) "parameters" else openComponent?.name
    val movableDetail = remember(detailKey, shownDetail != null) {
        shownDetail?.let { androidx.compose.runtime.movableContentOf { pane: Modifier -> latestDetail?.invoke(pane) } }
    }
    BoxWithConstraints(modifier.fillMaxSize()) {
        val detail = movableDetail
        if (maxWidth >= LIST_DETAIL_MIN_WIDTH) {
            LaunchedEffect(components, navigation.setupPage) {
                if (opened == null && !parametersOpen && navigation.setupPage == null) {
                    components.firstOrNull { headCanOpen(setupPage(setupJson, it.name), it.name) }?.let { opened = it to null }
                }
            }
            Row(Modifier.fillMaxSize()) {
                overview(Modifier.width(LIST_PANE_WIDTH).background(MaterialTheme.colorScheme.surfaceContainerLow))
                Box(Modifier.weight(1f).fillMaxHeight()) {
                    CompositionLocalProvider(LocalTwoPane provides true) {
                        detail?.invoke(Modifier.widthIn(max = DETAIL_PANE_MAX_WIDTH)) ?: EmptyState(R.drawable.ic_build, "Vehicle setup", "Choose a component on the left.")
                    }
                }
            }
        } else {
            (detail ?: overview)(Modifier)
        }
    }
}

@Composable
private fun ReadinessHeader(
    vehicle: String,
    firmware: String,
    vehicleId: Int?,
    note: String?,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .padding(top = 20.dp, bottom = 8.dp),
    ) {
        Text("Vehicle setup", style = MaterialTheme.typography.headlineMedium)
        Text(
            text = setupSubtitle(vehicle, firmware),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        vehicleId?.let {
            Text("Vehicle $it", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        note?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

