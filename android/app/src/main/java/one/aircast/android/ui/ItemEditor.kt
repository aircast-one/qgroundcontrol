package one.aircast.android.ui

import one.aircast.mapspike.aircast
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
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
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.PlanBridge
import one.aircast.mapspike.TrackPoint
import one.aircast.mapspike.optText
import org.json.JSONArray
import org.json.JSONObject
import androidx.compose.material3.Button

internal fun itemFactsPath(index: Int): String = "view.itemFacts($index)"

internal fun itemCommandPath(index: Int): String = "plan.missionController.visualItems.$index.command"

internal fun itemRawEditPath(index: Int): String = "plan.missionController.visualItems.$index.rawEdit"

internal const val RAW_EDIT_NOTE = "Provides advanced access to all commands/parameters. Be very careful!"

internal fun itemNote(view: JSONObject?, rawOn: Boolean): String? =
    if (rawOn) RAW_EDIT_NOTE else view?.optText("commandDescription")?.ifBlank { null }

internal fun wizardLines(view: JSONObject?): List<String> =
    view?.takeIf { it.optBoolean("wizardMode") }?.optJSONArray("wizardText")?.let { lines -> (0 until lines.length()).map { lines.optString(it) } }.orEmpty()

internal fun wizardModePath(index: Int): String = "plan.missionController.visualItems.$index.wizardMode"

internal fun commandEditable(view: JSONObject?): Boolean = view?.optBoolean("simple") == true && view.optBoolean("takeoff") != true

internal fun startCategory(categories: List<String>, itemCategory: String?): String? =
    itemCategory?.takeIf { it in categories } ?: categories.firstOrNull()

internal fun mapCenterHintPath(index: Int): String = "plan.missionController.visualItems.$index.setMapCenterHintForCommandChange"
internal const val RAW_EDIT_STUCK = "You have made changes to the mission item which cannot be shown in Simple Mode"

internal data class EntryPoint(val label: String, val value: String, val path: String)

internal fun entryPoint(view: JSONObject?): EntryPoint? =
    view?.optJSONObject("entryPoint")?.let { EntryPoint(it.optText("label"), it.optText("value"), it.optText("path")) }?.takeIf { it.path.isNotBlank() }

internal fun isLandingPattern(view: JSONObject?): Boolean = view?.optBoolean("landing") == true

internal fun landingNotes(view: JSONObject?): List<String> =
    view?.optJSONArray("landingNotes")?.let { notes -> (0 until notes.length()).map { notes.optString(it) }.filter { it.isNotBlank() } }.orEmpty()

internal fun vehicleHeading(fact: JSONObject?): Double? = fact?.optDouble("value")?.takeIf { !it.isNaN() }

internal fun vehicleCoordinate(coordinate: JSONObject?): JSONObject? =
    coordinate?.takeIf { it.optBoolean("valid", true) && it.has("latitude") && it.has("longitude") }
        ?.let { JSONObject().put("latitude", it.optDouble("latitude")).put("longitude", it.optDouble("longitude")).put("altitude", 0) }

private fun setToVehicleHeading(index: Int): String? =
    vehicleHeading(Qgc.get("vehicle.heading"))?.let { Qgc.writeRefusal("plan.missionController.visualItems.$index.landingHeading", it) } ?: "The vehicle has not reported its heading."

private fun setToVehicleLocation(index: Int): String? =
    vehicleCoordinate(Qgc.get("vehicle.coordinate")?.let { it.optJSONObject("value") ?: it })
        ?.let { Qgc.writeRefusal("plan.missionController.visualItems.$index.landingCoordinate", it) } ?: "The vehicle has no position yet."

internal fun areaHelp(view: JSONObject?): String? = view?.optText("areaHelp")?.takeIf { it.isNotBlank() }

internal data class RawEdit(val on: Boolean, val friendlyAllowed: Boolean)

internal fun rawEdit(view: JSONObject?): RawEdit? =
    view?.takeIf { it.optBoolean("simple") }?.let { RawEdit(it.optBoolean("rawEdit"), it.optBoolean("friendlyEditAllowed")) }

internal fun rawEditRefusal(current: RawEdit): String? = RAW_EDIT_STUCK.takeIf { current.on && !current.friendlyAllowed }

