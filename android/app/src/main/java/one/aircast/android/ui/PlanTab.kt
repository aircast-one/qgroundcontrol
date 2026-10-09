package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import one.aircast.android.bridge.PlanCommands
import one.aircast.android.bridge.offMainInOrder
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
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
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import one.aircast.map.aircast
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
import one.aircast.map.PlanMapScreen

private const val NOTICE_MILLIS = 4000L
private const val HEADER_ALPHA = 0.94f
private const val FLY_LABEL = "Fly"
private const val DISABLED_PILL_ALPHA = 0.38f
private val PILL_HEIGHT = 40.dp
private val HEADER_CORNER = 20.dp

internal const val APPLY_DEFAULT_ALTITUDE = "core.plan.applyDefaultAltitude"
internal const val DISMISS_ALTITUDE_PROMPT = "core.plan.dismissAltitudePrompt"

internal data class AltitudePrompt(val title: String, val text: String)

internal const val LOAD_VEHICLE_PLAN = "core.plan.loadVehiclePlan"
internal const val KEEP_CURRENT_PLAN = "core.plan.keepCurrentPlan"

internal data class VehicleChangePrompt(val connected: Boolean, val dirty: Boolean, val aircraftItems: Int?)

internal fun vehicleChangePrompt(view: org.json.JSONObject?): VehicleChangePrompt? =
    view?.optJSONObject("vehicleChangePrompt")?.let {
        VehicleChangePrompt(it.optBoolean("connected"), it.optBoolean("dirty"), it.optInt("aircraftItems", 0).takeIf { count -> count > 0 })
    }

internal data class PromptCopy(val title: String, val text: String, val primary: String, val secondary: String, val primaryKeeps: Boolean)

internal fun promptCopy(prompt: VehicleChangePrompt): PromptCopy {
    val stored = prompt.aircraftItems?.let { " (${itemsWord(it)})" }.orEmpty()
    return when {
        prompt.connected && prompt.dirty -> PromptCopy("Aircraft connected", "Keep the route you drew, or replace it with the route stored on the aircraft$stored?", "Keep my route", "Load the aircraft's route", primaryKeeps = true)
        prompt.connected -> PromptCopy("Aircraft connected", "Load the route stored on the aircraft$stored, or keep this one?", "Load the aircraft's route", "Keep this route", primaryKeeps = false)
        prompt.dirty -> PromptCopy("Aircraft disconnected", "Keep the route you were working on?", "Keep my route", "Discard it", primaryKeeps = true)
        else -> PromptCopy("Aircraft disconnected", "Keep this route, or start a new plan?", "Keep this route", "Start a new plan", primaryKeeps = true)
    }
}

private fun itemsWord(count: Int): String = if (count == 1) "1 item" else "$count items"

internal fun applyAltitudePrompt(view: org.json.JSONObject?): AltitudePrompt? =
    view?.optJSONObject("applyAltitudePrompt")?.let { AltitudePrompt(it.optString("title"), it.optString("text")) }

