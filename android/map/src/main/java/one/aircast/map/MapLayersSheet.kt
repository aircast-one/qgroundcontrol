package one.aircast.map

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import androidx.compose.foundation.clickable
import androidx.compose.material3.Switch
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun MapLayersSheet(onDismiss: () -> Unit) {
    var listed by remember { mutableStateOf<MapTypes?>(null) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val syntheticJson by mapPath(SYNTHETIC_VIEW)
    val offers3d = syntheticJson?.optBoolean("native") == true
    val obstacleJson by mapPath(OBSTACLE_OVERLAY)
    val obstacles = obstacleJson?.optBoolean("value") == true
    LaunchedEffect(Unit) {
        listed = withContext(Dispatchers.Default) { mapTypes(runCatching { JSONObject(QGCBridge.get(MAP_TYPES_VIEW)) }.getOrNull()) }
    }
    AircastSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 24.dp).padding(bottom = 32.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Column {
                Text("Map layers", style = MaterialTheme.typography.headlineSmall)
                Text("What the map shows while flying", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listed?.types?.forEach { type ->
                    FilterChip(
                        selected = type == listed?.current,
                        onClick = {
                            val path = listed?.path ?: return@FilterChip
                            scope.launch {
                                withContext(Dispatchers.Default) { setOk(path, settingJson(JSONObject.quote(type))) }
                                listed = listed?.copy(current = type)
                            }
                        },
                        label = { Text(type) },
                    )
                }
            }
            if (offers3d) {
                LayerSwitch(MAP_3D_TITLE, MAP_3D_DETAIL, Map3d.isShown) { Map3d.show(context, !Map3d.isShown) }
            }
            LayerSwitch("Obstacle distance", "Proximity sensor readings around the vehicle", obstacles) {
                scope.launch { withContext(Dispatchers.Default) { setOk(OBSTACLE_OVERLAY, settingJson((!obstacles).toString())) } }
            }
        }
    }
}

@Composable
private fun LayerSwitch(title: String, detail: String, checked: Boolean, onToggle: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clickable(onClick = onToggle).padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyLarge)
            Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Switch(checked = checked, onCheckedChange = null)
    }
}

private const val OBSTACLE_OVERLAY = "settings.flyViewSettings.showObstacleDistanceOverlay"
private const val SYNTHETIC_VIEW = "view.syntheticView"
private const val MAP_3D_TITLE = "3D view"
private const val MAP_3D_DETAIL = "Terrain and buildings around the aircraft, and its height above the ground"
