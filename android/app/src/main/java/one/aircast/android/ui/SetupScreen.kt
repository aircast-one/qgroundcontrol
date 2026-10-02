package one.aircast.android.ui

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
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import one.aircast.mapspike.optText
import one.aircast.mapspike.aircast
import androidx.compose.material3.OutlinedButton


internal data class FirmwareLine(val summary: String, val vehicleType: String)

// view.firmware spells the line - "PX4 1.15.0 beta" - and knows that a major version of -1 is
// Vehicle's versionNotSetValue rather than a version.
internal fun firmwareLine(view: JSONObject?): FirmwareLine =
    FirmwareLine(view?.optText("summary").orEmpty(), view?.optText("vehicleType").orEmpty())

internal data class SetupComponent(
    val index: Int,
    val name: String,
    val known: String? = null,
    val needsAttention: Boolean,
    val blockedReason: String? = null,
    val prerequisite: String? = null,
)

internal fun setupIcon(known: String?): Int = when (known) {
    "radio", "joystick" -> R.drawable.ic_gamepad
    "flightModes" -> R.drawable.ic_toggle_on
    "sensors" -> R.drawable.ic_sensors
    "safety" -> R.drawable.ic_shield
    "power" -> R.drawable.ic_bolt
    "esc" -> R.drawable.ic_tune
    else -> R.drawable.ic_build
}

internal fun prerequisiteText(first: String, wanted: String): String = "$first has to be set up before $wanted."

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
internal const val SETUP_SUMMARY = "view.setupSummary"
private const val SETUP_SUMMARY_POLL_MS = 2000L

internal fun setupSummaries(view: JSONObject?): Map<String, List<SummaryLine>> {
    val listed = view?.optJSONArray("components") ?: return emptyMap()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it) }.associate { component ->
        val rows = component.optJSONArray("rows")
        component.optText("name") to (0 until (rows?.length() ?: 0)).mapNotNull { rows?.optJSONObject(it) }.map { SummaryLine(it.optText("label"), it.optText("value")) }
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
            needsAttention = element.optBoolean("needsAttention"),
            blockedReason = element.optText("blockedReason")
                .takeIf { !element.isNull("blockedReason") && it.isNotBlank() },
            prerequisite = element.optText("prerequisite").takeIf { !element.isNull("prerequisite") && it.isNotBlank() },
        )
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

