package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Surface
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.background
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.LocalContentColor
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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import androidx.compose.runtime.DisposableEffect
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.PlanMapScreen

private const val NOTICE_MILLIS = 4000L

internal const val APPLY_DEFAULT_ALTITUDE = "core.plan.applyDefaultAltitude"
internal const val DISMISS_ALTITUDE_PROMPT = "core.plan.dismissAltitudePrompt"

internal data class AltitudePrompt(val title: String, val text: String)

internal const val LOAD_VEHICLE_PLAN = "core.plan.loadVehiclePlan"
internal const val KEEP_CURRENT_PLAN = "core.plan.keepCurrentPlan"

internal data class VehicleChangePrompt(val title: String, val text: String, val loadText: String, val keepText: String)

internal fun vehicleChangePrompt(view: org.json.JSONObject?): VehicleChangePrompt? =
    view?.optJSONObject("vehicleChangePrompt")?.let { VehicleChangePrompt(it.optString("title"), it.optString("text"), it.optString("loadText"), it.optString("keepText")) }

internal fun applyAltitudePrompt(view: org.json.JSONObject?): AltitudePrompt? =
    view?.optJSONObject("applyAltitudePrompt")?.let { AltitudePrompt(it.optString("title"), it.optString("text")) }

@Composable
fun PlanTab(modifier: Modifier = Modifier, onBack: () -> Unit = {}) {
    var notice by remember { mutableStateOf<String?>(null) }
    var menuOpen by remember { mutableStateOf(false) }
    var pending by remember { mutableStateOf<PlanConfirm?>(null) }
    val files = rememberPlanFileActions { notice = it }

    val planStatus by qgcPath("view.plan")
    val containsItems = remember(planStatus) { planContainsItems(planStatus) }
    val dirty = remember(planStatus) { planIsDirty(planStatus) }
    val syncing = remember(planStatus) { planIsSyncing(planStatus) }
    val syncProgress = remember(planStatus) { planSyncProgress(planStatus) }
    var showDefaults by remember { mutableStateOf(false) }
    var showTransform by remember { mutableStateOf(false) }
    val can = planActions(planStatus)
    val history = planHistory(planStatus)
    var undrawn by remember { mutableStateOf<List<String>>(emptyList()) }
    var centre by remember { mutableStateOf<Pair<Double, Double>?>(null) }

    DisposableEffect(Unit) {
        offMainDetached { Qgc.set("plan.undoTracking", true) }
        onDispose { offMainDetached { Qgc.set("plan.undoTracking", false) } }
    }

    LaunchedEffect(syncing, containsItems, files.documentName()) {
        undrawn = withContext(Dispatchers.Default) { undrawnItemNames(visualItems()) }
    }

    LaunchedEffect(notice) {
        if (notice != null) {
            delay(NOTICE_MILLIS)
            notice = null
        }
    }

    if (showDefaults) {
        PlanDefaultsDialog(planStatus) { showDefaults = false }
    }

    vehicleChangePrompt(planStatus)?.let { prompt ->
        AlertDialog(
            onDismissRequest = {},
            title = { Text(prompt.title) },
            text = {
                androidx.compose.foundation.layout.Column {
                    Text(prompt.text)
                    TextButton(onClick = { offMainDetached { Qgc.invoke(LOAD_VEHICLE_PLAN) } }) { Text(prompt.loadText) }
                    TextButton(onClick = { offMainDetached { Qgc.invoke(KEEP_CURRENT_PLAN) } }) { Text(prompt.keepText) }
                }
            },
            confirmButton = {},
        )
    }

    applyAltitudePrompt(planStatus)?.let { prompt ->
        AlertDialog(
            onDismissRequest = { offMainDetached { Qgc.invoke(DISMISS_ALTITUDE_PROMPT) } },
            title = { Text(prompt.title) },
            text = { Text(prompt.text) },
            confirmButton = { TextButton(onClick = { offMainDetached { Qgc.invoke(APPLY_DEFAULT_ALTITUDE) } }) { Text("Yes") } },
            dismissButton = { TextButton(onClick = { offMainDetached { Qgc.invoke(DISMISS_ALTITUDE_PROMPT) } }) { Text("No") } },
        )
    }

    if (showTransform) {
        PlanTransformDialog { showTransform = false }
    }

    pending?.let { kind ->
        val copy = confirmCopy(kind)
        val act = when (kind) {
            PlanConfirm.Open -> files.open
            PlanConfirm.NewPlan -> files.newPlan
            PlanConfirm.ClearMission -> files.clearMission
            PlanConfirm.Download -> files.download
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

    files.patternChoice.options().takeIf { it.isNotEmpty() }?.let { options ->
        AlertDialog(
            onDismissRequest = files.patternChoice.cancel,
            title = { Text("Import as which pattern?") },
            text = {
                Column {
                    options.forEach { name ->
                        TextButton(onClick = { files.patternChoice.pick(name) }) { Text(name) }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = files.patternChoice.cancel) { Text("Cancel") }
            },
        )
    }

    Box(modifier.fillMaxSize()) {
        PlanMapScreen(
            Modifier.fillMaxSize(),
            onCentre = { lat, lon -> centre = lat to lon },
            itemEditor = { index, at, close -> ItemEditor(index, at, centre, close) },
            header = { upload ->
                Column {
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 64.dp).padding(start = 4.dp, end = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        IconButton(onClick = onBack) { Icon(painterResource(R.drawable.ic_arrow_back), "Back to Fly") }
                        val title = planTitle(files.documentName())
                        Column(Modifier.weight(1f)) {
                            Text(
                                title,
                                style = MaterialTheme.typography.titleLarge,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            (notice ?: planStatusText(planStatus)).takeIf { it.isNotBlank() && it != title }?.let { line ->
                                Text(
                                    line,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = if (notice == null) 1 else 3,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                        Surface(
                            onClick = upload.onClick,
                            enabled = upload.enabled,
                            modifier = Modifier.alpha(if (upload.enabled) 1f else 0.38f),
                            shape = CircleShape,
                            color = if (upload.emphasised) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest,
                            contentColor = if (upload.emphasised) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
                        ) {
                            Box(Modifier.height(40.dp)) {
                                if (syncing) {
                                    val ink = LocalContentColor.current.copy(alpha = 0.28f)
                                    Box(Modifier.matchParentSize().drawBehind { drawRect(ink, size = size.copy(width = size.width * syncProgress)) })
                                }
                                Row(
                                    Modifier.height(40.dp).padding(start = 16.dp, end = 20.dp),
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Icon(painterResource(R.drawable.ic_upload), null, Modifier.size(20.dp))
                                    Text(upload.label, style = MaterialTheme.typography.labelLarge)
                                }
                            }
                        }
                        Box {
                            IconButton(onClick = { menuOpen = true }) { Icon(painterResource(R.drawable.ic_more_vert), "Plan menu") }
                            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                                DropdownMenuItem(
                                    text = { Text("Undo") },
                                    enabled = history.canUndo,
                                    onClick = { menuOpen = false; offMainDetached { Qgc.invoke("plan.undo") } },
                                )
                                DropdownMenuItem(
                                    text = { Text("Redo") },
                                    enabled = history.canRedo,
                                    onClick = { menuOpen = false; offMainDetached { Qgc.invoke("plan.redo") } },
                                )
                                HorizontalDivider()
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
                                    text = { Text("Save As…") },
                                    enabled = can.save,
                                    onClick = { menuOpen = false; files.saveAs() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Export KML…") },
                                    enabled = can.exportKml,
                                    onClick = { menuOpen = false; files.exportKml() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Import boundary…") },
                                    enabled = can.open,
                                    onClick = { menuOpen = false; files.importBoundary() },
                                )
                                HorizontalDivider()
                                DropdownMenuItem(
                                    text = { Text("Defaults…") },
                                    onClick = { menuOpen = false; showDefaults = true },
                                )
                                DropdownMenuItem(
                                    text = { Text("Transform…") },
                                    enabled = containsItems,
                                    onClick = { menuOpen = false; showTransform = true },
                                )
                                HorizontalDivider()
                                DropdownMenuItem(
                                    text = { Text("New Plan…") },
                                    enabled = can.newPlan,
                                    onClick = {
                                        menuOpen = false
                                        if (containsItems) pending = PlanConfirm.NewPlan else files.newPlan()
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Download from Vehicle") },
                                    enabled = can.download,
                                    onClick = {
                                        menuOpen = false
                                        if (dirty) pending = PlanConfirm.Download else files.download()
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Clear Mission") },
                                    enabled = can.clearFromVehicle,
                                    colors = MenuDefaults.itemColors(textColor = MaterialTheme.colorScheme.error),
                                    onClick = { menuOpen = false; pending = PlanConfirm.ClearMission },
                                )
                            }
                        }
                    }
                    undrawnItemsWarning(undrawn)?.let { warning ->
                        Text(
                            text = warning,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onErrorContainer,
                            modifier = Modifier
                                .fillMaxWidth()
                                .background(MaterialTheme.colorScheme.errorContainer)
                                .padding(horizontal = 12.dp, vertical = 8.dp),
                        )
                    }
                }
            },
        )
        PlanTemplates(
            planStatus = planStatus,
            centre = centre,
            onRefused = { notice = it },
            modifier = Modifier.align(Alignment.Center).padding(horizontal = 16.dp),
        )
    }
}
