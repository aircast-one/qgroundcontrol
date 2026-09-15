package one.aircast.mapspike

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier

@Composable
fun AltitudeModePicker(
    item: MissionItem,
    onPick: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    var open by remember(item.index) { mutableStateOf(false) }
    val json by mapPath(altitudeModesPath(MISSION_CONTEXT, item.altitudeMode))
    val view = altitudeModesView(json)
    val picks = choosable(view)
    val live = offersChoice(view)
    val current = picks.firstOrNull { it.current }?.title
        ?: item.altitudeFrameText.ifBlank { FRAME_UNKNOWN }

    TextButton(onClick = { open = true }, enabled = live, modifier = modifier) {
        Text(current)
    }
    DropdownMenu(expanded = open && live, onDismissRequest = { open = false }) {
        picks.forEach { offer ->
            DropdownMenuItem(
                text = {
                    Column {
                        Text(offer.title)
                        val note = refusalFor(view, offer.raw) ?: offer.help
                        if (note.isNotBlank()) {
                            Text(note, style = MaterialTheme.typography.labelSmall)
                        }
                    }
                },
                enabled = offer.enabled,
                onClick = {
                    open = false
                    onPick(offer.raw)
                },
            )
        }
    }
}
