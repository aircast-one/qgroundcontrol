package one.aircast.android.ui

import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.TextButton
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import org.json.JSONArray
import one.aircast.mapspike.optText

private const val PARAMETER_MANAGER = "vehicle.parameterManager"
private const val DEFAULT_COMPONENT = -1

internal fun parameterPath(name: String) = "$PARAMETER_MANAGER.getParameter($DEFAULT_COMPONENT,$name)"

@Composable
fun ParametersScreen(modifier: Modifier = Modifier, initialSearch: String = "") {
    val setupJson by qgcPath(SETUP)
    val ready = remember(setupJson) { parametersReady(setupJson) }
    val px4 = remember(setupJson) { isPx4(setupReadiness(setupJson)) }
    var search by remember { mutableStateOf(initialSearch) }
    var names by remember { mutableStateOf<List<String>>(emptyList()) }
    var descriptions by remember { mutableStateOf<Map<String, List<String>>>(emptyMap()) }
    var modified by remember { mutableStateOf<Set<String>>(emptySet()) }
    var modifiedOnly by remember { mutableStateOf(false) }
    var reads by remember { mutableStateOf(0) }

    LaunchedEffect(ready, reads) {
        names = if (!ready) emptyList() else withContext(Dispatchers.Default) { parameterNames() }
        descriptions = emptyMap()
        modified = emptySet()
    }

    LaunchedEffect(names) {
        if (names.isNotEmpty()) {
            val summary = withContext(Dispatchers.Default) { parameterSummary(names) }
            descriptions = summary.descriptions
            modified = summary.modified
        }
    }

    val matches = remember(names, descriptions, modified, search, modifiedOnly) {
        names.filter { parameterShown(it, descriptions[it].orEmpty(), search, modifiedOnly, modified) }
    }
    Column(modifier.fillMaxSize()) {
        OutlinedTextField(
            value = search,
            onValueChange = { search = it },
            label = { Text("Search parameters") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        )

        if (!ready) {
            Text("Waiting for parameters from the vehicle.", Modifier.padding(16.dp))
            return@Column
        }

        Row(
            Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "${matches.size} parameter${if (matches.size == 1) "" else "s"}",
                style = MaterialTheme.typography.labelMedium,
                modifier = Modifier.weight(1f),
            )
            Checkbox(checked = modifiedOnly, onCheckedChange = { modifiedOnly = it })
            Text("Modified", style = MaterialTheme.typography.labelLarge)
            ParameterToolsMenu(onRefreshed = { reads++ })
        }

        LazyColumn(Modifier.fillMaxSize()) {
            items(matches, key = { it }) { name ->
                ParameterRow(name, px4)
            }
        }
    }
}

@Composable
private fun ParameterRow(name: String, offersRcToParam: Boolean) {
    var revision by remember { mutableStateOf(0) }
    var mapping by remember { mutableStateOf(false) }
    var forcing by remember { mutableStateOf(false) }
    val fact by produceState<Fact?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name) }
    }

    when (val loaded = fact) {
        null -> Text(
            text = name,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 20.dp),
        )
        else -> Row(
            Modifier
                .fillMaxWidth()
                .clickable { forcing = true }
                .heightIn(min = 72.dp)
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Text(loaded.name, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (loaded.description.isNotBlank()) {
                    Text(loaded.description, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
            Text(
                text = parameterValueText(loaded),
                style = MaterialTheme.typography.labelLarge,
                color = if (loaded.changedFromDefault) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.End,
                modifier = Modifier.widthIn(max = 160.dp),
            )
            if (offersRcToParam && !loaded.readOnly) TextButton(onClick = { mapping = true }) { Text("RC") }
            if (forcing) ParameterEditDialog(name) {
                forcing = false
                revision++
            }
            if (mapping) RcToParamDialog(loaded) { mapping = false }
        }
    }
}

internal fun parameterShown(name: String, descriptions: List<String>, search: String, modifiedOnly: Boolean, modified: Set<String>): Boolean =
    parameterMatches(name, descriptions, search) && (!modifiedOnly || name in modified)

internal fun parameterMatches(name: String, descriptions: List<String>, search: String): Boolean =
    search.split(' ').filter { it.isNotEmpty() }.all { word ->
        val pattern = runCatching { Regex(word, RegexOption.IGNORE_CASE) }.getOrNull()
        (listOf(name) + descriptions).any { text ->
            pattern?.containsMatchIn(text) ?: text.contains(word, ignoreCase = true)
        }
    }

internal fun parameterValueText(fact: Fact): String = when {
    fact.isBitmask -> bitmaskSummary(fact)
    else -> listOf(enumLabel(fact), fact.units.takeIf { !fact.isEnum }.orEmpty()).filter { it.isNotBlank() }.joinToString(" ")
}

internal fun parameterSubtitle(description: String, units: String): String =
    listOf(description, units).filter { it.isNotBlank() }.joinToString(" · ")

private fun parameterNames(): List<String> {
    val result = Qgc.invokeResult("$PARAMETER_MANAGER.parameterNames", DEFAULT_COMPONENT) as? JSONArray
        ?: return emptyList()
    return (0 until result.length()).map { result.optText(it) }.sorted()
}

internal data class ParameterSummary(val descriptions: Map<String, List<String>>, val modified: Set<String>)

private fun parameterSummary(names: List<String>): ParameterSummary {
    val facts = names.mapNotNull { name -> parameterFact(name)?.let { name to it } }
    return ParameterSummary(
        descriptions = facts.associate { (name, fact) -> name to listOf(fact.description, fact.longDescription).filter { it.isNotBlank() } },
        modified = facts.filter { it.second.changedFromDefault }.map { it.first }.toSet(),
    )
}

internal fun parameterFact(name: String): Fact? =
    factFromParameter(name, Qgc.get(parameterPath(name)))
