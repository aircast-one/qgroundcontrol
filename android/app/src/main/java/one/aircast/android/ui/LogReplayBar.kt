package one.aircast.android.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val LOG_REPLAY_VIEW = "view.logReplay"
internal const val LOG_REPLAY_START = "logReplay.start"
internal const val LOG_REPLAY_TOGGLE = "logReplay.togglePlay"
internal const val LOG_REPLAY_SPEED = "logReplay.speed"
internal const val LOG_REPLAY_SEEK = "logReplay.seek"
internal const val LOG_REPLAY_CLOSE = "logReplay.close"
private const val REPLAY_CACHE = "log-replay.tlog"
private const val PLAYING_POLL_MS = 250L
private const val IDLE_POLL_MS = 1000L

internal data class LogReplay(
    val shown: Boolean,
    val loaded: Boolean,
    val playing: Boolean,
    val percent: Float,
    val playheadTime: String,
    val totalTime: String,
    val speedIndex: Int,
    val speeds: List<String>,
    val canLoad: Boolean,
    val loadRefusal: String,
    val error: String,
)

internal fun logReplay(view: JSONObject?): LogReplay? = view?.takeIf { it.optBoolean("available") }?.let {
    val speeds = it.optJSONArray("speeds")
    LogReplay(
        shown = it.optBoolean("shown"),
        loaded = it.optBoolean("loaded"),
        playing = it.optBoolean("playing"),
        percent = it.optDouble("percent", 0.0).toFloat(),
        playheadTime = it.optText("playheadTime"),
        totalTime = it.optText("totalTime"),
        speedIndex = it.optInt("speedIndex", 3),
        speeds = (0 until (speeds?.length() ?: 0)).map { at -> speeds!!.optString(at) },
        canLoad = it.optBoolean("canLoad"),
        loadRefusal = it.optText("loadRefusal"),
        error = it.optText("error"),
    )
}

@Composable
fun LogReplayBar() {
    var read by remember { mutableStateOf<LogReplay?>(null) }
    var refresh by remember { mutableStateOf(0) }
    var dragging by remember { mutableStateOf(false) }
    var dragged by remember { mutableFloatStateOf(0f) }
    var message by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    LaunchedEffect(refresh) {
        read = withContext(Dispatchers.Default) { logReplay(Qgc.get(LOG_REPLAY_VIEW)) }
        delay(if (read?.playing == true) PLAYING_POLL_MS else IDLE_POLL_MS)
        refresh++
    }

    fun act(path: String, vararg args: Any) {
        scope.launch {
            message = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            refresh++
        }
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            message = withContext(Dispatchers.IO) {
                val staged = File(context.cacheDir, REPLAY_CACHE)
                val copied = runCatching {
                    context.contentResolver.openInputStream(chosen)?.use { source -> staged.outputStream().use { source.copyTo(it) } } != null
                }.getOrDefault(false)
                if (copied) Qgc.refusalOf(LOG_REPLAY_START, staged.absolutePath) else "That file could not be read."
            }
            refresh++
        }
    }

    val replay = read ?: return
    if (!replay.shown) return
    Surface(color = MaterialTheme.colorScheme.surfaceContainer) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(enabled = replay.loaded, onClick = { act(LOG_REPLAY_TOGGLE) }) { Text(if (replay.playing) "Pause" else "Play") }
                SpeedPicker(replay) { act(LOG_REPLAY_SPEED, it) }
                Text(replay.playheadTime, style = MaterialTheme.typography.bodySmall)
                Slider(
                    value = if (dragging) dragged else replay.percent,
                    onValueChange = {
                        dragging = true
                        dragged = it
                    },
                    onValueChangeFinished = {
                        dragging = false
                        act(LOG_REPLAY_SEEK, dragged.toDouble())
                    },
                    valueRange = 0f..100f,
                    enabled = replay.loaded,
                    modifier = Modifier.weight(1f),
                )
                Text(replay.totalTime, style = MaterialTheme.typography.bodySmall)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (!replay.loaded) {
                    TextButton(onClick = { if (replay.canLoad) picker.launch(arrayOf("*/*")) else message = replay.loadRefusal }) { Text("Load Telemetry Log") }
                }
                TextButton(onClick = { act(LOG_REPLAY_CLOSE) }) { Text("Close") }
            }
        }
    }
    message?.let {
        AlertDialog(
            onDismissRequest = { message = null },
            title = { Text("Log Replay") },
            text = { Text(it) },
            confirmButton = { TextButton(onClick = { message = null }) { Text("OK") } },
        )
    }
}

@Composable
private fun SpeedPicker(replay: LogReplay, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        TextButton(onClick = { open = true }) { Text(replay.speeds.getOrElse(replay.speedIndex) { "1x" }) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            replay.speeds.forEachIndexed { index, label ->
                DropdownMenuItem(text = { Text(label) }, onClick = {
                    open = false
                    onPick(index)
                })
            }
        }
    }
}
