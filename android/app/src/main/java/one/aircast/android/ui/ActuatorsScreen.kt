package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.abs

internal const val ACTUATOR_OUTPUTS_VIEW = "view.actuatorOutputs"
internal const val ACTUATORS_SCREEN = "actuators"
private const val SHOW_BITSET = "bitset"
private const val SHOW_TRUE_IF_POSITIVE = "true-if-positive"

internal data class ActuatorFact(val label: String, val showAs: String, val bit: Int, val advanced: Boolean, val fact: Fact)
internal data class ActuatorColumn(val label: String, val advanced: Boolean, val visible: Boolean)
internal data class ActuatorChannel(val label: String, val configs: List<ActuatorFact?>)
internal data class ActuatorSubgroup(val label: String, val primary: ActuatorFact?, val params: List<ActuatorFact>, val columns: List<ActuatorColumn>, val channels: List<ActuatorChannel>)
internal data class ActuatorGroup(val label: String, val enable: ActuatorFact?, val groupsVisible: Boolean, val params: List<ActuatorFact>, val subgroups: List<ActuatorSubgroup>, val notes: List<String> = emptyList())
internal sealed interface GeometryCell {
    val hidden: Boolean
    data class Editable(val item: ActuatorFact, val param: String, val channelFunction: Int, override val hidden: Boolean, val disabled: Boolean) : GeometryCell
    data class Fixed(val label: String, val valueString: String, val advanced: Boolean, override val hidden: Boolean = false) : GeometryCell
    data class Axis(val options: List<String>, val index: Int, val params: List<String>, val advanced: Boolean, override val hidden: Boolean, val disabled: Boolean) : GeometryCell
    data class Unavailable(val label: String, val advanced: Boolean, override val hidden: Boolean) : GeometryCell
}

internal const val PARAM_NOT_AVAILABLE = "(Param not available)"

internal const val ACTUATOR_MIXER_SET = "actuatorMixer.set"
internal const val ACTUATOR_MIXER_AXIS = "actuatorMixer.setAxis"
internal data class GeometryChannel(val label: String, val cells: List<GeometryCell?>)
internal data class GeometryGroup(val label: String, val count: Fact?, val channels: List<GeometryChannel>, val params: List<ActuatorFact>)
internal data class Geometry(val title: String, val helpUrl: String, val groups: List<GeometryGroup>, val motors: List<GeometryMotor> = emptyList())
internal data class MotorAssignmentState(
    val multirotor: Boolean = false,
    val enabled: Boolean = false,
    val active: Boolean = false,
    val message: String = "",
    val highlighted: Set<Int> = emptySet(),
)

internal const val MOTOR_ASSIGNMENT_INIT = "motorAssignment.init"
internal const val MOTOR_ASSIGNMENT_START = "motorAssignment.start"
internal const val MOTOR_ASSIGNMENT_SELECT = "motorAssignment.selectMotor"
internal const val MOTOR_ASSIGNMENT_SPIN = "motorAssignment.spinCurrentMotor"
internal const val MOTOR_ASSIGNMENT_ABORT = "motorAssignment.abort"
private const val ASSIGNMENT_POLL_MS = 300L
private val TAG = Regex("<[^>]+>")

internal fun plainMessage(html: String): String = html.replace("<br />", "\n").replace(TAG, "").lines().joinToString("\n") { it.trimEnd() }

internal fun motorAssignment(json: JSONObject?): MotorAssignmentState = json?.let { read ->
    val highlighted = read.optJSONArray("highlighted")
    MotorAssignmentState(
        multirotor = read.optBoolean("multirotor"),
        enabled = read.optBoolean("enabled"),
        active = read.optBoolean("active"),
        message = read.optText("message"),
        highlighted = (0 until (highlighted?.length() ?: 0)).map { highlighted!!.optInt(it) }.toSet(),
    )
} ?: MotorAssignmentState()

internal data class ActuatorOutputs(
    val available: Boolean,
    val reason: String,
    val showUi: Boolean,
    val groups: List<ActuatorGroup>,
    val testing: ActuatorTesting? = null,
    val geometry: Geometry? = null,
    val hasUnsetRequiredFunctions: Boolean = false,
    val assignment: MotorAssignmentState = MotorAssignmentState(),
    val actions: List<ActuatorActionGroup> = emptyList(),
)

