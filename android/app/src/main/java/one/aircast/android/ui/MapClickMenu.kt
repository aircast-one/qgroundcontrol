package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import java.util.Locale

internal const val MAP_CLICK_PATH = "view.mapClick"

internal data class MapPoint(val latitude: Double, val longitude: Double)

internal data class MapClickAction(
    val path: String,
    val label: String,
    val title: String,
    val message: String,
    val confirm: Boolean,
)

internal fun mapClickActions(view: JSONObject?): List<MapClickAction> {
    val listed = view?.optJSONArray("actions") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let {
            MapClickAction(
                path = it.optText("path"),
                label = it.optText("label"),
                title = it.optText("title"),
                message = it.optText("message"),
                confirm = it.optBoolean("confirm", true),
            )
        }
    }
}

internal fun coordinateLines(point: MapPoint): List<String> = listOf(
    String.format(Locale.US, "Lat: %.6f", point.latitude),
    String.format(Locale.US, "Lon: %.6f", point.longitude),
)

private fun send(action: MapClickAction, point: MapPoint): String? =
    Qgc.refusalOf(action.path, JSONObject().put("latitude", point.latitude).put("longitude", point.longitude))

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MapClickMenu(point: MapPoint, onDismiss: () -> Unit) {
    val view by qgcPath(MAP_CLICK_PATH)
    val actions = remember(view) { mapClickActions(view) }
    var confirming by remember(point) { mutableStateOf<MapClickAction?>(null) }
    var refusal by remember(point) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun run(action: MapClickAction) {
        scope.launch {
            val refused = withContext(Dispatchers.Default) { send(action, point) }
            if (refused == null) onDismiss() else refusal = refused
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            val pending = confirming
            if (pending == null) {
                if (actions.isEmpty()) {
                    Text(
                        "No action is available at this point.",
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 12.dp),
                    )
                }
                actions.forEach { action ->
                    ListItem(
                        headlineContent = { Text(action.label) },
                        modifier = Modifier.clickable {
                            refusal = null
                            if (action.confirm) confirming = action else run(action)
                        },
                    )
                }
            } else {
                Column(
                    Modifier.padding(horizontal = 20.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(pending.title, style = MaterialTheme.typography.titleMedium)
                    Text(pending.message, style = MaterialTheme.typography.bodyMedium)
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        SlideToConfirm(label = "Slide to confirm", modifier = Modifier.weight(1f)) { run(pending) }
                        TextButton(onClick = { confirming = null }) { Text("Cancel") }
                    }
                }
            }
            refusal?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                )
            }
            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            coordinateLines(point).forEach {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                )
            }
        }
    }
}
