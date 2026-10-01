package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
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
    Column(modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(state.help, fontWeight = FontWeight.Bold)
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        LazyVerticalGrid(columns = GridCells.Adaptive(160.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.classes, key = { it.value }) { card ->
                FrameClassTile(card, state.frameType, state.invalidText, onPickClass = { act(APM_AIRFRAME_PICK_CLASS, card.value) }, onPickType = { act(APM_AIRFRAME_PICK_TYPE, it) })
            }
        }
    }
}

@Composable
private fun FrameClassTile(card: FrameClassCard, frameType: Int?, invalidText: String, onPickClass: () -> Unit, onPickType: (Int) -> Unit) {
    Column(Modifier.clickable(enabled = !card.chosen || !card.valid, onClick = onPickClass)) {
        Text(card.name, style = MaterialTheme.typography.titleSmall)
        Box(
            Modifier
                .fillMaxWidth()
                .background(if (card.chosen) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant)
                .padding(8.dp),
        ) {
            Column(Modifier.alpha(if (card.valid) 1f else 0.5f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                AsyncImage(
                    model = "file:///android_asset/$AIRFRAME_IMAGES/${card.image}",
                    imageLoader = IconLoader.of(LocalContext.current),
                    contentDescription = card.name,
                    contentScale = ContentScale.Fit,
                    modifier = Modifier.fillMaxWidth().aspectRatio(1.2f),
                )
                if (card.chosen && card.types.isNotEmpty() && card.valid) FrameTypePicker(card.types, frameType, onPickType)
            }
            if (!card.valid) Text(invalidText, Modifier.align(Alignment.Center), color = MaterialTheme.colorScheme.error)
        }
    }
}

@Composable
private fun FrameTypePicker(types: List<FrameTypeChoice>, frameType: Int?, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Text("Frame Type", style = MaterialTheme.typography.labelSmall)
    Box {
        OutlinedButton(onClick = { open = true }, modifier = Modifier.fillMaxWidth()) { Text(types.find { it.value == frameType }?.name.orEmpty()) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            types.forEach { type -> DropdownMenuItem(text = { Text(type.name) }, onClick = { open = false; onPick(type.value) }) }
        }
    }
}
