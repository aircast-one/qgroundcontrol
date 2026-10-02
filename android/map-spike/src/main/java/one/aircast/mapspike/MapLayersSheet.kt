package one.aircast.mapspike

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun MapLayersSheet(onDismiss: () -> Unit) {
    var listed by remember { mutableStateOf<MapTypes?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) {
        listed = withContext(Dispatchers.Default) { mapTypes(runCatching { JSONObject(QGCBridge.get(MAP_TYPES_VIEW)) }.getOrNull()) }
    }
    ModalBottomSheet(onDismissRequest = onDismiss) {
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
        }
    }
}
