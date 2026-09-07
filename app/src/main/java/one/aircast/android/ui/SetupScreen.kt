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
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcString
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.Alignment

private const val PLUGIN = "vehicle.autopilotPlugin"
private const val COMPONENTS = "vehicle.autopilotPlugin.vehicleComponents"

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
    val description: String,
    val requiresSetup: Boolean,
    val setupComplete: Boolean,
) {
    val needsAttention: Boolean get() = requiresSetup && !setupComplete
}

private fun readComponents(): List<SetupComponent> {
    val count = Qgc.get(COMPONENTS).optJSONArray("value")?.length() ?: 0
    return (0 until count).mapNotNull { index ->
        val json = Qgc.get("$COMPONENTS.$index")
        val name = json.optString("name")
        if (name.isBlank()) {
            null
        } else {
            SetupComponent(
                index = index,
                name = name,
                description = json.optString("description"),
                requiresSetup = json.optBoolean("requiresSetup"),
                setupComplete = json.optBoolean("setupComplete"),
            )
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

@Composable
fun SetupScreen(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val parametersReady by qgcBool("vehicle.parameterManager.parametersReady")
    val setupComplete by qgcBool("$PLUGIN.setupComplete")
    val isPx4 by qgcBool("vehicle.px4Firmware")
    val vehicleId by qgcDouble("vehicle.id")
    val major by qgcDouble("vehicle.firmwareMajorVersion", -1.0)
    val minor by qgcDouble("vehicle.firmwareMinorVersion", 0.0)
    val patch by qgcDouble("vehicle.firmwarePatchVersion", 0.0)
    val versionType by qgcString("vehicle.firmwareVersionTypeString")
    val vehicleType by qgcString("vehicle.vehicleTypeString")
    val firmwareType by qgcString("vehicle.firmwareTypeString")
    var components by remember { mutableStateOf(emptyList<SetupComponent>()) }
    var openComponent by remember { mutableStateOf<SetupComponent?>(null) }

    BackHandler(enabled = openComponent != null) { openComponent = null }

    LaunchedEffect(hasVehicle, parametersReady, setupComplete) {
        if (!hasVehicle) {
            openComponent = null
        }
        components = if (hasVehicle && parametersReady) {
            withContext(Dispatchers.Default) { readComponents() }
        } else {
            emptyList()
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

    val open = openComponent
    if (open != null && hasNativeSetupPage(open.name, isPx4)) {
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
            val sections = setupSectionsFor(open.name, isPx4)
            when {
                open.name == SENSORS -> SensorsScreen(Modifier.weight(1f))
                open.name == RADIO -> RadioScreen(Modifier.weight(1f))
                sections == null -> RemoteSupportScreen(Modifier.weight(1f))
                else -> ParameterForm(sections, Modifier.weight(1f))
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
            ReadinessHeader(
                ready = setupComplete,
                vehicle = vehicleType.ifBlank { "Vehicle" },
                firmware = firmware,
                outstanding = needSetup.size,
            )
        }

        if (needSetup.isNotEmpty()) {
            item(key = "attention") { SectionHeader("Needs setup before flight") }
            items(needSetup, key = { "a${it.index}" }) { component ->
                SetupRow(
                    title = component.name,
                    status = "Needs setup",
                    state = SetupState.NeedsAttention,
                    onClick = if (hasNativeSetupPage(component.name, isPx4)) {
                        { openComponent = component }
                    } else {
                        null
                    },
                )
            }
        }

        if (components.isEmpty()) {
            item(key = "empty") {
                SetupNotice("This vehicle reports no setup components.")
            }
        } else {
            item(key = "allheader") { SectionHeader("Setup") }
            items(components, key = { it.index }) { component ->
                val openable = hasNativeSetupPage(component.name, isPx4)
                SetupRow(
                    title = component.name,
                    status = when {
                        component.needsAttention -> "Needs setup"
                        !openable -> "On desktop"
                        else -> ""
                    },
                    state = when {
                        component.needsAttention -> SetupState.NeedsAttention
                        !openable -> SetupState.Unavailable
                        else -> SetupState.Neutral
                    },
                    onClick = if (openable) {
                        { openComponent = component }
                    } else {
                        null
                    },
                )
            }
        }

        item(key = "footnote") {
            FootNote(
                "Radio and motor calibration stay on the desktop: they need you " +
                    "watching the aircraft while it moves. Any parameter can still be " +
                    "edited from the Params tab.",
            )
        }
    }
}

@Composable
private fun ReadinessHeader(
    ready: Boolean,
    vehicle: String,
    firmware: String,
    outstanding: Int,
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
        if (!ready && outstanding > 0) {
            Text(
                text = if (outstanding == 1) {
                    "1 item needs setup"
                } else {
                    "$outstanding items need setup"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}
