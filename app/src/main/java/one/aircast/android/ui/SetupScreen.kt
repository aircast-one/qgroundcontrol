package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.Alignment
import one.aircast.mapspike.optText

private const val PLUGIN = "vehicle.autopilotPlugin"

internal fun firmwareSummary(
    firmwareType: String,
    major: Int,
    minor: Int,
    patch: Int,
    versionType: String,
): String {
    val version = if (major < 0) "" else "$major.$minor.$patch"
    val suffix = versionType.takeIf { it.isNotBlank() && !it.equals("Official", true) }
    return listOfNotNull(
        firmwareType.takeIf { it.isNotBlank() },
        version.takeIf { it.isNotEmpty() },
        suffix,
    ).joinToString(" ")
}

internal data class SetupComponent(
    val index: Int,
    val name: String,
    val needsAttention: Boolean,
    val blockedReason: String? = null,
)

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
            needsAttention = element.optBoolean("needsAttention"),
            blockedReason = element.optText("blockedReason")
                .takeIf { !element.isNull("blockedReason") && it.isNotBlank() },
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
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val parametersReady by qgcBool("vehicle.parameterManager.parametersReady")
    val setupComplete by qgcBool("$PLUGIN.setupComplete")
    val isPx4 by qgcBool("vehicle.px4Firmware")
    val setupJson by qgcPath(SETUP)
    val vehicleId by qgcDouble("vehicle.id")
    val major by qgcDouble("vehicle.firmwareMajorVersion", -1.0)
    val minor by qgcDouble("vehicle.firmwareMinorVersion", 0.0)
    val patch by qgcDouble("vehicle.firmwarePatchVersion", 0.0)
    val versionType by qgcString("vehicle.firmwareVersionTypeString")
    val vehicleType by qgcString("vehicle.vehicleTypeString")
    val firmwareType by qgcString("vehicle.firmwareTypeString")
    var openComponent by remember { mutableStateOf<SetupComponent?>(null) }
    var parametersOpen by remember { mutableStateOf(false) }

    BackHandler(enabled = openComponent != null) { openComponent = null }

    val components = remember(setupJson) { setupComponents(setupJson) }

    LaunchedEffect(hasVehicle) {
        if (!hasVehicle) {
            openComponent = null
        }
    }

    if (!hasVehicle) {
        SetupNotice("Connect a vehicle to set it up.", modifier)
        return
    }

    if (!parametersReady) {
        SetupNotice("Loading parameters from the vehicle.", modifier)
        return
    }

    if (parametersOpen) {
        Column(modifier.fillMaxSize()) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = { parametersOpen = false }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to Setup")
                }
                Text("Parameters", style = MaterialTheme.typography.titleMedium)
            }
            HorizontalDivider()
            ParametersScreen(Modifier.weight(1f))
        }
        return
    }

    val open = openComponent
    if (open != null && headCanOpen(setupPage(setupJson, open.name), open.name)) {
        Column(modifier.fillMaxSize()) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = { openComponent = null }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to Setup")
                }
                Text(open.name, style = MaterialTheme.typography.titleMedium)
            }
            HorizontalDivider()
            val nativePage = setupPage(setupJson, open.name)
            val blocked = open.blockedReason
            when {
                blocked != null -> SetupNotice(
                    "${open.name} cannot be set up while the vehicle is $blocked.",
                    Modifier.weight(1f),
                )
                open.name == SENSORS -> SensorsScreen(Modifier.weight(1f))
                open.name == RADIO -> RadioScreen(Modifier.weight(1f))
                open.name == REMOTE_SUPPORT -> RemoteSupportScreen(Modifier.weight(1f))
                open.name == MOTORS -> MotorsScreen(Modifier.weight(1f))
                open.name == FLIGHT_MODES_PAGE -> FlightModesSetup(Modifier.weight(1f))
                nativePage?.parameterSections == true -> ParameterForm(open.name, Modifier.weight(1f))
                else -> SetupNotice(
                    "${open.name} is set up on the desktop.",
                    Modifier.weight(1f),
                )
            }
        }
        return
    }

    val firmware = firmwareSummary(
        firmwareType,
        major.toInt(),
        minor.toInt(),
        patch.toInt(),
        versionType,
    )
    val needSetup = components.filter { it.needsAttention }

    LazyColumn(modifier.fillMaxSize()) {
        item(key = "verdict") {
            val readiness = setupReadiness(setupJson)
            ReadinessHeader(
                ready = readiness?.ready ?: setupComplete,
                vehicle = vehicleType.ifBlank { "Vehicle" },
                firmware = firmware,
                headline = readiness?.headline.orEmpty(),
                detail = readiness?.detail.orEmpty(),
            )
        }

        if (needSetup.isNotEmpty()) {
            item(key = "attention") { SectionHeader("Needs setup before flight") }
            items(needSetup, key = { "a${it.index}" }) { component ->
                val blocked = component.blockedReason
                val page = setupPage(setupJson, component.name)
                SetupRow(
                    title = component.name,
                    status = blocked?.let { "Not while $it" }
                        ?: setupBadge(component.name, headCanOpen(page, component.name)),
                    state = if (blocked != null) SetupState.Unavailable else SetupState.NeedsAttention,
                    onClick = if (blocked == null && headCanOpen(page, component.name)) {
                        { openComponent = component }
                    } else {
                        null
                    },
                )
            }
        }

        val remaining = remainingSetup(components)
        if (components.isEmpty()) {
            item(key = "empty") {
                SetupNotice("This vehicle reports no setup components.")
            }
        } else if (remaining.isNotEmpty()) {
            item(key = "allheader") { SectionHeader("Setup") }
            items(remaining, key = { it.index }) { component ->
                val page = setupPage(setupJson, component.name)
                val openable = headCanOpen(page, component.name)
                val blocked = component.blockedReason
                SetupRow(
                    title = component.name,
                    status = when {
                        blocked != null -> "Not while $blocked"
                        component.needsAttention -> setupBadge(component.name, openable)
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
                )
            }
        }

        item(key = "parameters") {
            SectionHeader("Everything else")
            SetupRow(
                title = "Parameters",
                status = "Every setting the vehicle has",
                state = SetupState.Neutral,
                onClick = { parametersOpen = true },
            )
        }

        item(key = "footnote") {
            FootNote(
                "Radio calibration stays on the desktop: it needs you holding each " +
                    "stick at its extremes while watching the aircraft. Any parameter can " +
                    "still be edited under Parameters above.",
            )
        }
    }
}

@Composable
private fun ReadinessHeader(
    ready: Boolean,
    vehicle: String,
    firmware: String,
    headline: String,
    detail: String,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp)
            .padding(top = 20.dp, bottom = 8.dp),
    ) {
        Text(
            text = if (ready) "Ready to fly" else "Not ready to fly",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Bold,
            color = if (ready) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.error
            },
        )
        Text(
            text = listOfNotNull(
                vehicle,
                firmware.takeIf { it.isNotBlank() },
            ).joinToString(" · "),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (!ready && headline.isNotBlank()) {
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