private fun <T> JSONArray?.objects(read: (JSONObject) -> T?): List<T> =
    (0 until (this?.length() ?: 0)).mapNotNull { index -> this?.optJSONObject(index)?.let(read) }

private fun actuatorFact(json: JSONObject?): ActuatorFact? =
    json?.let { control ->
        factFromControl(control)?.let { ActuatorFact(control.optText("label"), control.optText("showAs"), control.optInt("bit"), control.optBoolean("advanced"), it) }
    }

private fun strings(json: JSONObject, key: String): List<String> =
    json.optJSONArray(key)?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty()

internal fun geometryCell(json: JSONObject?): GeometryCell? = when {
    json == null -> null
    json.optBoolean("axis") -> GeometryCell.Axis(strings(json, "options"), json.optInt("index"), strings(json, "params"), json.optBoolean("advanced"), json.optBoolean("hidden"), json.optBoolean("disabled"))
    json.optBoolean("unavailable") -> GeometryCell.Unavailable(json.optText("label"), json.optBoolean("advanced"), json.optBoolean("hidden"))
    json.optBoolean("fixed") -> GeometryCell.Fixed(json.optText("label"), json.optText("valueString"), json.optBoolean("advanced"), json.optBoolean("hidden"))
    else -> actuatorFact(json)?.let { GeometryCell.Editable(it, json.optText("param"), json.optInt("channelFunction"), json.optBoolean("hidden"), json.optBoolean("disabled")) }
}

internal fun geometry(json: JSONObject?): Geometry? = json?.let { read ->
    Geometry(
        title = read.optText("title"),
        helpUrl = read.optText("helpUrl"),
        motors = geometryMotors(read),
        groups = read.optJSONArray("groups").objects { group ->
            GeometryGroup(
                label = group.optText("label"),
                count = group.optJSONObject("count")?.let(::factFromControl),
                channels = group.optJSONArray("channels").objects { channel ->
                    val cells = channel.optJSONArray("cells")
                    GeometryChannel(channel.optText("label"), (0 until (cells?.length() ?: 0)).map { geometryCell(cells?.optJSONObject(it)) })
                },
                params = group.optJSONArray("params").objects(::actuatorFact),
            )
        },
    )
}

internal fun actuatorOutputs(view: JSONObject?): ActuatorOutputs? =
    view?.takeIf { it.optString("class") == "ActuatorOutputs" }?.let { read ->
        ActuatorOutputs(
            available = read.optBoolean("available"),
            reason = read.optText("reason"),
            showUi = read.optBoolean("showUi", true),
            testing = actuatorTesting(read),
            geometry = geometry(read.optJSONObject("geometry")),
            hasUnsetRequiredFunctions = read.optBoolean("hasUnsetRequiredFunctions"),
            assignment = motorAssignment(read.optJSONObject("motorAssignment")),
            actions = actuatorActions(read),
            groups = read.optJSONArray("groups").objects { group ->
                ActuatorGroup(
                    label = group.optText("label"),
                    enable = actuatorFact(group.optJSONObject("enable")),
                    groupsVisible = group.optBoolean("groupsVisible"),
                    notes = group.optJSONArray("notes").let { list -> (0 until (list?.length() ?: 0)).map { list!!.optString(it) } },
                    params = group.optJSONArray("params").objects(::actuatorFact),
                    subgroups = group.optJSONArray("subgroups").objects { subgroup ->
                        val configs = { channel: JSONObject -> channel.optJSONArray("configs").let { list -> (0 until (list?.length() ?: 0)).map { actuatorFact(list?.optJSONObject(it)) } } }
                        ActuatorSubgroup(
                            label = subgroup.optText("label"),
                            primary = actuatorFact(subgroup.optJSONObject("primary")),
                            params = subgroup.optJSONArray("params").objects(::actuatorFact),
                            columns = subgroup.optJSONArray("columns").objects { ActuatorColumn(it.optText("label"), it.optBoolean("advanced"), it.optBoolean("visible")) },
                            channels = subgroup.optJSONArray("channels").objects { ActuatorChannel(it.optText("label"), configs(it)) },
                        )
                    },
                )
            },
        )
    }

