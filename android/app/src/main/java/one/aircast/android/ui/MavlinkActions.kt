package one.aircast.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val MAVLINK_ACTIONS_VIEW = "view.mavlinkActions"
internal const val MAVLINK_ACTIONS_SEND = "mavlinkActions.send"
internal const val MAVLINK_ACTIONS_GROUP = "mavlinkActionsSettings"
internal const val NO_ACTIONS_FILE = "<None>"

internal data class MavlinkActionEntry(val label: String, val description: String)

internal data class MavlinkActions(
    val files: List<String>,
    val flyViewFile: String,
    val joystickFile: String,
    val flyViewPath: String,
    val joystickPath: String,
    val actions: List<MavlinkActionEntry>,
    val folderNote: String = "",
)

internal fun mavlinkActions(view: JSONObject?): MavlinkActions? = view?.optJSONArray("files")?.let { files ->
    val actions = view.optJSONArray("actions")
    MavlinkActions(
        files = (0 until files.length()).map { files.optString(it) },
        flyViewFile = view.optText("flyViewFile"),
        joystickFile = view.optText("joystickFile"),
        flyViewPath = view.optText("flyViewPath"),
        joystickPath = view.optText("joystickPath"),
        actions = (0 until (actions?.length() ?: 0)).mapNotNull { at -> actions!!.optJSONObject(at)?.let { MavlinkActionEntry(it.optText("label"), it.optText("description")) } },
        folderNote = view.optText("folderNote"),
    )
}

internal fun chosenFile(option: String): String = if (option == NO_ACTIONS_FILE) "" else option

@Composable
internal fun MavlinkActionsSection(onWrite: () -> Unit) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<MavlinkActions?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) { read = withContext(Dispatchers.Default) { mavlinkActions(Qgc.get(MAVLINK_ACTIONS_VIEW)) } }
    val actions = read ?: return
    val options = listOf(NO_ACTIONS_FILE) + actions.files
    fun choose(path: String, option: String) {
        scope.launch {
            withContext(Dispatchers.Default) { Qgc.set(path, chosenFile(option)) }
            revision++
            onWrite()
        }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp)) {
        if (actions.folderNote.isNotBlank()) Text(actions.folderNote, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        FileChoice("Fly view actions", actions.flyViewFile, options) { choose(actions.flyViewPath, it) }
        FileChoice("Joystick actions", actions.joystickFile, options) { choose(actions.joystickPath, it) }
    }
}

@Composable
private fun FileChoice(label: String, current: String, options: List<String>, onPick: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(enabled = options.size > 1, onClick = { open = true }) { Text(current.ifBlank { NO_ACTIONS_FILE }) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                options.forEach { option ->
                    DropdownMenuItem(text = { Text(option) }, onClick = {
                        open = false
                        onPick(option)
                    })
                }
            }
        }
    }
}

@Composable
internal fun FlyViewMavlinkActions(onSent: () -> Unit) {
    var read by remember { mutableStateOf<MavlinkActions?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) { read = withContext(Dispatchers.Default) { mavlinkActions(Qgc.get(MAVLINK_ACTIONS_VIEW)) } }
    read?.actions?.forEachIndexed { index, action ->
        TextButton(
            onClick = {
                onSent()
                scope.launch(Dispatchers.Default) { Qgc.invoke(MAVLINK_ACTIONS_SEND, index) }
            },
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(action.label, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary, modifier = Modifier.fillMaxWidth())
        }
    }
}
