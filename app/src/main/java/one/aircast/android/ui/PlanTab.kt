package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MenuDefaults
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
    var pending by remember { mutableStateOf<PlanConfirm?>(null) }
    val files = rememberPlanFileActions { notice = it }

    val dirty by qgcBool("plan.dirty")
    val syncing by qgcBool("plan.syncInProgress")
    val containsItems by qgcBool("plan.containsItems")
    val hasMissionItems by qgcBool("plan.missionController.containsItems")
    val offline by qgcBool("plan.offline")
    val can = planActions(syncing, containsItems, hasMissionItems, offline)

    LaunchedEffect(notice) {
        if (notice != null) {
            delay(NOTICE_MILLIS)
            notice = null
        }
    }

    pending?.let { kind ->
        val copy = confirmCopy(kind)
        val act = when (kind) {
            PlanConfirm.Open -> files.open
            PlanConfirm.NewPlan -> files.newPlan
            PlanConfirm.ClearMission -> files.clearMission
        }
        AlertDialog(
            onDismissRequest = { pending = null },
            title = { Text(copy.title) },
            text = { Text(copy.body) },
            confirmButton = {
                TextButton(
                    colors = if (copy.destructive) {
                        ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error)
                    } else {
                        ButtonDefaults.textButtonColors()
                    },
                    onClick = { pending = null; act() },
                ) { Text(copy.confirm) }
            },
            dismissButton = {
                TextButton(onClick = { pending = null }) { Text("Keep editing") }
            },
        )
    }

    Column(modifier.fillMaxSize()) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Box {
                TextButton(onClick = { menuOpen = true }) { Text("File") }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    DropdownMenuItem(
                        text = { Text("Open…") },
                        enabled = can.open,
                        onClick = {
                            menuOpen = false
                            if (dirty) pending = PlanConfirm.Open else files.open()
                        },
                    )
                    DropdownMenuItem(
                        text = { Text("Save") },
                        enabled = can.save,
                        onClick = { menuOpen = false; files.save() },
                    )
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
                    HorizontalDivider()
                    DropdownMenuItem(
                        text = { Text("New plan") },
                        enabled = can.newPlan,
                        onClick = {
                            menuOpen = false
                            if (dirty) pending = PlanConfirm.NewPlan else files.newPlan()
                        },
                    )
                    DropdownMenuItem(
                        text = { Text("Clear mission") },
                        enabled = can.clearMission,
                        colors = MenuDefaults.itemColors(textColor = MaterialTheme.colorScheme.error),
                        onClick = { menuOpen = false; pending = PlanConfirm.ClearMission },
                    )
                }
            }
            Text(
                text = notice ?: planStatusText(files.documentName(), dirty, offline),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = if (notice == null) 1 else 3,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(start = 8.dp),
            )
        }
        PlanMapScreen(Modifier.weight(1f))
    }
}
