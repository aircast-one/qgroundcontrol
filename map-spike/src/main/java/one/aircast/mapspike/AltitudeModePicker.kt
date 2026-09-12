package one.aircast.mapspike

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

@Composable
fun AltitudeModePicker(
    item: MissionItem,
    read: (String) -> JSONObject?,
    onPick: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    var open by remember(item.index) { mutableStateOf(false) }
    var view by remember(item.index) { mutableStateOf<AltitudeModesView?>(null) }
    LaunchedEffect(item.index, item.altitudeMode) {
        val path = altitudeModesPath(MISSION_CONTEXT, item.altitudeMode)
        view = withContext(Dispatchers.Default) { altitudeModesView(read(path)) }
    }
    val picks = choosable(view)
    if (picks.isEmpty()) {
        return
    }
    val current = picks.firstOrNull { it.current }?.title ?: item.altitudeFrameText.ifBlank { "Frame" }

    TextButton(onClick = { open = true }, modifier = modifier) { Text(current) }
    DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
        picks.forEach { offer ->
            DropdownMenuItem(
                text = { Text(offer.title) },
                enabled = offer.enabled,
                onClick = {
                    open = false
                    onPick(offer.raw)
                },
                trailingIcon = {
                    val note = refusalFor(view, offer.raw) ?: offer.help
                    if (note.isNotBlank()) {
                        Text(note, style = MaterialTheme.typography.labelSmall)
                    }
                },
            )
        }
    }
}
