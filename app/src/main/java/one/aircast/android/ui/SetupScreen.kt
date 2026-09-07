package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
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
            if (sections == null) {
                RemoteSupportScreen(Modifier.weight(1f))
            } else {
                ParameterForm(sections, Modifier.weight(1f))
            }
        }
        return
    }

    Column(modifier.fillMaxSize()) {
        val firmware = firmwareSummary(
            firmwareType,
            major.toInt(),
            minor.toInt(),
            patch.toInt(),
            versionType,
        )
        val needSetup = components.filter { it.needsAttention }

        ListItem(
            headlineContent = { Text(vehicleType.ifBlank { "Vehicle" }) },
            supportingContent = {
                Text(
                    listOfNotNull(
                        firmware.takeIf { it.isNotBlank() },
                        if (vehicleId > 0) "MAV ${vehicleId.toInt()}" else null,
                        if (setupComplete) "ready to fly" else "needs setup",
                    ).joinToString(" · "),
                )
            },
        )
        HorizontalDivider()

        if (needSetup.isNotEmpty()) {
            Text(
                text = "Needs setup before flight",
                style = MaterialTheme.typography.titleSmall,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
            )
            needSetup.forEach { component ->
                ListItem(
                    headlineContent = { Text(component.name) },
                    supportingContent = { Text(component.description) },
                    trailingContent = {
                        Text(
                            text = "Needs setup",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.error,
                        )
                    },
                )
                HorizontalDivider()
            }
        }

        if (components.isEmpty()) {
            SetupNotice("This vehicle reports no setup components.")
            return@Column
        }

        Text(
            text = "Pages marked Open work here. Radio and sensor calibration and the " +
                "motor test are not carried over yet, so use QGroundControl on a " +
                "computer for those. Any parameter can still be edited from the " +
                "Params tab.",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )

        LazyColumn(Modifier.fillMaxSize()) {
            items(components, key = { it.index }) { component ->
                val openable = hasNativeSetupPage(component.name, isPx4)
                ListItem(
                    modifier = if (!openable) {
                        Modifier
                    } else {
                        Modifier.clickable { openComponent = component }
                    },
                    headlineContent = { Text(component.name) },
                    supportingContent = { Text(component.description) },
                    trailingContent = {
                        Text(
                            text = when {
                                openable -> "Open"
                                !component.requiresSetup -> ""
                                component.setupComplete -> "Ready"
                                else -> "Needs setup"
                            },
                            style = MaterialTheme.typography.labelMedium,
                            color = if (component.needsAttention) {
                                MaterialTheme.colorScheme.error
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            },
                        )
                    },
                )
                HorizontalDivider()
            }
        }
    }
}
