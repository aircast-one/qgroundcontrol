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

private fun readSections(sections: List<ParameterSection>): List<ParameterRows> =
    sections.mapNotNull { section ->
        val facts = section.names.mapNotNull { name ->
            factFromParameter(name, Qgc.get(parameterPath(name)))
        }
        if (facts.isEmpty()) {
            null
        } else {
            ParameterRows(section.title, facts, section.note)
        }
    }

@Composable
internal fun ParameterForm(
    sections: List<ParameterSection>,
    modifier: Modifier = Modifier,
) {
    var rows by remember { mutableStateOf(emptyList<ParameterRows>()) }
    var loaded by remember { mutableStateOf(false) }
    var reloads by remember { mutableIntStateOf(0) }

    LaunchedEffect(sections, reloads) {
        rows = withContext(Dispatchers.Default) { readSections(sections) }
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
