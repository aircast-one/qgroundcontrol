package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

private const val SENSOR_FAULT_STATE = "unhealthy"
private const val SENSORS_SETUP_PAGE = "Sensors"

internal object DeckRequest {
    var action by mutableStateOf<String?>(null)
}

internal const val ARM_REQUEST = "arm"
internal const val FORCE_ARM_REQUEST = "forceArm"

internal const val ARM_UNAVAILABLE = "Arming is not available right now."

internal fun deckRequestRefusal(offer: GuidedOffer?): String = offer?.reason?.ifBlank { null } ?: ARM_UNAVAILABLE

internal data class ArmControls(
    val sliderText: String,
    val sliderEnabled: Boolean,
    val mayBeRefused: Boolean,
    val forceLink: Boolean,
    val forceSlider: Boolean,
)

internal fun armControls(state: FlyState, forceOpen: Boolean): ArmControls = ArmControls(
    sliderText = if (state.armed) "Slide to Disarm" else "Slide to Arm",
    sliderEnabled = state.canArm,
    mayBeRefused = !state.armed && !state.nominal && state.canArm && !forceOpen,
    forceLink = !state.armed && !forceOpen && (!state.canArm || state.fault),
    forceSlider = !state.armed && forceOpen,
)

internal const val SENSOR_HEALTHY_STATE = "healthy"

internal fun shownSensors(sensors: List<SensorHealth>, showAll: Boolean): List<SensorHealth> =
    if (showAll) sensors else sensors.filter { it.state != SENSOR_HEALTHY_STATE }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun VehicleStatusSheet(onDismiss: () -> Unit) {
    val healthJson by qgcPath(SENSOR_HEALTH)
    val health = remember(healthJson) { sensorHealth(healthJson) }
    var showAll by remember { mutableStateOf(false) }
    val open: (String) -> Unit = { page ->
        AppNavigation.setupPage = page
        onDismiss()
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            StatusSummary()
            ArmSection(onDismiss)
            health?.takeIf { healthJson?.optBoolean("healthChecksSupported") != true && it.available && it.sensors.isNotEmpty() }?.let { reading ->
                Text("Sensors", style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp))
                shownSensors(reading.sensors, showAll).forEach { sensor ->
                    val fault = sensor.state == SENSOR_FAULT_STATE
                    Row(
                        Modifier.fillMaxWidth().clickable { open(SENSORS_SETUP_PAGE) }.padding(horizontal = 20.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(sensor.name, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                        Text(sensor.label, style = MaterialTheme.typography.bodySmall, color = if (fault) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                val normal = reading.sensors.count { it.state == SENSOR_HEALTHY_STATE }
                if (normal > 0) TextButton(onClick = { showAll = !showAll }, modifier = Modifier.padding(horizontal = 12.dp)) {
                    Text(if (showAll) "Show less" else "Show $normal more")
                }
            }
            ParameterForm(STATUS_SETTINGS_PAGE, Modifier.heightIn(max = 360.dp))
            listOf("Vehicle parameters" to SETUP_PARAMETERS_PAGE, "Vehicle configuration" to SETUP_OVERVIEW_PAGE).forEach { (label, page) ->
                Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                    OutlinedButton(onClick = { open(page) }) { Text("Configure") }
                }
            }
        }
    }
}

@Composable
private fun StatusSummary() {
    val stateJson by qgcPath(FLY_STATE)
    val lines = remember(stateJson) { flyState(stateJson)?.let { it.stateText to it.summaryDetail } }
    lines?.takeIf { it.second.isNotBlank() }?.let { (title, detail) ->
        Column(Modifier.padding(horizontal = 20.dp, vertical = 4.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun ArmSection(onDismiss: () -> Unit) {
    val stateJson by qgcPath(FLY_STATE)
    val state = remember(stateJson) { flyState(stateJson) }?.takeIf { it.connected } ?: return
    var forceOpen by remember { mutableStateOf(false) }
    val controls = armControls(state, forceOpen)
    val request: (String) -> Unit = { action ->
        DeckRequest.action = action
        onDismiss()
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        SlideToConfirm(controls.sliderText, destructive = state.armed, enabled = controls.sliderEnabled) { request(ARM_REQUEST) }
        if (controls.mayBeRefused) {
            Text("Arming may be refused.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (controls.forceLink) {
            TextButton(onClick = { forceOpen = true }) { Text("Force arm…") }
        }
        if (controls.forceSlider) {
            SlideToConfirm("Slide to Force Arm", destructive = true) { request(FORCE_ARM_REQUEST) }
        }
    }
}
