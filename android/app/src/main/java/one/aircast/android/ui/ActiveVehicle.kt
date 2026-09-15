package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
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
import org.json.JSONObject
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.VehicleBridge
import one.aircast.mapspike.optText

internal const val VEHICLES_VIEW = "view.vehicles"

internal data class VehicleChoice(
    val id: Int,
    val name: String,
    val state: String,
    val link: String,
    val contactLost: Boolean,
    val active: Boolean,
)

internal data class VehicleChoices(
    val ambiguous: Boolean,
    val choices: List<VehicleChoice>,
) {
    val active: VehicleChoice? = choices.firstOrNull { it.active }
}

internal fun vehicleChoices(view: JSONObject?): VehicleChoices {
    val listed = view?.optJSONArray("vehicles")
    return VehicleChoices(
        ambiguous = view?.optBoolean("ambiguous") == true,
        choices = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed!!.optJSONObject(index)?.let { entry ->
                val id = entry.optInt("id", -1).takeIf { it >= 0 } ?: return@mapNotNull null
                VehicleChoice(
                    id = id,
                    name = entry.optText("name").ifBlank { "Vehicle $id" },
                    state = vehicleChoiceState(entry),
                    link = entry.optText("link"),
                    contactLost = !entry.isNull("contactLost") && entry.optBoolean("contactLost"),
                    active = entry.optBoolean("active"),
                )
            }
        },
    )
}

private fun vehicleChoiceState(entry: JSONObject): String = listOfNotNull(
    entry.optText("flightMode").ifBlank { null },
    when {
        entry.optBoolean("flying") -> "Flying"
        entry.optBoolean("armed") -> "Armed"
        else -> "Disarmed"
    },
).joinToString(" · ")

internal fun vehicleChoiceLine(choice: VehicleChoice): String = when {
    choice.contactLost -> "No contact · ${choice.link}"
    else -> listOfNotNull(choice.state.ifBlank { null }, choice.link.ifBlank { null }).joinToString(" · ")
}

internal fun activeVehicleTitle(choices: VehicleChoices, subtitle: String): String = when {
    !choices.ambiguous -> subtitle
    else -> listOfNotNull(choices.active?.name, subtitle.ifBlank { null }).joinToString(" · ")
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VehicleStateChip(modifier: Modifier = Modifier) {
    val flyJson by qgcPath(FLY_STATE)
    val fly = remember(flyJson) { flyState(flyJson) }
    val vehiclesJson by qgcPath(VEHICLES_VIEW)
    val choices = remember(vehiclesJson) { vehicleChoices(vehiclesJson) }
    val lost = fly?.contactLost == true
    val subtitle = vehicleSubtitle(fly)
    var picking by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }

    Row(
        modifier = modifier.let { if (choices.ambiguous) it.clickable { picking = true } else it },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = activeVehicleTitle(choices, subtitle),
            style = MaterialTheme.typography.labelLarge,
            fontWeight = if (lost) FontWeight.Bold else FontWeight.Normal,
            color = if (lost) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface,
            maxLines = 1,
        )
        if (choices.ambiguous) {
            Icon(
                imageVector = Icons.Default.KeyboardArrowDown,
                contentDescription = "Choose which vehicle to fly",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }

    if (picking) {
        ModalBottomSheet(onDismissRequest = { picking = false }) {
            Text(
                text = "Flying",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            choices.choices.forEach { choice ->
                ListItem(
                    headlineContent = { Text(choice.name) },
                    supportingContent = { Text(vehicleChoiceLine(choice)) },
                    trailingContent = {
                        if (choice.active) {
                            Icon(Icons.Default.Check, contentDescription = "Flying this one")
                        }
                    },
                    modifier = Modifier.clickable(enabled = !choice.active) {
                        scope.launch {
                            val switched = withContext(Dispatchers.Default) {
                                VehicleBridge.askFor(choice.id)
                            }
                            refusal = if (switched) null else VehicleBridge.lastRefusal
                                ?: "That vehicle did not take control."
                            if (switched) picking = false
                        }
                    },
                )
                HorizontalDivider()
            }
            refusal?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp, vertical = 12.dp),
                )
            }
            FootNote(
                "Arm, Takeoff and every action on this screen go to the vehicle shown here.",
            )
        }
    }
}
