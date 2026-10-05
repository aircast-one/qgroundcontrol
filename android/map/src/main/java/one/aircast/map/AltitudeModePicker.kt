package one.aircast.map

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

const val GLOBAL_FRAME_MIXED = 0
const val GLOBAL_FRAME_RELATIVE = 1

fun itemReferenceShown(globalFrame: Int?): Boolean = globalFrame != GLOBAL_FRAME_RELATIVE

fun itemReferenceSelectable(globalFrame: Int?): Boolean = globalFrame == null || globalFrame == GLOBAL_FRAME_MIXED

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun AltitudeModePicker(
    item: MissionItem,
    onPick: (Int) -> Unit,
    modifier: Modifier = Modifier,
    globalFrameMixed: Boolean = true,
) {
    val json by mapPath(altitudeModesPath(ITEM_CONTEXT, item.altitudeMode))
    val view = altitudeModesView(json)
    val picks = choosable(view)
    val live = globalFrameMixed && offersChoice(view)
    val notes = picks.filterNot { it.current }.mapNotNull { offer -> refusalFor(view, offer.raw)?.let { "${offer.title}: $it" } }

    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        if (picks.isEmpty()) {
            Text(item.altitudeFrameText.ifBlank { FRAME_UNKNOWN }, style = MaterialTheme.typography.labelMedium)
        }
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            picks.forEach { offer ->
                FilterChip(
                    selected = offer.current,
                    enabled = live && offer.enabled || offer.current,
                    onClick = { if (live && !offer.current) onPick(offer.raw) },
                    label = { Text(sentenceCase(offer.title)) },
                )
            }
        }
        if (live) notes.forEach { Text(it, style = MaterialTheme.typography.labelSmall) }
    }
}