internal data class CommandChoice(val id: Int, val name: String, val description: String)

internal fun commandChoices(result: Any?): List<CommandChoice> {
    val listed = result as? JSONArray ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let {
            CommandChoice(it.optInt("command"), it.optText("friendlyName"), it.optText("description"))
        }
    }
}

internal fun categoryNames(result: Any?): List<String> {
    val listed = result as? JSONArray ?: return emptyList()
    return (0 until listed.length()).map { listed.optText(it) }
}

@Composable
private fun OptionalFactRow(fact: one.aircast.android.bridge.Fact, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    Row(Modifier.fillMaxWidth().padding(start = 20.dp), verticalAlignment = Alignment.CenterVertically) {
        Switch(checked = fact.optionalSet, onCheckedChange = { on ->
            scope.launch {
                withContext(Dispatchers.Default) { Qgc.set(fact.path, if (on) 0.0 else null) }
                onWrite()
            }
        })
        Box(Modifier.weight(1f)) {
            FactRow(if (fact.optionalSet) fact else fact.copy(enabled = false), onWrite = onWrite)
        }
    }
}

internal fun altitudesRelative(view: JSONObject?): Boolean? =
    view?.takeIf { it.optBoolean("landing") && it.has("altitudesAreRelative") && !it.isNull("altitudesAreRelative") }?.optBoolean("altitudesAreRelative")

internal fun altitudeHint(view: JSONObject?): String? =
    view?.takeIf { it.has("altitudeHint") && !it.isNull("altitudeHint") }?.optString("altitudeHint")?.ifBlank { null }

internal fun previousCoordinate(view: JSONObject?): Pair<Double, Double>? =
    view?.optJSONObject("previousCoordinate")?.let { it.optDouble("latitude") to it.optDouble("longitude") }?.takeIf { !it.first.isNaN() && !it.second.isNaN() }