@Composable
fun PlanTab(modifier: Modifier = Modifier, onFly: () -> Unit = {}) {
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
    var newPlanOpen by remember { mutableStateOf(false) }
    var startFrom by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun startPlan(template: String?) {
        if (template == null) {
            files.newPlan()
        } else {
            scope.launch {
                val (lat, lon) = centre ?: return@launch run { notice = "The map has not reported its centre yet." }
                val refused = withContext(Dispatchers.IO) { Qgc.refusalOf(CREATE_FROM_TEMPLATE, template, lat, lon) }
                if (refused == null) one.aircast.map.PlanFocus.newPattern.value = one.aircast.map.NEWEST_PATTERN else notice = refused
            }
        }
    }

    if (newPlanOpen) {
        NewPlanDialog(planTemplates(planStatus), replacing = containsItems, onDismiss = { newPlanOpen = false }) { template ->
            newPlanOpen = false
            startFrom = template
            if (containsItems) pending = PlanConfirm.NewPlan else startPlan(template)
        }
    }

    DisposableEffect(Unit) {
        offMainInOrder { PlanCommands.setUndoTracking(true) }
        onDispose { offMainInOrder { PlanCommands.setUndoTracking(false) } }
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
        val copy = promptCopy(prompt)
        val keep = { offMainDetached { Qgc.invoke(KEEP_CURRENT_PLAN) } }
        val load = { offMainDetached { Qgc.invoke(LOAD_VEHICLE_PLAN) } }
        AlertDialog(
            onDismissRequest = {},
            title = { Text(copy.title) },
            text = { Text(copy.text) },
            confirmButton = { Button(onClick = if (copy.primaryKeeps) keep else load) { Text(copy.primary) } },
            dismissButton = { TextButton(onClick = if (copy.primaryKeeps) load else keep) { Text(copy.secondary) } },
        )
    }

    applyAltitudePrompt(planStatus)?.let { prompt ->
        AlertDialog(
            onDismissRequest = { offMainDetached { Qgc.invoke(DISMISS_ALTITUDE_PROMPT) } },
            title = { Text(sentenceCase(prompt.title)) },
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
        val begin: () -> Unit = { startPlan(startFrom) }
        val act = when (kind) {
            PlanConfirm.Open -> files.open
            PlanConfirm.NewPlan -> begin
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
            fitKey = files.opened(),
            onCentre = { lat, lon -> centre = lat to lon },
            itemPanel = { index, at, leg, remove -> ItemEditor(index, at, centre, leg, remove) },
            routeSettings = { RouteSettings(planStatus) },
            onTemplates = { newPlanOpen = true },
            header = { bar ->
                Column(Modifier.padding(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(
                        Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        val title = planTitle(files.documentName())
                        Box(Modifier.weight(1f)) {
                            Surface(shape = RoundedCornerShape(HEADER_CORNER), color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = HEADER_ALPHA)) {
                                Column(Modifier.heightIn(min = 40.dp).padding(horizontal = 16.dp, vertical = 6.dp), verticalArrangement = Arrangement.Center) {
                                    Text(
                                        title,
                                        style = MaterialTheme.typography.titleSmall,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    val warned = notice == null && !syncing && bar.warning != null
                                    (notice ?: bar.warning?.takeIf { warned } ?: bar.note?.takeIf { !syncing } ?: headerLine(planStatusText(planStatus), syncing, bar.stats)).takeIf { it.isNotBlank() && it != title }?.let { line ->
                                        Text(
                                            line,
                                            style = MaterialTheme.typography.labelSmall,
                                            color = if (warned) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                                            maxLines = if (notice == null) 1 else 3,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }
                                }
                            }
                        }
                        if (bar.upload.shown) {
                            PlanActionPill(
                                label = if (bar.upload.done) FLY_LABEL else bar.upload.label,
                                icon = when {
                                    bar.upload.done -> R.drawable.ic_flight
                                    bar.warning != null -> R.drawable.ic_warning
                                    else -> R.drawable.ic_upload
                                },
                                enabled = bar.upload.enabled,
                                container = when {
                                    bar.upload.done -> MaterialTheme.aircast.success
                                    bar.upload.emphasised -> MaterialTheme.colorScheme.primary
                                    else -> MaterialTheme.colorScheme.surfaceContainerHighest
                                },
                                content = when {
                                    bar.upload.done -> MaterialTheme.aircast.onSuccess
                                    bar.upload.emphasised -> MaterialTheme.colorScheme.onPrimary
                                    else -> MaterialTheme.colorScheme.onSurface
                                },
                                progress = syncProgress.takeIf { syncing },
                                onClick = if (bar.upload.done) onFly else bar.upload.onClick,
                            )
                        } else if (containsItems) {
                            PlanActionPill(
                                label = if (dirty) "Save" else "Saved",
                                icon = if (dirty) R.drawable.ic_download else R.drawable.ic_check_circle,
                                enabled = can.save && dirty,
                                container = if (dirty) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHighest,
                                content = if (dirty) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
                                progress = null,
                                onClick = { files.save() },
                            )
                        }
                        Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = HEADER_ALPHA)) {
                            IconButton(onClick = { menuOpen = true }, modifier = Modifier.size(40.dp)) { Icon(painterResource(R.drawable.ic_more_vert), "Plan menu") }
                            DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                                DropdownMenuItem(
                                    text = { Text("Redo") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_redo), null) },
                                    enabled = history.canRedo,
                                    onClick = { menuOpen = false; offMainInOrder { PlanCommands.redo() } },
                                )
                                HorizontalDivider()
                                DropdownMenuItem(
                                    text = { Text("Open plan…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_description), null) },
                                    enabled = can.open,
                                    onClick = {
                                        menuOpen = false
                                        if (dirty) pending = PlanConfirm.Open else files.open()
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Save") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_download), null) },
                                    enabled = can.save,
                                    onClick = { menuOpen = false; files.save() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Save as…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_edit), null) },
                                    enabled = can.save,
                                    onClick = { menuOpen = false; files.saveAs() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Export KML…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_send), null) },
                                    enabled = can.exportKml,
                                    onClick = { menuOpen = false; files.exportKml() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Import boundary…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_map), null) },
                                    enabled = can.open,
                                    onClick = { menuOpen = false; files.importBoundary() },
                                )
                                HorizontalDivider()
                                DropdownMenuItem(
                                    text = { Text("Defaults…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_tune), null) },
                                    onClick = { menuOpen = false; showDefaults = true },
                                )
                                DropdownMenuItem(
                                    text = { Text("Transform…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_straighten), null) },
                                    enabled = containsItems,
                                    onClick = { menuOpen = false; showTransform = true },
                                )
                                HorizontalDivider()
                                DropdownMenuItem(
                                    text = { Text("New plan…") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_add), null) },
                                    enabled = can.newPlan,
                                    onClick = {
                                        menuOpen = false
                                        newPlanOpen = true
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Load from vehicle") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_upload), null) },
                                    enabled = can.download,
                                    onClick = {
                                        menuOpen = false
                                        if (dirty) pending = PlanConfirm.Download else files.download()
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Clear mission") },
                                    leadingIcon = { Icon(painterResource(R.drawable.ic_delete), null) },
                                    enabled = can.clearFromVehicle,
                                    colors = MenuDefaults.itemColors(textColor = MaterialTheme.colorScheme.error, leadingIconColor = MaterialTheme.colorScheme.error),
                                    onClick = { menuOpen = false; pending = PlanConfirm.ClearMission },
                                )
                            }
                        }
                    }
                    undrawnItemsWarning(undrawn)?.let { warning ->
                        Surface(shape = RoundedCornerShape(HEADER_CORNER), color = MaterialTheme.colorScheme.errorContainer) {
                            Text(
                                text = warning,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onErrorContainer,
                                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                            )
                        }
                    }
                }
            },
        )
    }
}