internal fun rawNumber(fact: Fact): Double? = (fact.value as? Number)?.toDouble() ?: fact.valueString.toDoubleOrNull()

internal fun bitsetChecked(raw: Double, bit: Int): Boolean = (raw.toLong() and (1L shl bit)) != 0L

internal fun bitsetWritten(raw: Double, bit: Int, on: Boolean): Long =
    if (on) raw.toLong() or (1L shl bit) else raw.toLong() and (1L shl bit).inv()

internal fun signWritten(raw: Double, on: Boolean): Double = if (on) abs(raw) else -abs(raw)

@Composable
fun ActuatorsScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<ActuatorOutputs?>(null) }
    var tab by remember { mutableIntStateOf(0) }
    var advanced by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var testing by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf<String?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { actuatorOutputs(Qgc.get(ACTUATOR_OUTPUTS_VIEW)) }
    }
    LaunchedEffect(read?.assignment?.active) {
        while (read?.assignment?.active == true) {
            kotlinx.coroutines.delay(ASSIGNMENT_POLL_MS)
            revision++
        }
    }
    val outputs = read ?: run {
        Text("Reading actuator metadata.", modifier.padding(16.dp))
        return
    }
    if (!outputs.available || !outputs.showUi) {
        Text(outputs.reason.ifBlank { "This vehicle does not configure its actuators here." }, modifier.padding(16.dp))
        return
    }

    fun write(path: String, value: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(path, value) }
            revision++
        }
    }

    val group = outputs.groups.getOrNull(tab) ?: outputs.groups.firstOrNull() ?: return
    Column(modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
            Spacer(Modifier.weight(1f))
            Checkbox(checked = advanced, onCheckedChange = { advanced = it })
            Text("Advanced")
        }
        outputs.geometry?.let { geometry ->
            Column(Modifier.padding(16.dp)) {
                GeometrySection(geometry, advanced, mixerEditable(testing, outputs.assignment.active), ::write, outputs.assignment.highlighted, onMotor = { motor ->
                    scope.launch {
                        withContext(Dispatchers.IO) { Qgc.invoke(MOTOR_ASSIGNMENT_SELECT, motor) }
                        revision++
                    }
                }) { revision++ }
            }
        }
        outputs.testing?.let { Column(Modifier.padding(16.dp)) { ActuatorTestSection(it, outputs.actions, testing, outputs.assignment.active) { on -> testing = on } } }
        Text("Actuator outputs", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 16.dp))
        if (outputs.hasUnsetRequiredFunctions) {
            Text("One or more actuator still needs to be assigned to an output.", color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 16.dp))
        }
        if (outputs.assignment.multirotor) {
            Row(Modifier.padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (!outputs.assignment.active && group.groupsVisible) {
                    androidx.compose.material3.Button(
                        onClick = {
                            scope.launch {
                                val refused = withContext(Dispatchers.IO) { Qgc.refusalOf(MOTOR_ASSIGNMENT_INIT, outputs.groups.indexOf(group)) }
                                val message = withContext(Dispatchers.Default) { actuatorOutputs(Qgc.get(ACTUATOR_OUTPUTS_VIEW))?.assignment?.message.orEmpty() }
                                if (refused == null) confirming = plainMessage(message) else failure = plainMessage(refused)
                            }
                        },
                        enabled = outputs.assignment.enabled && !testing,
                    ) { Text("Identify & Assign Motors") }
                }
                if (outputs.assignment.active) {
                    androidx.compose.material3.OutlinedButton(onClick = { scope.launch(Dispatchers.IO) { Qgc.invoke(MOTOR_ASSIGNMENT_SPIN) } }) { Text("Spin motor again") }
                    androidx.compose.material3.OutlinedButton(onClick = {
                        scope.launch {
                            withContext(Dispatchers.IO) { Qgc.invoke(MOTOR_ASSIGNMENT_ABORT) }
                            revision++
                        }
                    }) { Text("Abort") }
                }
            }
        }
        confirming?.let { message ->
            androidx.compose.material3.AlertDialog(
                onDismissRequest = { confirming = null },
                title = { Text("Motor order identification and assignment") },
                text = { Text(message) },
                confirmButton = {
                    androidx.compose.material3.TextButton(onClick = {
                        confirming = null
                        scope.launch {
                            withContext(Dispatchers.IO) { Qgc.invoke(MOTOR_ASSIGNMENT_START) }
                            revision++
                        }
                    }) { Text("Yes") }
                },
                dismissButton = { androidx.compose.material3.TextButton(onClick = { confirming = null }) { Text("No") } },
            )
        }
        failure?.let { message ->
            androidx.compose.material3.AlertDialog(
                onDismissRequest = { failure = null },
                title = { Text("Error") },
                text = { Text(message) },
                confirmButton = { androidx.compose.material3.TextButton(onClick = { failure = null }) { Text("Ok") } },
            )
        }
        ScrollableTabRow(selectedTabIndex = outputs.groups.indexOf(group)) {
            outputs.groups.forEachIndexed { index, each -> Tab(selected = each == group, onClick = { tab = index }, text = { Text(each.label) }) }
        }
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            group.enable?.let { ActuatorFactRow(it, ::write) { revision++ } }
            if (group.groupsVisible) {
                group.subgroups.forEach { subgroup ->
                    if (subgroup.label.isNotEmpty()) Text(subgroup.label, style = MaterialTheme.typography.titleSmall)
                    subgroup.primary?.let { ActuatorFactRow(it, ::write) { revision++ } }
                    subgroup.channels.forEach { channel ->
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(8.dp)) {
                                Text(channel.label, style = MaterialTheme.typography.labelLarge)
                                channel.configs.forEachIndexed { index, config ->
                                    val column = subgroup.columns.getOrNull(index)
                                    if (column != null && column.visible && (advanced || !column.advanced)) {
                                        config?.let { ActuatorFactRow(it.copy(label = column.label), ::write) { revision++ } } ?: NotAvailableRow(column.label)
                                    }
                                }
                            }
                        }
                    }
                    subgroup.params.forEach { ActuatorFactRow(it, ::write) { revision++ } }
                }
            }
            group.params.forEach { ActuatorFactRow(it, ::write) { revision++ } }
            group.notes.forEach { Text(it, style = MaterialTheme.typography.bodySmall) }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }
}

