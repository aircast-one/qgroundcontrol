package one.aircast.mapspike

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

@Composable
fun ItemCameraSection(index: Int, modifier: Modifier = Modifier, onChanged: () -> Unit = {}) {
    var revision by remember(index) { mutableIntStateOf(0) }
    var open by remember(index) { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val camera by produceState<JSONObject?>(null, index, revision) {
        value = withContext(Dispatchers.Default) { ItemCameraBridge.read(index) }
    }
    val choices = cameraChoices(camera) ?: return
    fun write(work: () -> Unit) {
        scope.launch {
            withContext(Dispatchers.Default) { work() }
            revision += 1
            onChanged()
        }
    }
    Column(modifier) {
        itemCameraTextBeside(camera, choices.labels.getOrNull(choices.chosen))?.let { CameraNote(it) }
        Box(Modifier.fillMaxWidth()) {
            OutlinedTextField(
                value = choices.labels.getOrElse(choices.chosen) { "…" },
                onValueChange = {},
                readOnly = true,
                label = { Text("Camera") },
                trailingIcon = { Icon(Icons.Default.ArrowDropDown, null) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Box(Modifier.matchParentSize().clickable { open = true })
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                choices.labels.forEachIndexed { at, label ->
                    DropdownMenuItem(
                        text = { Text(label) },
                        onClick = {
                            open = false
                            write { ItemCameraBridge.chooseAction(index, at) }
                        },
                    )
                }
            }
        }
        cameraExtras(camera)?.let { extras ->
            CameraSectionExtras(extras) { member, value -> write { ItemCameraBridge.set(index, member, value) } }
        }
        itemCameraNote(camera)?.let { CameraNote(it) }
    }
}

@Composable
private fun CameraNote(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
    )
}