internal fun planStatsLine(stats: List<one.aircast.map.PlanStat>): String =
    stats.mapNotNull { stat ->
        when (stat.label) {
            "Items" -> stat.value.takeIf { it != "0" }?.let { "$it ${if (it == "1") "item" else "items"}" }
            "Max alt" -> "max ${stat.value}"
            else -> stat.value
        }
    }.joinToString(" \u00b7 ")

internal fun headerLine(status: String, syncing: Boolean, stats: List<one.aircast.map.PlanStat>): String =
    if (syncing || stats.none { it.label == "Items" && it.value != "0" }) status else planStatsLine(stats)

@Composable
private fun PlanActionPill(
    label: String,
    icon: Int,
    enabled: Boolean,
    container: androidx.compose.ui.graphics.Color,
    content: androidx.compose.ui.graphics.Color,
    progress: Float?,
    onClick: () -> Unit,
) {
    Surface(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.alpha(if (enabled) 1f else DISABLED_PILL_ALPHA),
        shape = CircleShape,
        color = container,
        contentColor = content,
    ) {
        Box(Modifier.height(PILL_HEIGHT)) {
            progress?.let { done ->
                val ink = LocalContentColor.current.copy(alpha = 0.28f)
                Box(Modifier.matchParentSize().drawBehind { drawRect(ink, size = size.copy(width = size.width * done)) })
            }
            Row(
                Modifier.height(PILL_HEIGHT).padding(start = 20.dp, end = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(painterResource(icon), null, Modifier.size(20.dp))
                Text(label, style = MaterialTheme.typography.labelLarge)
            }
        }
    }
}