@Composable
fun SetupScreen(modifier: Modifier = Modifier) {
    val setupJson by qgcPath(SETUP)
    val setup = remember(setupJson) { setupReadiness(setupJson) }
    val hasVehicle = setup?.connected == true
    val firmwareJson by qgcPath(FIRMWARE)
    val line = remember(firmwareJson) { firmwareLine(firmwareJson) }
    val vehicleType = line.vehicleType
    var openComponent by remember { mutableStateOf<SetupComponent?>(null) }
    var parametersOpen by remember { mutableStateOf(false) }
    var setupSearch by remember { mutableStateOf("") }
    var parametersSearch by remember { mutableStateOf("") }

    BackHandler(enabled = openComponent != null) { openComponent = null }

    val components = remember(setupJson) { setupComponents(setupJson) }
    var summaries by remember { mutableStateOf(emptyMap<String, List<SummaryLine>>()) }
    val overviewShown = hasVehicle && openComponent == null && !parametersOpen
    LaunchedEffect(overviewShown, hasVehicle) {
        if (!hasVehicle) summaries = emptyMap()
        while (overviewShown) {
            summaries = withContext(Dispatchers.Default) { setupSummaries(Qgc.get(SETUP_SUMMARY)) }
            delay(SETUP_SUMMARY_POLL_MS)
        }
    }

    LaunchedEffect(AppNavigation.setupPage, components) {
        AppNavigation.setupPage?.takeIf { components.isNotEmpty() }?.let { requested ->
            when (requested) {
                SETUP_PARAMETERS_PAGE -> {
                    parametersSearch = ""
                    parametersOpen = true
                }
                else -> components.firstOrNull { it.name == requested }?.let { openComponent = it }
            }
            AppNavigation.setupPage = null
        }
    }

    LaunchedEffect(hasVehicle) {
        if (!hasVehicle) {
            openComponent = null
        }
    }

    if (!hasVehicle) {
        Column(modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            EmptyState(R.drawable.ic_build, NO_VEHICLE_HEADLINE, NO_VEHICLE_TEXT)
            OutlinedButton(onClick = { AppNavigation.settingsPage = "Connections" }) { Text("Set Up Connection") }
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
                ) { Text("Download Parameters") }
            }
        }
        return
    }

    if (parametersOpen) {
        Column(modifier.fillMaxSize()) {
            PageTopBar("Parameters", "Back to Setup") { parametersOpen = false }
            ParametersScreen(Modifier.weight(1f), initialSearch = parametersSearch)
        }
        return
    }

    val open = openComponent
    if (open != null && headCanOpen(setupPage(setupJson, open.name), open.name)) {
        Column(modifier.fillMaxSize()) {
            PageTopBar(open.name, "Back to Setup") { openComponent = null }
            val nativePage = setupPage(setupJson, open.name)
            val blocked = open.blockedReason
            val first = open.prerequisite
            when {
                blocked != null -> SetupNotice(
                    "${open.name} cannot be set up while the vehicle is $blocked.",
                    Modifier.weight(1f),
                )
                first != null -> androidx.compose.foundation.layout.Column(Modifier.weight(1f).fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                    val firstComponent = setupComponents(setupJson).firstOrNull { it.name == first }
                    EmptyState(setupIcon(firstComponent?.known), "$first first", prerequisiteText(first, open.name))
                    androidx.compose.material3.Button(onClick = {
                        openComponent = setupComponents(setupJson).firstOrNull { it.name == first }
                    }) { Text("Set Up $first") }
                }
                headPage(open) == SENSORS -> SensorsScreen(Modifier.weight(1f))
                headPage(open) == RADIO -> RadioScreen(Modifier.weight(1f))
                headPage(open) == REMOTE_SUPPORT -> RemoteSupportScreen(Modifier.weight(1f))
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
                nativePage?.screen == APM_SUB_FRAME_SCREEN -> ApmSubFrameScreen(Modifier.weight(1f))
                nativePage?.screen == APM_AIRFRAME_SCREEN -> ApmAirframeScreen(Modifier.weight(1f))
                nativePage?.screen == APM_SUB_MOTORS_SCREEN -> ApmSubMotorsScreen(Modifier.weight(1f))
                nativePage?.screen == OPTICAL_FLOW_SCREEN -> OpticalFlowScreen(Modifier.weight(1f))
                nativePage?.parameterSections == true -> {
                    if (open.known == "power") PowerLiveCard()
                    ParameterForm(open.name, Modifier.weight(1f))
                }
                else -> SetupNotice(
                    "${open.name} is set up on the desktop.",
                    Modifier.weight(1f),
                )
            }
        }
        return
    }

    val firmware = line.summary
    val needSetup = components.filter { it.needsAttention && setupMatches(it.name, setupSearch) }

    LazyColumn(modifier.fillMaxSize()) {
        item(key = "verdict") {
            val readiness = setup
            ReadinessHeader(
                ready = readiness?.setupComplete,
                vehicle = vehicleType.ifBlank { "Vehicle" },
                firmware = firmware,
                headline = readiness?.headline.orEmpty(),
                detail = readiness?.detail.orEmpty(),
            )
        }

        item(key = "search") {
            SearchPill(
                value = setupSearch,
                onValueChange = { setupSearch = it },
                placeholder = "Search",
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = {
                    if (setupSearch.isNotBlank()) {
                        parametersSearch = setupSearch.trim()
                        parametersOpen = true
                    }
                }),
            )
        }

        if (needSetup.isNotEmpty()) {
            item(key = "attention") { SectionHeader("Needs setup") }
            items(needSetup, key = { "a${it.index}" }) { component ->
                val blocked = component.blockedReason
                val page = setupPage(setupJson, component.name)
                SetupRow(
                    title = component.name,
                    status = blocked?.let { "Not while $it" }
                        ?: NEEDS_SETUP_BADGE,
                    state = if (blocked != null) SetupState.Unavailable else SetupState.NeedsAttention,
                    onClick = if (blocked == null && headCanOpen(page, component.name)) {
                        { openComponent = component }
                    } else {
                        null
                    },
                    summary = summaries[component.name].orEmpty(),
                    icon = setupIcon(component.known),
                )
            }
        }

        val remaining = remainingSetup(components).filter { setupMatches(it.name, setupSearch) }
        if (components.isEmpty()) {
            item(key = "empty") {
                parametersIncomplete(setupJson)?.let { SetupNotice(it) } ?: EmptyState(R.drawable.ic_build, NOTHING_TO_CONFIGURE, NOTHING_TO_CONFIGURE_TEXT)
            }
        } else if (remaining.isNotEmpty()) {
            item(key = "allheader") { SectionHeader("Setup") }
            items(remaining, key = { it.index }) { component ->
                val page = setupPage(setupJson, component.name)
                val openable = headCanOpen(page, headPage(component))
                val blocked = component.blockedReason
                SetupRow(
                    title = component.name,
                    status = when {
                        blocked != null -> "Not while $blocked"
                        component.needsAttention -> NEEDS_SETUP_BADGE
                        !openable -> "On desktop"
                        else -> ""
                    },
                    state = when {
                        blocked != null -> SetupState.Unavailable
                        component.needsAttention -> SetupState.NeedsAttention
                        !openable -> SetupState.Unavailable
                        else -> SetupState.Neutral
                    },
                    onClick = if (blocked == null && openable) {
                        { openComponent = component }
                    } else {
                        null
                    },
                    summary = summaries[component.name].orEmpty(),
                    icon = setupIcon(component.known),
                )
            }
        }

        if (setupMatches("Parameters", setupSearch)) item(key = "parameters") {
            SectionHeader("Advanced")
            SetupRow(
                title = "Parameters",
                status = "Every setting the vehicle has",
                state = SetupState.Neutral,
                icon = R.drawable.ic_tune,
                onClick = {
                    parametersSearch = ""
                    parametersOpen = true
                },
            )
        }

    }
}

@Composable
private fun ReadinessHeader(
    ready: Boolean?,
    vehicle: String,
    firmware: String,
    headline: String,
    detail: String,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .padding(top = 20.dp, bottom = 8.dp),
    ) {
        Text("Vehicle setup", style = MaterialTheme.typography.headlineMedium)
        ready?.let { verdict ->
            Text(
                text = setupPillText(verdict),
                style = MaterialTheme.typography.titleMedium,
                color = if (verdict) MaterialTheme.aircast.success else MaterialTheme.aircast.warning,
            )
        }
        Text(
            text = listOfNotNull(
                vehicle,
                firmware.takeIf { it.isNotBlank() },
            ).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (ready != true && headline.isNotBlank()) {
            Text(
                text = headline,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (detail.isNotBlank()) {
            Text(
                text = detail,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