internal fun itemFields(view: JSONObject?): List<one.aircast.android.bridge.Fact> {
    val listed = view?.optJSONArray("fields") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it)?.let(::factFromControl) }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ItemEditor(index: Int, at: TrackPoint?, mapCentre: Pair<Double, Double>?, onDismiss: () -> Unit) {
    var revision by remember(index) { mutableIntStateOf(0) }
    var view by remember(index) { mutableStateOf<JSONObject?>(null) }
    var choosing by remember(index) { mutableStateOf(false) }
    var editingPosition by remember(index) { mutableStateOf(false) }
    var positionMenu by remember(index) { mutableStateOf(false) }
    var refusal by remember(index) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(index, revision) {
        view = withContext(Dispatchers.Default) { Qgc.get(itemFactsPath(index)) }
    }

    val fields = remember(view) { itemFields(view) }
    val raw = remember(view) { rawEdit(view) }
    val connected = hasVehicle()
    val camera = remember(view) { cameraCalc(view) }
    val stats by androidx.compose.runtime.produceState<one.aircast.mapspike.SurveyStats?>(null, index, revision, camera != null) {
        value = if (camera == null) null else withContext(Dispatchers.Default) { one.aircast.mapspike.surveyStats(Qgc.get("view.surveyStats($index)")) }
    }
    val positionStart = at ?: (mapCentre ?: previousCoordinate(view))?.let { (latitude, longitude) -> TrackPoint(latitude, longitude) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(itemEditorTitle(view, index), style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                if (at != null || view?.optBoolean("specifiesCoordinate") == true) {
                    Box {
                        TextButton(onClick = { positionMenu = true }) { Text("Position") }
                        val previous = previousCoordinate(view)
                        val moveTo: (Pair<Double, Double>?) -> Unit = { target ->
                            positionMenu = false
                            scope.launch {
                                val moved = withContext(Dispatchers.Default) {
                                    (target ?: geoOf(Qgc.get("vehicle.coordinate")))?.let { PlanBridge.moveItem(index, it.first, it.second) } ?: false
                                }
                                refusal = if (moved) null else "The item could not be moved there."
                                revision++
                            }
                        }
                        DropdownMenu(expanded = positionMenu, onDismissRequest = { positionMenu = false }) {
                            DropdownMenuItem(text = { Text("Move to Vehicle Position") }, enabled = connected, onClick = { moveTo(null) })
                            DropdownMenuItem(text = { Text("Move to Previous Item") }, enabled = previous != null, onClick = { moveTo(previous) })
                            DropdownMenuItem(text = { Text("Edit position…") }, enabled = positionStart != null, onClick = { positionMenu = false; editingPosition = true })
                        }
                    }
                }
                if (commandEditable(view)) {
                    TextButton(onClick = { choosing = true }) { Text("Change command") }
                }
            }
            val wizard = wizardLines(view)
            if (wizard.isNotEmpty()) {
                Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(wizard.first(), style = MaterialTheme.typography.bodyMedium)
                    wizard.drop(1).forEach { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    fields.firstOrNull { it.path.endsWith(".loiterClockwise") }?.let { clockwise -> FactRow(clockwise) { revision++ } }
                    Button(onClick = {
                        scope.launch {
                            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(wizardModePath(index), false) }
                            revision++
                        }
                    }, modifier = Modifier.fillMaxWidth()) { Text("Done") }
                }
            }
            if (index == 0) {
                MissionAltitudeFrame()
                PlanVehicleRows()
                view?.optJSONObject("launchAltitude")?.let(::factFromControl)?.let { launch ->
                    Text("Launch position", style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp))
                    FactRow(launch, subtitle = "Actual position is set by the vehicle at flight time.") { revision++ }
                    Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("Position", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                        TextButton(enabled = mapCentre != null, onClick = {
                            val (latitude, longitude) = mapCentre ?: return@TextButton
                            scope.launch {
                                val moved = withContext(Dispatchers.Default) { PlanBridge.moveItem(0, latitude, longitude) }
                                refusal = if (moved) null else "The launch position could not be moved there."
                                revision++
                            }
                        }) { Text("Set to map center") }
                    }
                }
            }
            altitudesRelative(view)?.takeIf { wizard.isEmpty() }?.let { relative ->
                Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("Altitudes relative to launch", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Switch(checked = relative, onCheckedChange = { wanted ->
                        scope.launch {
                            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal("plan.missionController.visualItems.$index.altitudesAreRelative", wanted) }
                            revision++
                        }
                    })
                }
            }
            altitudeHint(view)?.let { hint ->
                Text(hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp))
            }
            raw?.let { current ->
                Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("Show all values", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Switch(checked = current.on, onCheckedChange = { wanted ->
                        scope.launch {
                            refusal = rawEditRefusal(current) ?: withContext(Dispatchers.Default) { Qgc.writeRefusal(itemRawEditPath(index), wanted) }
                            revision++
                        }
                    })
                }
            }
            itemNote(view, raw?.on == true)?.let { note ->
                Text(note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 20.dp))
            }
            refusal?.let {
                Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 20.dp))
            }
            if (isLandingPattern(view) && connected && wizard.isEmpty()) {
                Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { scope.launch { refusal = withContext(Dispatchers.Default) { setToVehicleHeading(index) }; revision++ } }) { Text("Set to vehicle heading") }
                    TextButton(onClick = { scope.launch { refusal = withContext(Dispatchers.Default) { setToVehicleLocation(index) }; revision++ } }) { Text("Set to vehicle location") }
                }
            }
            landingNotes(view).takeIf { wizard.isEmpty() }.orEmpty().forEach { note ->
                Text(note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning, modifier = Modifier.padding(horizontal = 20.dp))
            }
            areaHelp(view)?.let { help ->
                Text(help, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 20.dp, vertical = 12.dp))
            }
            if (areaHelp(view) == null && wizard.isEmpty()) LazyColumn(Modifier.heightIn(max = 480.dp)) {
                items(fields, key = { it.path }) { fact ->
                    if (fact.optional) OptionalFactRow(fact) { revision++ } else FactRow(fact) { revision++ }
                }
                item(key = "itemCamera") {
                    one.aircast.mapspike.ItemCameraSection(index, Modifier.fillMaxWidth().padding(horizontal = 20.dp)) { revision++ }
                }
                camera?.let { block ->
                    item(key = "camera") {
                        CameraCalcHeader(block) { path, value ->
                            scope.launch {
                                refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(path, value) }
                                revision++
                            }
                        }
                    }
                    items(shownCameraFacts(block), key = { it.path }) { fact ->
                        FactRow(fact) { revision++ }
                    }
                }
                entryPoint(view)?.let { entry ->
                    item(key = "entry") {
                        Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text("${entry.label}: ${entry.value}", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                            TextButton(onClick = {
                                scope.launch {
                                    refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(entry.path) }
                                    revision++
                                }
                            }) { Text("Rotate") }
                        }
                    }
                }
                stats?.let { known ->
                    item(key = "statistics") {
                        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            known.warning.takeIf { it.isNotBlank() }?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.aircast.warning) }
                            Text("Statistics", style = MaterialTheme.typography.titleSmall)
                            one.aircast.mapspike.statisticsRows(known).forEach { (label, value) ->
                                Row(Modifier.fillMaxWidth()) {
                                    Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                                    Text(value.ifBlank { "\u2014" }, style = MaterialTheme.typography.bodyMedium)
                                }
                            }
                        }
                    }
                }
                presetKind(view)?.let { kind ->
                    item(key = "presets") { PatternPresets(index, kind) { revision++ } }
                }
                speedSection(view)?.let { speed ->
                    item(key = "speed") {
                        SpeedSectionRow(speed) { written ->
                            scope.launch {
                                refusal = withContext(Dispatchers.Default) { written() }
                                revision++
                            }
                        }
                    }
                }
            }
        }
    }

    if (editingPosition && positionStart != null) {
        EditPositionDialog(
            at = positionStart,
            onDismiss = { editingPosition = false },
            altitudeMode = view?.takeIf { !it.isNull("altitudeMode") }?.optInt("altitudeMode", -1),
            onAltitude = { shown ->
                scope.launch {
                    val set = withContext(Dispatchers.Default) { PlanBridge.setAltitude(index, shown) }
                    refusal = if (set) null else "The altitude could not be set."
                    revision++
                }
            },
            onMove = { latitude, longitude ->
                editingPosition = false
                scope.launch {
                    val moved = withContext(Dispatchers.Default) { PlanBridge.moveItem(index, latitude, longitude) }
                    refusal = if (moved) null else "The item could not be moved there."
                    revision++
                }
            },
        )
    }

    if (choosing) {
        CommandPicker(
            itemCategory = view?.optText("category")?.ifBlank { null },
            onDismiss = { choosing = false },
            onChosen = { command ->
                choosing = false
                scope.launch {
                    refusal = withContext(Dispatchers.Default) {
                        mapCentre?.let { (latitude, longitude) -> Qgc.refusalOf(mapCenterHintPath(index), JSONObject().put("latitude", latitude).put("longitude", longitude)) }
                        Qgc.writeRefusal(itemCommandPath(index), command)
                    }
                    revision++
                }
            },
        )
    }
}

