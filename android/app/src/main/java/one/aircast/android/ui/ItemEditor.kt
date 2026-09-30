package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
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

internal fun itemFactsPath(index: Int): String = "view.itemFacts($index)"

internal fun itemCommandPath(index: Int): String = "plan.missionController.visualItems.$index.command"

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

internal fun itemFields(view: JSONObject?): List<one.aircast.android.bridge.Fact> {
    val listed = view?.optJSONArray("fields") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it)?.let(::factFromControl) }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ItemEditor(index: Int, at: TrackPoint?, onDismiss: () -> Unit) {
    var revision by remember(index) { mutableIntStateOf(0) }
    var view by remember(index) { mutableStateOf<JSONObject?>(null) }
    var choosing by remember(index) { mutableStateOf(false) }
    var editingPosition by remember(index) { mutableStateOf(false) }
    var refusal by remember(index) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(index, revision) {
        view = withContext(Dispatchers.Default) { Qgc.get(itemFactsPath(index)) }
    }

    val fields = remember(view) { itemFields(view) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Item ${index}", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                if (at != null) {
                    TextButton(onClick = { editingPosition = true }) { Text("Edit position") }
                }
                if (view?.optBoolean("simple") == true) {
                    TextButton(onClick = { choosing = true }) { Text("Change command") }
                }
            }
            refusal?.let {
                Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 20.dp))
            }
            LazyColumn(Modifier.heightIn(max = 480.dp)) {
                items(fields, key = { it.path }) { fact ->
                    FactRow(fact) { revision++ }
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

    if (editingPosition && at != null) {
        EditPositionDialog(
            at = at,
            onDismiss = { editingPosition = false },
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
            onDismiss = { choosing = false },
            onChosen = { command ->
                choosing = false
                scope.launch {
                    refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(itemCommandPath(index), command) }
                    revision++
                }
            },
        )
    }
}

@Composable
private fun CommandPicker(onDismiss: () -> Unit, onChosen: (Int) -> Unit) {
    var categories by remember { mutableStateOf<List<String>>(emptyList()) }
    var category by remember { mutableStateOf<String?>(null) }
    var commands by remember { mutableStateOf<List<CommandChoice>>(emptyList()) }

    LaunchedEffect(Unit) {
        categories = withContext(Dispatchers.Default) { categoryNames(Qgc.invokeResult("missionCommandTree.categoriesForVehicle")) }
        category = categories.firstOrNull()
    }
    LaunchedEffect(category) {
        val chosen = category ?: return@LaunchedEffect
        commands = withContext(Dispatchers.Default) {
            commandChoices(Qgc.invokeResult("missionCommandTree.getCommandsForCategory", null, chosen, true))
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select Mission Command") },
        text = {
            Column {
                Row(Modifier.horizontalScroll(rememberScrollState())) {
                    categories.forEach { name ->
                        FilterChip(
                            selected = name == category,
                            onClick = { category = name },
                            label = { Text(name) },
                            modifier = Modifier.padding(end = 4.dp),
                        )
                    }
                }
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    items(commands, key = { it.id }) { command ->
                        ListItem(
                            headlineContent = { Text(command.name) },
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
                    enabled = typed.toDoubleOrNull() != null,
                    onClick = { typed.toDoubleOrNull()?.let { value -> onWrite { Qgc.writeRefusal(speed.path, value) } } },
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
private fun EditPositionDialog(at: TrackPoint, onDismiss: () -> Unit, onMove: (Double, Double) -> Unit) {
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

    fun move() {
        scope.launch {
            val target = withContext(Dispatchers.Default) {
                when (system) {
                    CoordinateSystem.Geographic -> latitude.toDoubleOrNull()?.let { lat -> longitude.toDoubleOrNull()?.let { lat to it } }
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
        title = { Text("Edit Position") },
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
                    CoordinateSystem.Vehicle -> Text("Move the item to the vehicle's current position.")
                }
                problem?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = { move() }) { Text("Move") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun PositionField(label: String, value: String, keyboard: KeyboardType = KeyboardType.Decimal, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = keyboard),
        modifier = Modifier.fillMaxWidth(),
    )
}