internal fun mixerEditable(testing: Boolean, assigning: Boolean): Boolean = !testing && !assigning

@Composable
private fun GeometrySection(geometry: Geometry, advanced: Boolean, editable: Boolean, write: (String, Any) -> Unit, highlighted: Set<Int> = emptySet(), onMotor: (Int) -> Unit = {}, onWrite: () -> Unit) {
    val uri = androidx.compose.ui.platform.LocalUriHandler.current
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(geometry.title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            if (geometry.helpUrl.isNotEmpty()) androidx.compose.material3.TextButton(onClick = { uri.openUri(geometry.helpUrl) }) { Text("?") }
        }
        if (geometry.motors.size > 1) GeometryImage(geometry.motors, highlighted = highlighted, onMotor = onMotor)
        androidx.compose.foundation.layout.Box {
            Column(Modifier.alpha(if (editable) 1f else DISABLED_ALPHA), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                geometry.groups.forEach { group ->
                    Text(group.label, style = MaterialTheme.typography.titleSmall)
                    group.count?.let { FactRow(it, onWrite = onWrite) }
                    group.channels.forEach { channel ->
                        OutlinedCard(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(8.dp)) {
                                Text(channel.label, style = MaterialTheme.typography.labelLarge)
                                channel.cells.filterNotNull().filter { !it.hidden }.forEach { cell ->
                                    when (cell) {
                                        is GeometryCell.Editable -> if (advanced || !cell.item.advanced) MixerCellRow(cell, onWrite)
                                        is GeometryCell.Fixed -> if (advanced || !cell.advanced) Row {
                                            Text(cell.label, modifier = Modifier.weight(1f))
                                            Text(cell.valueString)
                                        }
                                        is GeometryCell.Axis -> if (advanced || !cell.advanced) AxisRow(cell, onWrite)
                                        is GeometryCell.Unavailable -> if (advanced || !cell.advanced) NotAvailableRow(cell.label)
                                    }
                                }
                            }
                        }
                    }
                    group.params.forEach { if (advanced || !it.advanced) ActuatorFactRow(it, write, onWrite) }
                }
            }
            if (!editable) androidx.compose.foundation.layout.Box(Modifier.matchParentSize().swallowTouches())
        }
    }
}

