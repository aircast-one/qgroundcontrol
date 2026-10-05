package one.aircast.android.ui

import android.graphics.BitmapFactory
import android.util.Base64
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONObject

internal const val APM_SUB_FRAME_SCREEN = "apmSubFrame"
internal const val APM_SUB_FRAME_VIEW = "view.apmSubFrame"
internal const val APM_SUB_FRAME_SET = "apmSubFrame.set"
internal const val APM_SUB_FRAME_LOAD = "apmSubFrame.loadDefaults"
private const val APM_SUB_FRAME_POLL_MS = 1000L

internal data class SubFrame(val name: String, val value: Int, val hasDefaults: Boolean)

internal data class SubFrames(val frames: List<SubFrame>, val selected: Int?, val confirmFirst: Boolean, val loading: Boolean, val loadError: String)

internal fun subFrames(view: JSONObject?): SubFrames? = view?.takeIf { it.optBoolean("available") }?.let {
    val frames = it.optJSONArray("frames")
    SubFrames(
        frames = (0 until (frames?.length() ?: 0)).mapNotNull { at -> frames!!.optJSONObject(at)?.let { f -> SubFrame(f.optText("name"), f.optInt("value"), f.optBoolean("hasDefaults")) } },
        selected = if (it.isNull("selected")) null else it.optInt("selected"),
        confirmFirst = it.optBoolean("confirmFirst"),
        loading = it.optBoolean("loadingDefaults"),
        loadError = it.optText("loadError"),
    )
}

internal fun frameImage(value: Int): ImageBitmap? =
    Qgc.get("view.apmSubFrameImage($value)")?.optText("png")?.takeIf { it.isNotBlank() }?.let { png ->
        val bytes = Base64.decode(png, Base64.DEFAULT)
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()
    }

@Composable
fun ApmSubFrameScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<SubFrames?>(null) }
    var picked by remember { mutableStateOf<SubFrame?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { subFrames(Qgc.get(APM_SUB_FRAME_VIEW)) }
        delay(APM_SUB_FRAME_POLL_MS)
        revision++
    }
    fun act(path: String, frame: SubFrame) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, frame.value) } }
    }
    val state = read ?: run {
        Text("This page is for an ArduSub vehicle.", modifier.padding(16.dp))
        return
    }
    Column(modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (state.loading) Text("Loading the frame's default parameters…", style = MaterialTheme.typography.bodySmall)
        (refusal ?: state.loadError.takeIf { it.isNotBlank() })?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        LazyVerticalGrid(columns = GridCells.Adaptive(160.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.frames, key = { it.value }) { frame ->
                val image by produceState<ImageBitmap?>(null, frame.value) { value = withContext(Dispatchers.Default) { frameImage(frame.value) } }
                val chosen = frame.value == state.selected
                Column(
                    Modifier
                        .background(if (chosen) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant)
                        .clickable { if (state.confirmFirst) picked = frame else act(APM_SUB_FRAME_SET, frame) }
                        .padding(8.dp),
                ) {
                    Text(frame.name, style = MaterialTheme.typography.titleSmall)
                    image?.let { Image(it, contentDescription = frame.name, contentScale = ContentScale.Fit, modifier = Modifier.fillMaxWidth().aspectRatio(1.2f)) }
                }
            }
        }
    }
    picked?.let { frame ->
        AlertDialog(
            onDismissRequest = { picked = null },
            title = { Text("Frame selection") },
            text = { Text(if (frame.hasDefaults) "Would you like to load the default parameters for the frame?" else "Would you like to set the desired frame?") },
            confirmButton = {
                Column {
                    if (frame.hasDefaults) {
                        TextButton(onClick = {
                            picked = null
                            act(APM_SUB_FRAME_LOAD, frame)
                        }) { Text("Yes, Load default parameter set for ${frame.name}") }
                    }
                    TextButton(onClick = {
                        picked = null
                        act(APM_SUB_FRAME_SET, frame)
                    }) { Text(if (frame.hasDefaults) "No, set frame only" else "Confirm frame ${frame.name}") }
                }
            },
            dismissButton = { TextButton(onClick = { picked = null }) { Text("Close") } },
        )
    }
}
