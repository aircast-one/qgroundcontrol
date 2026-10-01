package one.aircast.android.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText

internal data class ParameterRows(
    val title: String,
    val facts: List<Fact>,
    val note: String,
    val calculators: Map<String, PowerCalculator> = emptyMap(),
)

internal fun factFromParameter(name: String, json: JSONObject): Fact? =
    if (json.optText("kind") == "fact") {
        Qgc.factAt(parameterPath(name), json).copy(name = name)
    } else {
        null
    }

internal fun readOnlyNote(facts: List<Fact>): String? {
    val locked = facts.filter { it.readOnly }
    return when {
        locked.isEmpty() -> null
        locked.size == facts.size ->
            "This firmware reports all of these as read-only, so they are shown for reference."
        else ->
            "This firmware reports " + locked.joinToString(", ") { it.name } +
                " as read-only, so they are shown but cannot be changed here."
    }
}

internal fun factFromControl(control: JSONObject): Fact? {
    val label = control.optText("label")
    val name = control.optText("name")
    if (label.isBlank() && name.isBlank()) return null
    val options = control.optJSONArray("options")
    val labels = (0 until (options?.length() ?: 0)).map { options!!.optJSONObject(it).optText("label") }
    val bits = control.optJSONArray("bits").takeIf { control.optText("control") == "bitmask" }
    val bitEntries = (0 until (bits?.length() ?: 0)).mapNotNull { index ->
        bits!!.optJSONObject(index)?.let { bit ->
            bit.optText("raw").toLongOrNull()?.let { raw -> bit.optText("label") to raw }
        }
    }
    return Fact(
        path = control.optText("path"),
        name = name,
        description = label,
        units = control.optText("units"),
        valueString = control.optText("valueString"),
        value = control.opt("value"),
        wholeNumbersOnly = control.optBoolean("wholeNumbersOnly"),
        enumStrings = labels,
        enumIndex = labels.indexOf(control.optText("display")),
        bitmaskStrings = bitEntries.map { it.first },
        bitmaskValues = bitEntries.map { it.second },
        controlKind = control.optText("control"),
        isBool = control.optText("control") == "toggle",
        isString = control.optText("control") == "text",
        readOnly = control.optBoolean("readOnly"),
        enabled = control.optBoolean("enabled", true),
        disabledReason = control.optText("disabledReason"),
        vehicleRebootRequired = control.optBoolean("rebootRequired"),
        minString = control.optText("minimumText"),
        maxString = control.optText("maximumText"),
        minIsDefaultForType = control.isNull("minimumText"),
        maxIsDefaultForType = control.isNull("maximumText"),
        defaultValueString = control.optText("defaultText"),
        qgcRebootRequired = control.optBoolean("applicationRestartRequired"),
        warning = control.optBoolean("warning"),
        optional = control.optBoolean("optional"),
    )
}

private fun readPage(page: String): List<ParameterRows> {
    val sections = Qgc.get(setupPagePath(page)).optJSONArray("sections") ?: return emptyList()
    return (0 until sections.length()).mapNotNull { index ->
        sections.optJSONObject(index)?.let { section ->
            val controls = section.optJSONArray("controls")
            val facts = (0 until (controls?.length() ?: 0)).mapNotNull { control ->
                controls!!.optJSONObject(control)?.let(::factFromControl)
            }
            if (facts.isEmpty()) {
                null
            } else {
                val note = listOfNotNull(
                    section.optText("note").ifBlank { null },
                    readOnlyNote(facts),
                )
                val calculators = (0 until (controls?.length() ?: 0)).mapNotNull { control ->
                    controls!!.optJSONObject(control)?.let { row -> powerCalculator(row.optJSONObject("calculator"))?.let { row.optText("path") to it } }
                }.toMap()
                ParameterRows(section.optText("title"), facts, note.joinToString(" "), calculators)
            }
        }
    }
}

@Composable
internal fun ParameterForm(
    page: String,
    modifier: Modifier = Modifier,
) {
    var rows by remember { mutableStateOf(emptyList<ParameterRows>()) }
    var loaded by remember { mutableStateOf(false) }
    var reloads by remember { mutableIntStateOf(0) }
    var calculating by remember { mutableStateOf<PowerCalculator?>(null) }
    var calibratingEscs by remember { mutableStateOf(false) }

    LaunchedEffect(page, reloads) {
        rows = withContext(Dispatchers.Default) { readPage(page) }
        loaded = true
    }

    if (!loaded) {
        Text("Reading parameters from the vehicle.", modifier.padding(16.dp))
        return
    }

    if (rows.isEmpty()) {
        Text("This vehicle exposes none of these parameters.", modifier.padding(16.dp))
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        rows.forEach { section ->
            item(key = "section:${section.title}") {
                SectionHeader(section.title)
                if (section.note.isNotBlank()) {
                    Text(
                        text = section.note,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 20.dp)
                            .padding(bottom = 12.dp),
                    )
                }
            }
            items(section.facts.size, key = { section.facts[it].path }) { index ->
                val fact = section.facts[index]
                if (fact.controlKind == DIALOG_CONTROL) {
                    OutlinedButton(enabled = fact.enabled, onClick = { calibratingEscs = true }, modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) { Text(fact.title) }
                } else if (fact.controlKind == LABEL_CONTROL) {
                    Text(
                        text = fact.title,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (fact.warning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp),
                    )
                } else {
                    FactRow(fact) { reloads++ }
                    section.calculators[fact.path]?.let { calculator ->
                        TextButton(onClick = { calculating = calculator }, modifier = Modifier.padding(horizontal = 12.dp)) { Text("Calculate") }
                    }
                }
            }
        }
        item(key = "refresh") {
            FootNote("Values refresh after each change.")
        }
    }

    if (calibratingEscs) EscCalibrationDialog { calibratingEscs = false }

    calculating?.let { calculator ->
        PowerCalcDialog(calculator) {
            calculating = null
            reloads++
        }
    }
}

internal fun bitmaskRaw(fact: Fact): Long =
    (fact.value as? Number)?.toLong() ?: fact.valueString.toDoubleOrNull()?.toLong() ?: 0L

internal fun bitmaskSummary(fact: Fact): String {
    val raw = bitmaskRaw(fact)
    val set = fact.bitmaskValues.indices.filter { raw and fact.bitmaskValues[it] != 0L }
    return when {
        set.isEmpty() -> "None"
        set.size == fact.bitmaskStrings.size -> "All"
        else -> set.joinToString(", ") { fact.bitmaskStrings[it] }
    }
}

internal const val LABEL_CONTROL = "label"

internal val KNOWN_CONTROL_KINDS = setOf("toggle", "choice", "bitmask", "text", "number", LABEL_CONTROL, DIALOG_CONTROL)

internal fun controlIsUnderstood(kind: String): Boolean =
    kind.isBlank() || kind in KNOWN_CONTROL_KINDS
