package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder

internal const val START_MOCK_LINK = "links.startMockLink"
internal const val STOP_ONE_MOCK_LINK = "links.stopOneMockLink"

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

@Composable
private fun MockCheck(text: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth().toggleable(value = checked, role = Role.Checkbox, onValueChange = onChecked), verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, onCheckedChange = null, modifier = Modifier.padding(12.dp))
        Text(text, style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
internal fun MockLinkPage(modifier: Modifier = Modifier) {
    var sendStatusText by rememberSaveable { mutableStateOf(false) }
    var camera by rememberSaveable { mutableStateOf(false) }
    var gimbal by rememberSaveable { mutableStateOf(false) }
    var proximity by rememberSaveable { mutableStateOf(false) }
    var freshParams by rememberSaveable { mutableStateOf(false) }
    var vehicle by rememberSaveable { mutableStateOf(0) }
    var videoStream by rememberSaveable { mutableStateOf(0) }
    val choices = MockLinkChoices(sendStatusText, camera, gimbal, proximity, freshParams, vehicle, videoStream)

    Column(
        modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        MockCheck("Send status text + voice", sendStatusText) { sendStatusText = it }
        MockCheck("Enable camera", camera) { camera = it }
        MockCheck("Enable gimbal", gimbal) { gimbal = it }
        MockCheck("Enable proximity sensors", proximity) { proximity = it }
        if (mockVehicleIsApm(choices)) MockCheck("Start with fresh firmware parameters (setup required)", freshParams) { freshParams = it }
        ChoiceField("Vehicle Type", MOCK_VEHICLES[vehicle].second, MOCK_VEHICLES.map { it.second }, Modifier.fillMaxWidth().padding(top = 8.dp)) { picked ->
            vehicle = picked
            if (!mockVehicleIsApm(choices.copy(vehicle = picked))) freshParams = false
        }
        if (camera) {
            ChoiceField("Served Video Stream", MOCK_VIDEO_STREAMS[videoStream], MOCK_VIDEO_STREAMS, Modifier.fillMaxWidth().padding(top = 8.dp)) { videoStream = it }
        }
        Button(onClick = { offMainInOrder { Qgc.invoke(START_MOCK_LINK, *mockLinkArguments(choices).toTypedArray()) } }, modifier = Modifier.fillMaxWidth().padding(top = 8.dp)) {
            Text("Start MockLink")
        }
        OutlinedButton(onClick = { offMainInOrder { Qgc.invoke(STOP_ONE_MOCK_LINK) } }, modifier = Modifier.fillMaxWidth()) {
            Text("Stop One MockLink")
        }
    }
}