@Composable
private fun CommandPicker(itemCategory: String?, onDismiss: () -> Unit, onChosen: (Int) -> Unit) {
    var categories by remember { mutableStateOf<List<String>>(emptyList()) }
    var category by remember { mutableStateOf<String?>(null) }
    var commands by remember { mutableStateOf<List<CommandChoice>>(emptyList()) }

    LaunchedEffect(Unit) {
        categories = withContext(Dispatchers.Default) { categoryNames(Qgc.invokeResult("missionCommandTree.categoriesForVehicle")) }
        category = startCategory(categories, itemCategory)
    }
    LaunchedEffect(category) {
        val chosen = category ?: return@LaunchedEffect
        commands = withContext(Dispatchers.Default) {
            commandChoices(Qgc.invokeResult("missionCommandTree.getCommandsForCategory", null, chosen, true))
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select mission command") },
        text = {
            Column {
                Row(Modifier.horizontalScroll(rememberScrollState())) {
                    categories.forEach { name ->
                        FilterChip(
                            selected = name == category,
                            onClick = { category = name },
                            label = { Text(sentenceCase(name)) },
                            modifier = Modifier.padding(end = 4.dp),
                        )
                    }
                }
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    items(commands, key = { it.id }) { command ->
                        ListItem(
                            headlineContent = { Text(sentenceCase(command.name)) },
                            supportingContent = { Text(command.description, style = MaterialTheme.typography.bodySmall) },
                            modifier = Modifier.clickable { onChosen(command.id) },
                        )
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

internal data class SpeedSection(val specified: Boolean, val value: Double?, val units: String, val path: String, val specifyPath: String)

internal fun speedSection(view: JSONObject?): SpeedSection? =
    view?.optJSONObject("speedSection")?.takeIf { it.optBoolean("available") }?.let {
        SpeedSection(
            specified = it.optBoolean("specified"),
            value = if (it.isNull("value")) null else it.optDouble("value"),
            units = it.optText("units"),
            path = it.optText("path"),
            specifyPath = it.optText("specifyPath"),
        )
    }

@Composable
private fun SpeedSectionRow(speed: SpeedSection, onWrite: (() -> String?) -> Unit) {
    var typed by remember(speed.value) { mutableStateOf(speed.value?.toString().orEmpty()) }
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Flight speed", Modifier.weight(1f))
            Switch(checked = speed.specified, onCheckedChange = { on -> onWrite { Qgc.writeRefusal(speed.specifyPath, on) } })
        }
        if (speed.specified) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typed = it },
                    label = { Text(listOf("Speed", speed.units).filter { it.isNotBlank() }.joinToString(" ")) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    modifier = Modifier.weight(1f),
                )
                TextButton(
                    enabled = one.aircast.mapspike.typedNumber(typed) != null,
                    onClick = { one.aircast.mapspike.typedNumber(typed)?.let { value -> onWrite { Qgc.writeRefusal(speed.path, value) } } },
                ) { Text("Set") }
            }
        }
    }
}

internal enum class CoordinateSystem(val label: String) {
    Geographic("Geographic"),
    Utm("Universal Transverse Mercator"),
    Mgrs("Military Grid Reference"),
    Vehicle("Vehicle Position"),
}

internal data class PositionForms(val zone: String, val southern: Boolean, val easting: String, val northing: String, val mgrs: String)

internal fun positionForms(view: JSONObject?): PositionForms? = view?.let {
    val utm = it.optJSONObject("utm")
    PositionForms(
        zone = utm?.optInt("zone")?.toString().orEmpty(),
        southern = utm?.optBoolean("southern") ?: false,
        easting = utm?.optDouble("easting")?.let { e -> String.format(java.util.Locale.US, "%.2f", e) }.orEmpty(),
        northing = utm?.optDouble("northing")?.let { n -> String.format(java.util.Locale.US, "%.2f", n) }.orEmpty(),
        mgrs = it.optText("mgrs"),
    )
}

internal fun positionFormsPath(at: TrackPoint): String =
    String.format(java.util.Locale.US, "view.positionForms(%.8f,%.8f)", at.latitude, at.longitude)

internal fun utmToGeoPath(easting: String, northing: String, zone: String, southern: Boolean): String =
    "view.utmToGeo($easting,$northing,$zone,$southern)"

internal fun geoOf(view: JSONObject?): Pair<Double, Double>? =
    view?.takeIf { it.optBoolean("valid") }?.let { it.optDouble("latitude") to it.optDouble("longitude") }

@Composable
internal fun EditPositionDialog(
    at: TrackPoint,
    onDismiss: () -> Unit,
    title: String = "Edit position",
    confirm: String = "Move",
    vehicleNote: String = "Move the item to the vehicle's current position.",
    altitudeMode: Int? = null,
    onAltitude: ((Double) -> Unit)? = null,
    onMove: (Double, Double) -> Unit,
) {
    val altitudePath = vehicleAltitudePath(altitudeMode).takeIf { onAltitude != null }
    var setPosition by remember { mutableStateOf(true) }
    var setAltitude by remember { mutableStateOf(false) }
    var system by remember { mutableStateOf(CoordinateSystem.Geographic) }
    var latitude by remember { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.latitude)) }
    var longitude by remember { mutableStateOf(String.format(java.util.Locale.US, "%.7f", at.longitude)) }
    var forms by remember { mutableStateOf<PositionForms?>(null) }
    var zone by remember { mutableStateOf("") }
    var southern by remember { mutableStateOf(false) }
    var easting by remember { mutableStateOf("") }
    var northing by remember { mutableStateOf("") }
    var mgrs by remember { mutableStateOf("") }
    var problem by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(at) {
        forms = withContext(Dispatchers.Default) { positionForms(Qgc.get(positionFormsPath(at))) }
        forms?.let {
            zone = it.zone
            southern = it.southern
            easting = it.easting
            northing = it.northing
            mgrs = it.mgrs
        }
    }

    fun fromVehicle() {
        scope.launch {
            val (position, altitude) = withContext(Dispatchers.Default) {
                geoOf(Qgc.get("vehicle.coordinate")).takeIf { setPosition } to
                    altitudePath?.takeIf { setAltitude }?.let { Qgc.get(it).optDouble("value", Double.NaN) }?.takeIf { it.isFinite() }
            }
            altitude?.let { onAltitude?.invoke(it) }
            when {
                setPosition && position == null -> problem = "That position could not be read."
                position != null -> onMove(position.first, position.second)
                else -> onDismiss()
            }
        }
    }

    fun move() {
        if (system == CoordinateSystem.Vehicle) return fromVehicle()
        scope.launch {
            val target = withContext(Dispatchers.Default) {
                when (system) {
                    CoordinateSystem.Geographic -> one.aircast.mapspike.typedNumber(latitude)?.let { lat -> one.aircast.mapspike.typedNumber(longitude)?.let { lat to it } }
                    CoordinateSystem.Utm -> geoOf(Qgc.get(utmToGeoPath(easting, northing, zone, southern)))
                    CoordinateSystem.Mgrs -> geoOf(Qgc.get("view.mgrsToGeo(${mgrs.replace(" ", "")})"))
                    CoordinateSystem.Vehicle -> geoOf(Qgc.get("vehicle.coordinate"))
                }
            }
            if (target == null) problem = "That position could not be read." else onMove(target.first, target.second)
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column {
                Row(Modifier.horizontalScroll(rememberScrollState())) {
                    val connected = hasVehicle()
                    CoordinateSystem.entries.filter { it != CoordinateSystem.Vehicle || connected }.forEach { choice ->
                        FilterChip(selected = choice == system, onClick = { system = choice; problem = null }, label = { Text(choice.label) }, modifier = Modifier.padding(end = 4.dp))
                    }
                }
                when (system) {
                    CoordinateSystem.Geographic -> {
                        PositionField("Latitude", latitude) { latitude = it }
                        PositionField("Longitude", longitude) { longitude = it }
                    }
                    CoordinateSystem.Utm -> {
                        PositionField("Zone", zone) { zone = it }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("Southern hemisphere", Modifier.weight(1f))
                            Switch(checked = southern, onCheckedChange = { southern = it })
                        }
                        PositionField("Easting", easting) { easting = it }
                        PositionField("Northing", northing) { northing = it }
                    }
                    CoordinateSystem.Mgrs -> PositionField("MGRS", mgrs, KeyboardType.Text) { mgrs = it }
                    CoordinateSystem.Vehicle -> {
                        Text(vehicleNote)
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Checkbox(checked = setPosition, onCheckedChange = { setPosition = it })
                            Text("Set position from vehicle")
                        }
                        if (altitudePath != null) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Checkbox(checked = setAltitude, onCheckedChange = { setAltitude = it })
                                Text("Set altitude from vehicle")
                            }
                        }
                    }
                }
                problem?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = { move() }) { Text(confirm) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

internal fun vehicleAltitudePath(altitudeMode: Int?): String? = when (altitudeMode) {
    1 -> "vehicle.altitudeRelative"
    2 -> "vehicle.altitudeAMSL"
    3, 4 -> "vehicle.altitudeAboveTerr"
    else -> null
}

@Composable
private fun PositionField(label: String, value: String, keyboard: KeyboardType = KeyboardType.Text, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = keyboard),
        modifier = Modifier.fillMaxWidth(),
    )
}

internal fun itemEditorTitle(view: JSONObject?, index: Int): String {
    val name = view?.optText("commandName")?.ifBlank { null }
    val sequence = view?.takeIf { it.has("sequenceNumber") && !it.isNull("sequenceNumber") }?.optInt("sequenceNumber")
    return listOfNotNull(sequence?.let { "#$it" }, name).joinToString(" ").ifBlank { "Item $index" }
}
