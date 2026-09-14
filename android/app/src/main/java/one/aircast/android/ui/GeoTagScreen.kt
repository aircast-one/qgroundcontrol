package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcString
import java.io.File

private fun telemetryLogs(savePath: String): List<TelemetryLog> =
    File(savePath, "Telemetry")
        .listFiles { file -> file.isFile && file.name.endsWith(".tlog") }
        .orEmpty()
        .sortedByDescending { it.lastModified() }
        .map { TelemetryLog(path = it.absolutePath, name = it.name, bytes = it.length()) }

@Composable
fun GeoTagScreen(modifier: Modifier = Modifier) {
    val savePath by qgcString("settings.appSettings.savePath")
    var logs by remember { mutableStateOf<List<TelemetryLog>>(emptyList()) }
    var readings by remember { mutableStateOf<Map<String, GeoTagReading>>(emptyMap()) }

    LaunchedEffect(savePath) {
        if (savePath.isBlank()) return@LaunchedEffect
        val found = withContext(Dispatchers.IO) { telemetryLogs(savePath) }
        logs = found
        readings = withContext(Dispatchers.Default) {
            found.mapNotNull { log ->
                geoTagPath(log.path)
                    ?.let { path -> geoTagReading(Qgc.get(path)) }
                    ?.let { log.path to it }
            }.toMap()
        }
    }

    if (savePath.isNotBlank() && logs.isEmpty()) {
        Text(
            text = "No telemetry logs on this device yet. Turn on \"Save telemetry Log after " +
                "each flight\" under MAVLink and telemetry logs, then fly.",
            style = MaterialTheme.typography.bodyLarge,
            modifier = modifier.fillMaxWidth().padding(20.dp),
        )
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        item(key = "note") {
            FootNote(
                "A telemetry log records where the vehicle was when each photograph was " +
                    "taken. Matching them to the photographs themselves is the next step and " +
                    "is not here yet.",
            )
        }
        items(logs.size, key = { logs[it].path }) { index ->
            val log = logs[index]
            val reading = readings[log.path]
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                Text(log.name, style = MaterialTheme.typography.bodyLarge)
                Text(
                    text = commaRefusal(log)
                        ?: (logSize(log.bytes) + (reading?.let { " · " + triggerSummary(it) } ?: "")),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
