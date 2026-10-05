package one.aircast.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal const val APM_AIRFRAME_SCREEN = "apmAirframe"
internal const val APM_AIRFRAME_VIEW = "view.apmAirframe"
internal const val APM_AIRFRAME_PICK_CLASS = "apmAirframe.pickClass"
internal const val APM_AIRFRAME_PICK_TYPE = "apmAirframe.pickType"
private const val APM_AIRFRAME_POLL_MS = 1000L
private const val AIRFRAME_IMAGES = "Airframe"

internal data class FrameTypeChoice(val name: String, val value: Int)

internal data class FrameClassCard(val name: String, val value: Int, val chosen: Boolean, val image: String, val types: List<FrameTypeChoice>, val valid: Boolean)

internal data class ApmAirframe(val help: String, val frameType: Int?, val invalidText: String, val classes: List<FrameClassCard>)

private fun <T> JSONArray?.objects(read: (JSONObject) -> T): List<T> =
    (0 until (this?.length() ?: 0)).mapNotNull { at -> this?.optJSONObject(at)?.let(read) }

internal fun apmAirframe(view: JSONObject?): ApmAirframe? = view?.takeIf { it.optBoolean("available") }?.let {
    ApmAirframe(
        help = it.optText("help"),
        frameType = if (it.isNull("frameType")) null else it.optInt("frameType"),
        invalidText = it.optText("invalidText"),
        classes = it.optJSONArray("classes").objects { c ->
            FrameClassCard(
                name = c.optText("name"),
                value = c.optInt("value"),
                chosen = c.optBoolean("chosen"),
                image = c.optText("image"),
                types = c.optJSONArray("types").objects { t -> FrameTypeChoice(t.optText("name"), t.optInt("value")) },
                valid = c.optBoolean("valid", true),
            )
        },
    )
}

@Composable
fun ApmAirframeScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<ApmAirframe?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { apmAirframe(Qgc.get(APM_AIRFRAME_VIEW)) }
        delay(APM_AIRFRAME_POLL_MS)
        revision++
    }
    fun act(path: String, value: Int) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, value) }
            read = withContext(Dispatchers.Default) { apmAirframe(Qgc.get(APM_AIRFRAME_VIEW)) }
        }
    }
    val state = read ?: run {
        Text("This page is for an ArduPilot vehicle with a FRAME_CLASS parameter.", modifier.padding(16.dp))
        return
    }
    val chosen = state.classes.firstOrNull { it.chosen && it.valid && it.types.isNotEmpty() }
    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Surface(color = MaterialTheme.colorScheme.surfaceContainerHigh, shape = MaterialTheme.shapes.large, modifier = Modifier.fillMaxWidth()) {
            Text(state.help, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(16.dp))
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        chosen?.let { card -> FrameTypeChips(card.types, state.frameType) { act(APM_AIRFRAME_PICK_TYPE, it) } }
        LazyVerticalGrid(columns = GridCells.Adaptive(FRAME_TILE_MIN), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.classes, key = { it.value }) { card ->
                FrameClassTile(card, state.invalidText) { act(APM_AIRFRAME_PICK_CLASS, card.value) }
            }
        }
    }
}

private val FRAME_TILE_MIN = 104.dp

@Composable
private fun FrameClassTile(card: FrameClassCard, invalidText: String, onPickClass: () -> Unit) {
    Surface(
        onClick = onPickClass,
        enabled = !card.chosen || !card.valid,
        shape = MaterialTheme.shapes.large,
        color = if (card.chosen) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerHigh,
        border = if (card.chosen) BorderStroke(2.dp, MaterialTheme.colorScheme.primary) else null,
    ) {
        Box {
            Column(Modifier.padding(8.dp).alpha(if (card.valid) 1f else 0.5f), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                AsyncImage(
                    model = "file:///android_asset/$AIRFRAME_IMAGES/${card.image}",
                    imageLoader = IconLoader.of(LocalContext.current),
                    contentDescription = card.name,
                    contentScale = ContentScale.Fit,
                    modifier = Modifier.fillMaxWidth().aspectRatio(1f),
                )
                Text(card.name, style = MaterialTheme.typography.labelLarge, maxLines = 1)
            }
            if (!card.valid) Text(invalidText, Modifier.align(Alignment.Center).padding(4.dp), color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.labelSmall)
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun FrameTypeChips(types: List<FrameTypeChoice>, frameType: Int?, onPick: (Int) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Frame type", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            types.forEach { type ->
                FilterChip(selected = type.value == frameType, onClick = { if (type.value != frameType) onPick(type.value) }, label = { Text(type.name) })
            }
        }
    }
}
