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
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
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
    data class Editable(val item: ActuatorFact) : GeometryCell
    data class Fixed(val label: String, val valueString: String, val advanced: Boolean) : GeometryCell
}
internal data class GeometryChannel(val label: String, val cells: List<GeometryCell?>)
internal data class GeometryGroup(val label: String, val count: Fact?, val channels: List<GeometryChannel>, val params: List<ActuatorFact>)
internal data class Geometry(val title: String, val helpUrl: String, val groups: List<GeometryGroup>)
internal data class ActuatorOutputs(
    val available: Boolean,
    val reason: String,
    val showUi: Boolean,
    val groups: List<ActuatorGroup>,
    val testing: ActuatorTesting? = null,
    val geometry: Geometry? = null,
    val hasUnsetRequiredFunctions: Boolean = false,
)

private fun <T> JSONArray?.objects(read: (JSONObject) -> T?): List<T> =
    (0 until (this?.length() ?: 0)).mapNotNull { index -> this?.optJSONObject(index)?.let(read) }

private fun actuatorFact(json: JSONObject?): ActuatorFact? =
    json?.let { control ->
        factFromControl(control)?.let { ActuatorFact(control.optText("label"), control.optText("showAs"), control.optInt("bit"), control.optBoolean("advanced"), it) }
    }

private fun geometryCell(json: JSONObject?): GeometryCell? = when {
    json == null -> null
    json.optBoolean("fixed") -> GeometryCell.Fixed(json.optText("label"), json.optText("valueString"), json.optBoolean("advanced"))
    else -> actuatorFact(json)?.let { GeometryCell.Editable(it) }
}

internal fun geometry(json: JSONObject?): Geometry? = json?.let { read ->
    Geometry(
        title = read.optText("title"),
        helpUrl = read.optText("helpUrl"),
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
    val scope = rememberCoroutineScope()

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { actuatorOutputs(Qgc.get(ACTUATOR_OUTPUTS_VIEW)) }
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
        outputs.geometry?.let { Column(Modifier.padding(16.dp)) { GeometrySection(it, advanced, ::write) { revision++ } } }
        outputs.testing?.let { Column(Modifier.padding(16.dp)) { ActuatorTestSection(it) } }
        Text("Actuator Outputs", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 16.dp))
        if (outputs.hasUnsetRequiredFunctions) {
            Text("One or more actuator still needs to be assigned to an output.", color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 16.dp))
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
                                    if (config != null && column != null && column.visible && (advanced || !column.advanced)) {
                                        ActuatorFactRow(config.copy(label = column.label), ::write) { revision++ }
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

@Composable
private fun GeometrySection(geometry: Geometry, advanced: Boolean, write: (String, Any) -> Unit, onWrite: () -> Unit) {
    val uri = androidx.compose.ui.platform.LocalUriHandler.current
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(geometry.title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            if (geometry.helpUrl.isNotEmpty()) androidx.compose.material3.TextButton(onClick = { uri.openUri(geometry.helpUrl) }) { Text("?") }
        }
        geometry.groups.forEach { group ->
            Text(group.label, style = MaterialTheme.typography.titleSmall)
            group.count?.let { FactRow(it, onWrite = onWrite) }
            group.channels.forEach { channel ->
                OutlinedCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(8.dp)) {
                        Text(channel.label, style = MaterialTheme.typography.labelLarge)
                        channel.cells.forEach { cell ->
                            when (cell) {
                                is GeometryCell.Editable -> if (advanced || !cell.item.advanced) ActuatorFactRow(cell.item, write, onWrite)
                                is GeometryCell.Fixed -> if (advanced || !cell.advanced) Row {
                                    Text(cell.label, modifier = Modifier.weight(1f))
                                    Text(cell.valueString)
                                }
                                null -> Unit
                            }
                        }
                    }
                }
            }
            group.params.forEach { if (advanced || !it.advanced) ActuatorFactRow(it, write, onWrite) }
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
