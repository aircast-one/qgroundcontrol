package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceTelemetryScreen(props: DeviceScreenProps) {
    val view = deviceView("telemetry", ::parseTelemetry) ?: return FootNote("Waiting for the device to answer…")
    DeviceSection("Ground station", view.status) {
        view.notices.map { notice -> key(notice.text) { DeviceNotice(notice) } }
        EndpointRows(view.endpoints)
        view.endpointNotice?.let { DeviceNotice(it) }
        view.cloud?.let { CloudPath(it) }
    }
    DeviceSection("MAVLink settings") {
        key(view.settings) { MavlinkEditor(view) }
    }
    view.gimbal?.let { GimbalSection(it) }
}

@Composable
private fun CloudPath(cloud: TelemetryCloud) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Cloud path", style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
            DeviceBadgeText(cloud.status)
        }
        Text(cloud.text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private fun startingRows(view: TelemetryView): List<EndpointRow> = parseEndpointSpecs(view.settings.endpoints).ifEmpty { listOf(newRow(view.schemes)) }

@Composable
private fun MavlinkEditor(view: TelemetryView) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var rows by remember { mutableStateOf(startingRows(view)) }
    var baud by remember { mutableStateOf(view.settings.baud) }
    val baudRefused = action.refusal?.field == "baud"
    val save = {
        scope.launch {
            action.run(DeviceRequest(DeviceMethod.PUT, "/api/telemetry/config", JSONObject(mapOf("endpoints" to serializeEndpoints(view.schemes, rows), "baud" to baud))), "MAVLink settings applied.")
        }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Ground-station endpoints", style = MaterialTheme.typography.bodyLarge)
        rows.mapIndexed { index, row ->
            key(index) {
                EndpointEditorRow(
                    schemes = view.schemes,
                    row = row,
                    onChange = { next -> rows = rows.mapIndexed { at, it -> if (at == index) next else it } },
                    onRemove = { rows = rows.filterIndexed { at, _ -> at != index } },
                )
            }
        }
        TextButton(onClick = { rows = rows + newRow(view.schemes) }) { Text("+ Add endpoint") }
        OutlinedTextField(
            value = baud,
            onValueChange = { baud = it },
            label = { Text("Serial baud") },
            placeholder = { Text("115200") },
            singleLine = true,
            isError = baudRefused,
            supportingText = { Text(if (baudRefused) action.refusal?.message.orEmpty() else "The baud only matters for a serial flight controller — USB autopilots ignore it, Betaflight ships at 115200.") },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth(),
        )
        Text("Takes effect on save — no reboot.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        action.refusal?.takeUnless { baudRefused }?.let { Text(it.message, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        FilledTonalButton(onClick = { save() }, enabled = !action.busy) { Text("Save") }
    }
}

@Composable
private fun EndpointEditorRow(schemes: List<MavlinkScheme>, row: EndpointRow, onChange: (EndpointRow) -> Unit, onRemove: () -> Unit) {
    val error = endpointError(schemes, row)
    Column(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            DeviceSelect("Connection type", row.scheme, schemeOptions(schemes, row), { scheme -> onChange(retargetRow(schemes, row.copy(scheme = scheme))) }, Modifier.weight(1f))
            IconButton(onClick = onRemove) { Icon(painterResource(R.drawable.ic_close), "Remove endpoint") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = row.host,
                onValueChange = { onChange(row.copy(host = it)) },
                label = { Text(hostLabel(schemes, row)) },
                placeholder = hostPlaceholder(schemes, row).takeIf { it.isNotEmpty() }?.let { { Text(it) } },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
                modifier = Modifier.weight(1f),
            )
            OutlinedTextField(
                value = row.port,
                onValueChange = { onChange(row.copy(port = it)) },
                label = { Text("Port") },
                placeholder = { Text("14550") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.width(112.dp),
            )
        }
        if (error.isNotEmpty()) {
            Text("$error It is not saved until it has one.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        } else {
            Text(describeEndpoint(schemes, row), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun GimbalSection(gimbal: TelemetryGimbal) {
    DeviceSection(gimbal.title, gimbal.status) {
        DeviceText(gimbal.about)
        key(gimbal.vendor, gimbal.model, gimbal.cameraAddr, gimbal.mavlinkAddr) {
            DeviceForm(
                fields = listOf(
                    DeviceField("vendor", "Make", gimbal.vendor, options = gimbal.vendors),
                    DeviceField("model", "Model", gimbal.model, options = gimbal.models),
                    DeviceField("cameraAddr", "Camera address", gimbal.cameraAddr, placeholder = "192.168.144.25:37260", keyboard = KeyboardType.Uri),
                    DeviceField("mavlinkAddr", "MAVLink endpoint", gimbal.mavlinkAddr, placeholder = "127.0.0.1:14550", keyboard = KeyboardType.Uri),
                ),
                method = DeviceMethod.PUT,
                path = "/api/gimbal/config",
                submit = "Save",
                success = "Gimbal settings applied.",
                note = "Takes effect on save — no reboot. Changing the make resets the model and address to that make's defaults if the old ones do not fit.",
            )
        }
    }
}
