package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Warning
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
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.CHOOSER_TITLE
import one.aircast.mapspike.VEHICLES_VIEW
import one.aircast.mapspike.VehicleBridge
import one.aircast.mapspike.activeVehicleTitle
import one.aircast.mapspike.lostVehicles
import one.aircast.mapspike.lostVehiclesText
import one.aircast.mapspike.vehicleChoiceLine
import one.aircast.mapspike.vehicleChoices

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
            val silent = lostVehiclesText(lostVehicles(choices))
            Icon(
                imageVector = if (silent == null) Icons.Default.KeyboardArrowDown else Icons.Default.Warning,
                contentDescription = silent ?: "Choose which vehicle to fly",
                tint = if (silent == null) {
                    MaterialTheme.colorScheme.onSurfaceVariant
                } else {
                    MaterialTheme.colorScheme.error
                },
            )
        }
    }

    if (picking) {
        ModalBottomSheet(onDismissRequest = { picking = false }) {
            Text(
                text = lostVehiclesText(lostVehicles(choices)) ?: CHOOSER_TITLE,
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