private const val DISABLED_ALPHA = 0.38f

@Composable
private fun NotAvailableRow(label: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Text(PARAM_NOT_AVAILABLE)
    }
}

@Composable
private fun MixerCellRow(cell: GeometryCell.Editable, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    var typed by remember(cell.item.fact.valueString) { mutableStateOf(cell.item.fact.valueString) }
    fun set(value: Any) {
        scope.launch {
            withContext(Dispatchers.IO) { Qgc.invoke(ACTUATOR_MIXER_SET, cell.param, value, cell.channelFunction) }
            onWrite()
        }
    }
    val raw = rawNumber(cell.item.fact)
    when {
        cell.item.showAs == SHOW_TRUE_IF_POSITIVE && raw != null -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(cell.item.label, modifier = Modifier.weight(1f))
            Checkbox(checked = raw > 0.0, enabled = !cell.disabled, onCheckedChange = { set(signWritten(raw, it)) })
        }
        cell.item.showAs == SHOW_BITSET && raw != null -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(cell.item.label, modifier = Modifier.weight(1f))
            Checkbox(checked = bitsetChecked(raw, cell.item.bit), enabled = !cell.disabled, onCheckedChange = { set(bitsetWritten(raw, cell.item.bit, it)) })
        }
        cell.item.fact.enumStrings.isNotEmpty() -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(cell.item.label, modifier = Modifier.weight(1f))
            var open by remember { mutableStateOf(false) }
            androidx.compose.foundation.layout.Box {
                androidx.compose.material3.OutlinedButton(onClick = { open = true }, enabled = !cell.disabled) { Text(cell.item.fact.valueString) }
                androidx.compose.material3.DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                    cell.item.fact.enumStrings.forEachIndexed { index, label ->
                        androidx.compose.material3.DropdownMenuItem(text = { Text(label) }, onClick = {
                            open = false
                            cell.item.fact.enumValues.getOrNull(index)?.toDoubleOrNull()?.let(::set)
                        })
                    }
                }
            }
        }
        else -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(cell.item.label, modifier = Modifier.weight(1f))
            androidx.compose.material3.OutlinedTextField(
                value = typed,
                onValueChange = { typed = it },
                enabled = !cell.disabled,
                singleLine = true,
                suffix = { Text(cell.item.fact.units) },
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = androidx.compose.ui.text.input.KeyboardType.Decimal),
                keyboardActions = androidx.compose.foundation.text.KeyboardActions(onDone = { typed.toDoubleOrNull()?.let(::set) }),
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun AxisRow(cell: GeometryCell.Axis, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    var open by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text("Axis", modifier = Modifier.weight(1f))
        androidx.compose.foundation.layout.Box {
            androidx.compose.material3.OutlinedButton(onClick = { open = true }, enabled = !cell.disabled) { Text(cell.options.getOrElse(cell.index) { "" }) }
            androidx.compose.material3.DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                cell.options.forEachIndexed { index, label ->
                    androidx.compose.material3.DropdownMenuItem(text = { Text(label) }, onClick = {
                        open = false
                        scope.launch {
                            withContext(Dispatchers.IO) { Qgc.invoke(ACTUATOR_MIXER_AXIS, org.json.JSONArray(cell.params), index) }
                            onWrite()
                        }
                    })
                }
            }
        }
    }
}

@Composable
private fun ActuatorFactRow(item: ActuatorFact, write: (String, Any) -> Unit, onWrite: () -> Unit) {
    val raw = rawNumber(item.fact)
    when {
        item.showAs == SHOW_BITSET && raw != null -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(item.label, modifier = Modifier.weight(1f))
            Checkbox(checked = bitsetChecked(raw, item.bit), onCheckedChange = { write(item.fact.path, bitsetWritten(raw, item.bit, it)) })
        }
        item.showAs == SHOW_TRUE_IF_POSITIVE && raw != null -> Row(verticalAlignment = Alignment.CenterVertically) {
            Text(item.label, modifier = Modifier.weight(1f))
            Checkbox(checked = raw > 0.0, onCheckedChange = { write(item.fact.path, signWritten(raw, it)) })
        }
        else -> Column {
            FactRow(item.fact, title = item.label.ifEmpty { item.fact.title }, onWrite = onWrite)
            HorizontalDivider()
        }
    }
}
