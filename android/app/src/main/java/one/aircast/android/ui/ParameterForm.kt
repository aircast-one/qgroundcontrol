package one.aircast.android.ui

import one.aircast.android.R
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Surface
import one.aircast.mapspike.aircast
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.layout.fillMaxSize
import one.aircast.android.bridge.FactSlider
import kotlinx.coroutines.launch
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.material3.Slider
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
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
    val image: String = "",
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
        enumValues = (0 until (options?.length() ?: 0)).map { options!!.optJSONObject(it).optText("raw") },
        enumIndex = labels.indexOf(control.optText("display")),
        bitmaskStrings = bitEntries.map { it.first },
        bitmaskValues = bitEntries.map { it.second },
        controlKind = control.optText("control"),
        isBool = control.optText("control") == "toggle",
        isString = control.optText("control") == "text",
        readOnly = control.optBoolean("readOnly"),
        enabled = control.optBoolean("enabled", true),
        disabledReason = control.optText("disabledReason"),
        vehicleRebootRequired = control.optBoolean("vehicleRebootRequired"),
        minString = control.optText("minimumText"),
        maxString = control.optText("maximumText"),
        minIsDefaultForType = control.isNull("minimumText"),
        maxIsDefaultForType = control.isNull("maximumText"),
        defaultValueString = control.optText("defaultText"),
        qgcRebootRequired = control.optBoolean("applicationRestartRequired"),
        slider = control.optJSONObject("slider")?.let { factSlider(it, control.optText("description")) },
        warning = control.optBoolean("warning"),
        optional = control.optBoolean("optional"),
        firstEntryIsAll = control.optBoolean("firstEntryIsAll"),
        indent = control.optBoolean("indent"),
        smallFont = control.optBoolean("smallFont"),
        shortLabel = control.optText("shortLabel"),
    )
}

internal fun readPage(page: String): List<ParameterRows> {
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
                ParameterRows(section.optText("title"), facts, note.joinToString(" "), calculators, section.optText("image"))
            }
        }
    }
}

internal val EMPTY_PAGE_TEXTS = mapOf("Gimbal" to "Gimbal settings are not available for this firmware version.")

@Composable
internal fun ParameterForm(
    page: String,
    modifier: Modifier = Modifier,
    highlighted: Set<String> = emptySet(),
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
        EmptyState(R.drawable.ic_tune, "Nothing to set here", EMPTY_PAGE_TEXTS[page] ?: "This vehicle exposes none of these parameters.", modifier)
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        rows.forEach { section ->
            item(key = "section:${section.title}") {
                SectionHeader(sentenceCase(section.title), section.image)
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
            val runs = fieldRuns(section.facts) { fact ->
                fact.slider == null && fact.controlKind != DIALOG_CONTROL && fact.controlKind != LABEL_CONTROL &&
                    !fact.indent && fact.path !in section.calculators && fact.name !in highlighted
            }
            items(runs.size, key = { runs[it].first().path }) { index ->
                val run = runs[index]
                if (run.size > 1) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        run.forEach { fact ->
                            FactRow(fact, fieldModifier = Modifier.weight(1f).padding(vertical = 8.dp)) { reloads++ }
                        }
                    }
                    return@items
                }
                val fact = run.first()
                Column(Modifier.padding(start = if (fact.indent) INDENT else 0.dp)) {
                    if (fact.controlKind == DIALOG_CONTROL) {
                        OutlinedButton(enabled = fact.enabled, onClick = { calibratingEscs = true }, modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) { Text(fact.title) }
                    } else if (fact.controlKind == LABEL_CONTROL) {
                        Text(
                            text = fact.title,
                            style = if (fact.smallFont) MaterialTheme.typography.bodySmall else MaterialTheme.typography.bodyMedium,
                            color = if (fact.warning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp),
                        )
                    } else if (fact.slider != null) {
                        FactSliderRow(fact, fact.slider) { reloads++ }
                    } else {
                        FactRow(fact, titleColor = if (fact.name in highlighted) MaterialTheme.aircast.warning else Color.Unspecified) { reloads++ }
                        section.calculators[fact.path]?.let { calculator ->
                            Surface(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.medium) {
                                Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                    Column(Modifier.weight(1f)) {
                                        Text(sentenceCase(calculator.title), style = MaterialTheme.typography.bodyLarge)
                                        Text("Measure the ${calculator.measure} with a meter", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    }
                                    FilledTonalButton(onClick = { calculating = calculator }) { Text("Calibrate") }
                                }
                            }
                        }
                    }
                }
            }
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

private val INDENT = 16.dp

internal val KNOWN_CONTROL_KINDS = setOf("toggle", "choice", "bitmask", "text", "number", LABEL_CONTROL, DIALOG_CONTROL)

internal fun controlIsUnderstood(kind: String): Boolean =
    kind.isBlank() || kind in KNOWN_CONTROL_KINDS

internal fun factSlider(json: JSONObject, hint: String): FactSlider? {
    val from = json.optDouble("from", Double.NaN).takeIf { !it.isNaN() } ?: return null
    val to = json.optDouble("to", Double.NaN).takeIf { !it.isNaN() && it > from } ?: return null
    return FactSlider(from.toFloat(), to.toFloat(), json.optInt("decimals", 2), hint)
}

@Composable
private fun FactSliderRow(fact: Fact, slider: FactSlider, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    val held = (fact.value as? Number)?.toFloat() ?: fact.valueString.toFloatOrNull() ?: slider.from
    var shown by remember(fact.path, held) { mutableStateOf(held.coerceIn(slider.from, slider.to)) }
    FactRow(fact.copy(slider = null), onWrite = onWrite)
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {
        if (slider.hint.isNotBlank()) Text(slider.hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Slider(
            value = shown,
            onValueChange = { shown = it },
            valueRange = slider.from..slider.to,
            enabled = fact.enabled && !fact.readOnly,
            onValueChangeFinished = {
                scope.launch {
                    withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, shown.toDouble()) }
                    onWrite()
                }
            },
        )
    }
}
