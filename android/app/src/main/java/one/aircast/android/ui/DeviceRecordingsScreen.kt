package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceRecordingsScreen() {
    val view = deviceView("recordings", ::parseRecordings) ?: return FootNote("Waiting for the device to answer…")
    DeviceSection("Storage") {
        key(view.limit.value) { LimitPicker(view.limit) }
        StatusLine("Recordings share the card this device boots from. Past the limit the oldest go first, so recording can never fill the card.")
    }
    if (view.cameras.isNotEmpty()) {
        DeviceSection("Cameras") {
            view.cameras.map { camera ->
                key(camera.name) {
                    DeviceSwitchRow(
                        title = camera.name,
                        checked = camera.recording,
                        on = DeviceRequest(DeviceMethod.PUT, recordPath(camera.name)),
                        off = DeviceRequest(DeviceMethod.DELETE, recordPath(camera.name)),
                        detail = camera.note,
                        success = "Recording turned on for ${camera.name}.",
                        offSuccess = "Recording turned off for ${camera.name} — its recordings stay below.",
                    )
                }
            }
        }
    }
    if (view.recorded.isNotEmpty()) {
        view.recorded.map { camera -> key(camera.name) { RecordedSection(camera) } }
    } else {
        DeviceSection("On this device") { DeviceText("Nothing recorded yet. Turn recording on for a camera above.") }
    }
}

@Composable
private fun LimitPicker(limit: RecordingsLimit) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var picked by remember { mutableStateOf(limit.value) }
    var lowering by remember { mutableStateOf<String?>(null) }
    val save = { value: String ->
        picked = value
        scope.launch { if (!action.run(DeviceRequest(DeviceMethod.PUT, "/api/recordings/limits", JSONObject(mapOf("maxBytes" to value))), "Recording limit saved.")) picked = limit.value }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        DeviceSelect("Recordings may use", picked, limit.options, { value -> if (lowersLimit(limit.value, value)) lowering = value else save(value) }, Modifier.widthIn(max = 320.dp), enabled = !action.busy)
        if (limit.used.isNotEmpty()) Text(limit.used, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    RefusalText(action.refusal, action.refusal?.field.orEmpty())
    lowering?.let { value ->
        ConfirmDialog(ActionConfirm("Lower the limit?", "Lowering the limit deletes the oldest recordings straight away.", "Lower it"), { save(value) }) { lowering = null }
    }
}

@Composable
private fun RecordedSection(camera: RecordedCamera) {
    DeviceSection(camera.name, camera.status) {
        if (camera.full) DeviceNotice(DeviceBadge(DeviceLevel.Warn, "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones."))
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("${camera.summary} · ${camera.usage}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            ActionButton(
                label = "Delete all",
                request = DeviceRequest(DeviceMethod.DELETE, recordingsPath(camera.name)),
                success = "Deleted every recording of ${camera.name}.",
                confirm = ActionConfirm("Delete every recording?", "Delete every recording of ${camera.name}? This cannot be undone.", "Delete all"),
                tonal = false,
            )
        }
        camera.segments.map { segment -> key(segment.at) { SegmentRow(camera.name, segment) } }
        StatusLine(
            "Each download carries the aircraft's height, speed, battery and position as a subtitle track. Players keep subtitles off until asked — in QuickTime choose View → Subtitles, " +
                "in VLC Subtitle → Sub Track.",
        )
    }
}

@Composable
private fun SegmentRow(camera: String, segment: RecordingSegment) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var asking by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Column(Modifier.weight(1f)) {
            Text(segment.started, style = MaterialTheme.typography.bodyLarge)
            Text(segmentDetail(segment).joinToString(" · "), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (!segment.open) LinkButton(deviceUrl(segment.url), "Download", look = LinkLook.Text, icon = R.drawable.ic_download)
        IconButton(onClick = { asking = true }, enabled = !action.busy) { Icon(painterResource(R.drawable.ic_delete), "Delete the recording from ${segment.started}") }
    }
    RefusalText(action.refusal, action.refusal?.field.orEmpty())
    if (asking) {
        ConfirmDialog(ActionConfirm("Delete this recording?", "Delete the recording from ${segment.started}?", "Delete"), { scope.launch { action.run(DeviceRequest(DeviceMethod.DELETE, segmentPath(camera, segment)), "Recording deleted.") } }) {
            asking = false
        }
    }
}
