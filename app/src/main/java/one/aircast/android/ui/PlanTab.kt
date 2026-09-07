package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.qgcBool
import one.aircast.mapspike.PlanMapScreen

private const val NOTICE_MILLIS = 4000L

@Composable
fun PlanTab(modifier: Modifier = Modifier) {
    var notice by remember { mutableStateOf<String?>(null) }
    var menuOpen by remember { mutableStateOf(false) }
    var confirmOpen by remember { mutableStateOf(false) }
    val files = rememberPlanFileActions { notice = it }

    val dirty by qgcBool("plan.dirty")
    val syncing by qgcBool("plan.syncInProgress")
    val containsItems by qgcBool("plan.containsItems")
    val hasMissionItems by qgcBool("plan.missionController.containsItems")
    val can = planActions(syncing, containsItems, hasMissionItems)

    LaunchedEffect(notice) {
        if (notice != null) {
            delay(NOTICE_MILLIS)
            notice = null
        }
    }

    if (confirmOpen) {
        AlertDialog(
            onDismissRequest = { confirmOpen = false },
            title = { Text("Discard unsaved changes?") },
            text = { Text("Opening a plan replaces the one you have. Your unsaved changes cannot be recovered.") },
            confirmButton = {
                TextButton(onClick = { confirmOpen = false; files.open() }) { Text("Discard and open") }
            },
            dismissButton = {
                TextButton(onClick = { confirmOpen = false }) { Text("Keep editing") }
            },
        )
    }

    Column(modifier.fillMaxSize()) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            TextButton(
                enabled = can.open,
                onClick = { if (dirty) confirmOpen = true else files.open() },
            ) { Text("Open") }
            TextButton(enabled = can.save, onClick = files.save) { Text("Save") }
            Box {
                TextButton(onClick = { menuOpen = true }) { Text("More") }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    DropdownMenuItem(
                        text = { Text("Save as…") },
                        enabled = can.save,
                        onClick = { menuOpen = false; files.saveAs() },
                    )
                    DropdownMenuItem(
                        text = { Text("Export KML…") },
                        enabled = can.exportKml,
                        onClick = { menuOpen = false; files.exportKml() },
                    )
                }
            }
            Text(
                text = notice ?: planStatusText(files.documentName(), dirty),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(start = 8.dp),
            )
        }
        PlanMapScreen(Modifier.weight(1f))
    }
}
