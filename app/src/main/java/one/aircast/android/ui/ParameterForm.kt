package one.aircast.android.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
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

internal data class ParameterSection(
    val title: String,
    val names: List<String>,
    val note: String = "",
)

internal data class ParameterRows(
    val title: String,
    val facts: List<Fact>,
    val note: String,
)

internal fun factFromParameter(name: String, json: JSONObject): Fact? =
    if (json.optString("kind") == "fact") {
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
    val label = control.optString("label")
    val name = control.optString("name")
    if (label.isBlank() && name.isBlank()) return null
    val options = control.optJSONArray("options")
    val labels = (0 until (options?.length() ?: 0)).map { options!!.optJSONObject(it).optString("label") }
    return Fact(
        path = control.optString("path"),
        name = name,
        description = label,
        units = control.optString("units"),
        valueString = control.optString("valueString"),
        value = control.opt("value"),
        enumStrings = labels,
        enumIndex = labels.indexOf(control.optString("display")),
        isBool = control.optString("control") == "toggle",
        isString = control.optString("control") == "text",
        readOnly = control.optBoolean("readOnly"),
        vehicleRebootRequired = control.optBoolean("rebootRequired"),
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
                    section.optString("note").ifBlank { null },
                    readOnlyNote(facts),
                )
                ParameterRows(section.optString("title"), facts, note.joinToString(" "))
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
                FactRow(section.facts[index]) { reloads++ }
            }
        }
        item(key = "refresh") {
            FootNote("Values refresh after each change.")
        }
    }
}
