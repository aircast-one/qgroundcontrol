package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp

internal val MOCK_VEHICLES = listOf(
    "px4" to "PX4 Vehicle",
    "apmCopter" to "APM ArduCopter Vehicle",
    "apmPlane" to "APM ArduPlane Vehicle",
    "apmSub" to "APM ArduSub Vehicle",
    "apmRover" to "APM ArduRover Vehicle",
    "generic" to "Generic Vehicle",
)

internal val MOCK_VIDEO_STREAMS = listOf("Disabled", "RTP/UDP H.264", "RTP/UDP H.265", "RTSP (H.264)", "MPEG-TS (UDP)", "MPEG-TS (TCP)")

internal data class MockLinkChoices(
    val sendStatusText: Boolean = false,
    val camera: Boolean = false,
    val gimbal: Boolean = false,
    val proximity: Boolean = false,
    val freshParams: Boolean = false,
    val vehicle: Int = 0,
    val videoStream: Int = 0,
)

internal fun mockVehicleIsApm(choices: MockLinkChoices): Boolean = MOCK_VEHICLES[choices.vehicle].first.startsWith("apm")

internal fun mockLinkArguments(choices: MockLinkChoices): List<Any> = listOf(
    MOCK_VEHICLES[choices.vehicle].first,
    choices.sendStatusText,
    choices.camera,
    choices.gimbal,
    choices.proximity,
    choices.freshParams && mockVehicleIsApm(choices),
    choices.videoStream,
)

internal fun withMockVehicle(choices: MockLinkChoices, vehicle: Int): MockLinkChoices =
    choices.copy(vehicle = vehicle).let { picked -> if (mockVehicleIsApm(picked)) picked else picked.copy(freshParams = false) }

@Composable
private fun MockCheck(text: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth().toggleable(value = checked, role = Role.Checkbox, onValueChange = onChecked), verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, onCheckedChange = null, modifier = Modifier.padding(12.dp))
        Text(text, style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
internal fun MockLinkFields(choices: MockLinkChoices, onChange: (MockLinkChoices) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        ChoiceField("Vehicle Type", MOCK_VEHICLES[choices.vehicle].second, MOCK_VEHICLES.map { it.second }, Modifier.fillMaxWidth()) { onChange(withMockVehicle(choices, it)) }
        MockCheck("Send status text + voice", choices.sendStatusText) { onChange(choices.copy(sendStatusText = it)) }
        MockCheck("Enable camera", choices.camera) { onChange(choices.copy(camera = it)) }
        MockCheck("Enable gimbal", choices.gimbal) { onChange(choices.copy(gimbal = it)) }
        MockCheck("Enable proximity sensors", choices.proximity) { onChange(choices.copy(proximity = it)) }
        if (mockVehicleIsApm(choices)) MockCheck("Start with fresh firmware parameters (setup required)", choices.freshParams) { onChange(choices.copy(freshParams = it)) }
        if (choices.camera) {
            ChoiceField("Served Video Stream", MOCK_VIDEO_STREAMS[choices.videoStream], MOCK_VIDEO_STREAMS, Modifier.fillMaxWidth().padding(top = 8.dp)) { onChange(choices.copy(videoStream = it)) }
        }
    }
}
