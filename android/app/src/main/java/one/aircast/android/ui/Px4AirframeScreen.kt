package one.aircast.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val PX4_AIRFRAME_VIEW = "view.px4Airframe"
internal const val PX4_AIRFRAME_SCREEN = "px4Airframe"
internal const val PX4_AIRFRAME_APPLY = "px4Airframe.apply"
internal const val PX4_AIRFRAME_RESET = "px4Airframe.reset"

internal data class AirframeChoice(val name: String, val autostartId: Long)
internal data class AirframeGroup(val name: String, val airframes: List<AirframeChoice>, val image: String = "")

internal fun airframeImageAsset(image: String): String? =
    image.takeIf { it.isNotBlank() }?.let { "file:///android_asset/Airframe/${it.removeSuffix(".svg")}.svg" }
internal data class Px4Airframes(
    val autostartId: Long,
    val custom: Boolean,
    val customText: String,
    val heading: String,
    val currentType: String?,
    val currentIndex: Int,
    val applyTitle: String,
    val applyText: String,
    val groups: List<AirframeGroup>,
)

internal fun px4Airframes(view: JSONObject?): Px4Airframes? =
    view?.takeIf { it.optBoolean("available") }?.let { read ->
        val types = read.optJSONArray("types")
        Px4Airframes(
            autostartId = read.optLong("autostartId"),
            custom = read.optBoolean("custom"),
            customText = read.optText("customText"),
            heading = read.optText("heading"),
            currentType = read.optText("currentType").takeIf { it.isNotEmpty() },
            currentIndex = read.optInt("currentIndex"),
            applyTitle = read.optText("applyTitle"),
            applyText = read.optText("applyText").replace("<br>", "\n"),
            groups = (0 until (types?.length() ?: 0)).mapNotNull { index ->
                types?.optJSONObject(index)?.let { type ->
                    val frames = type.optJSONArray("airframes")
                    AirframeGroup(
                        name = type.optText("name"),
                        image = type.optText("image"),
                        airframes = (0 until (frames?.length() ?: 0)).mapNotNull { at ->
                            frames?.optJSONObject(at)?.let { AirframeChoice(it.optText("name"), it.optLong("autostartId")) }
                        },
                    )
                }
            },
        )
    }

internal data class AirframeSelection(val group: String, val index: Int)

internal fun initialSelection(read: Px4Airframes): AirframeSelection? =
    read.currentType?.let { AirframeSelection(it, read.currentIndex) }

@Composable
fun Px4AirframeScreen(modifier: Modifier = Modifier) {
    val view by qgcPath(PX4_AIRFRAME_VIEW)
    val read = px4Airframes(view)
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }
    var confirming by remember { mutableStateOf(false) }
    var selection by remember(read?.autostartId) { mutableStateOf(read?.let(::initialSelection)) }

    if (read == null) {
        Text("This vehicle has no SYS_AUTOSTART and SYS_AUTOCONFIG.", modifier.padding(16.dp))
        return
    }

    fun act(path: String, vararg args: Any) {
        scope.launch { refusal = withContext(Dispatchers.IO) { Qgc.refusalOf(path, *args) } }
    }

    if (read.custom) {
        Column(modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(read.customText)
            Button(onClick = { act(PX4_AIRFRAME_RESET) }, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Reset") }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
        return
    }

    val chosen = selection?.let { picked -> read.groups.find { it.name == picked.group }?.airframes?.getOrNull(picked.index) }

    if (confirming && chosen != null) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text(read.applyTitle) },
            text = { Text(read.applyText) },
            confirmButton = { TextButton(onClick = { confirming = false; act(PX4_AIRFRAME_APPLY, chosen.autostartId) }) { Text("Apply") } },
            dismissButton = { TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
    }

    Column(modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(read.heading, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
            Button(onClick = { confirming = true }, enabled = chosen != null) { Text(read.applyTitle) }
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 16.dp)) }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(read.groups, key = { it.name }) { group ->
                val picked = selection?.group == group.name
                AirframeGroupCard(
                    group = group,
                    picked = picked,
                    index = if (picked) selection?.index ?: 0 else 0,
                    onPick = { index -> selection = AirframeSelection(group.name, index) },
                )
            }
        }
    }
}

@Composable
private fun AirframeGroupCard(group: AirframeGroup, picked: Boolean, index: Int, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Surface(
        shape = MaterialTheme.shapes.medium,
        border = BorderStroke(1.dp, if (picked) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant),
        color = if (picked) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surface,
        modifier = Modifier.fillMaxWidth().clickable { onPick(index) },
    ) {
        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            RadioButton(selected = picked, onClick = { onPick(index) })
            Column(Modifier.weight(1f)) {
                Text(group.name, style = MaterialTheme.typography.titleSmall)
                airframeImageAsset(group.image)?.let { asset ->
                    coil3.compose.AsyncImage(
                        model = asset,
                        imageLoader = IconLoader.of(androidx.compose.ui.platform.LocalContext.current),
                        contentDescription = group.name,
                        contentScale = androidx.compose.ui.layout.ContentScale.Fit,
                        modifier = Modifier.fillMaxWidth().height(96.dp).padding(vertical = 4.dp),
                    )
                }
                Box {
                    OutlinedButton(onClick = { open = true }) { Text(group.airframes.getOrNull(index)?.name.orEmpty()) }
                    DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                        group.airframes.forEachIndexed { at, frame ->
                            DropdownMenuItem(text = { Text(frame.name) }, onClick = { open = false; onPick(at) })
                        }
                    }
                }
            }
        }
    }
}
