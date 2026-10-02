package one.aircast.android.ui

import androidx.compose.material3.FilterChip
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
import androidx.compose.material3.MaterialTheme
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

private const val AUTOPILOT_COMPONENT = 1

internal fun parameterPath(key: String): String {
    val component = key.substringBefore(':', "").toIntOrNull()
    return "$PARAMETER_MANAGER.getParameter(${component ?: DEFAULT_COMPONENT},${if (component == null) key else key.substringAfter(':')})"
}

internal fun parameterKeys(namesByComponent: Map<Int, List<String>>): List<String> =
    namesByComponent.flatMap { (component, names) -> names.sorted().map { if (component == AUTOPILOT_COMPONENT || component == DEFAULT_COMPONENT) it else "$component:$it" } }

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
    var placement by remember { mutableStateOf<Map<String, Pair<String, String>>>(emptyMap()) }
    var chosenCategory by remember { mutableStateOf<String?>(null) }
    var chosenGroup by remember { mutableStateOf<String?>(null) }
    var reads by remember { mutableStateOf(0) }

    LaunchedEffect(ready, reads) {
        names = if (!ready) emptyList() else withContext(Dispatchers.Default) { parameterNames() }
    }

    LaunchedEffect(names, reads) {
        if (names.isNotEmpty()) {
            val summary = withContext(Dispatchers.Default) { parameterSummary(names) }
            descriptions = summary.descriptions
            modified = summary.modified
            placement = summary.placement
        }
    }

    val tree = remember(names, placement) { parameterTree(names, placement) }
    val browsing = search.isBlank() && !modifiedOnly
    val category = tree.firstOrNull { it.name == chosenCategory } ?: tree.firstOrNull()
    val group = category?.groups?.firstOrNull { it == chosenGroup } ?: category?.groups?.firstOrNull()
    val matches = remember(names, descriptions, modified, search, modifiedOnly, placement, category, group) {
        names.filter { parameterShown(it, descriptions[it].orEmpty(), search, modifiedOnly, modified) }
            .filter { !browsing || inGroup(it, placement, category?.name, group) }
    }
    Column(modifier.fillMaxSize()) {
        SearchPill(search, { search = it }, "Search parameters")

        if (!ready) {
            Text("Waiting for parameters from the vehicle.", Modifier.padding(16.dp))
            return@Column
        }

        Row(
            Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                parameterCountLine(matches.size, modified.size),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(selected = !modifiedOnly, onClick = { modifiedOnly = false }, label = { Text("All") })
                FilterChip(selected = modifiedOnly, onClick = { modifiedOnly = true }, label = { Text("Changed") })
            }
            ParameterToolsMenu(onRefreshed = { reads++ })
        }

        if (browsing && category != null) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                ChoiceButton(category.name, tree.map { it.name }) { chosenCategory = it; chosenGroup = null }
                group?.let { shown -> ChoiceButton(shown, category.groups) { chosenGroup = it } }
            }
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
    val live by qgcPath(parameterPath(name))
    val fact by produceState<Fact?>(null, name, revision, live) {
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
                style = MaterialTheme.typography.labelMedium,
                color = if (loaded.changedFromDefault) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.End,
                modifier = Modifier.widthIn(max = 160.dp),
            )
            if (offersRcToParam && !loaded.readOnly && ':' !in name) TextButton(onClick = { mapping = true }) { Text("RC") }
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
        (listOf(name.substringAfter(':')) + descriptions).any { text ->
            pattern?.containsMatchIn(text) ?: text.contains(word, ignoreCase = true)
        }
    }

internal fun parameterCountLine(shown: Int, changed: Int): String =
    listOfNotNull(
        "%,d parameter%s".format(java.util.Locale.US, shown, if (shown == 1) "" else "s"),
        "%,d changed".format(java.util.Locale.US, changed).takeIf { changed > 0 },
    ).joinToString(" \u00b7 ")

internal fun parameterValueText(fact: Fact): String = when {
    fact.isBitmask -> bitmaskSummary(fact)
    else -> listOf(enumLabel(fact), fact.units.takeIf { !fact.isEnum }.orEmpty()).filter { it.isNotBlank() }.joinToString(" ")
}

internal fun parameterSubtitle(description: String, units: String): String =
    listOf(description, units).filter { it.isNotBlank() }.joinToString(" · ")

internal fun parameterNames(): List<String> {
    fun names(component: Int): List<String> =
        (Qgc.invokeResult("$PARAMETER_MANAGER.parameterNames", component) as? JSONArray)?.let { list -> (0 until list.length()).map { list.optText(it) } }.orEmpty()
    val components = (Qgc.invokeResult("$PARAMETER_MANAGER.componentIds") as? JSONArray)?.let { list -> (0 until list.length()).map { list.optInt(it) } }.orEmpty()
    return parameterKeys(if (components.isEmpty()) mapOf(DEFAULT_COMPONENT to names(DEFAULT_COMPONENT)) else components.associateWith(::names))
}

internal data class ParameterSummary(
    val descriptions: Map<String, List<String>>,
    val modified: Set<String>,
    val placement: Map<String, Pair<String, String>> = emptyMap(),
)

internal data class ParameterCategory(val name: String, val groups: List<String>)

private const val STANDARD_CATEGORY = "Standard"
private const val DEFAULT_CATEGORY = "Other"
private const val DEFAULT_GROUP = "Misc"

internal fun parameterTree(names: List<String>, placement: Map<String, Pair<String, String>>): List<ParameterCategory> {
    val placed = names.mapNotNull { placement[it] }
    val categories = placed.map { it.first }.distinct()
    val ordered = categories.filter { it == STANDARD_CATEGORY } + categories.filter { it != STANDARD_CATEGORY && it != DEFAULT_CATEGORY } + categories.filter { it == DEFAULT_CATEGORY }
    return ordered.map { category ->
        val groups = placed.filter { it.first == category }.map { it.second }.distinct()
        ParameterCategory(category, groups.filter { it != DEFAULT_GROUP } + groups.filter { it == DEFAULT_GROUP })
    }
}

internal fun inGroup(name: String, placement: Map<String, Pair<String, String>>, category: String?, group: String?): Boolean =
    category == null || group == null || placement[name] == (category to group)

private fun parameterSummary(names: List<String>): ParameterSummary {
    val facts = names.mapNotNull { name -> parameterFact(name)?.let { name to it } }
    return ParameterSummary(
        descriptions = facts.associate { (name, fact) -> name to listOf(fact.description, fact.longDescription).filter { it.isNotBlank() } },
        modified = facts.filter { it.second.changedFromDefault }.map { it.first }.toSet(),
        placement = facts.associate { (name, fact) -> name to (fact.category to fact.group) },
    )
}

internal fun parameterFact(name: String): Fact? =
    factFromParameter(name, Qgc.get(parameterPath(name)))

@Composable
private fun ChoiceButton(shown: String, options: List<String>, onPick: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    androidx.compose.foundation.layout.Box {
        androidx.compose.material3.OutlinedButton(onClick = { open = true }) { Text(shown, maxLines = 1, overflow = TextOverflow.Ellipsis) }
        androidx.compose.material3.DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEach { option ->
                androidx.compose.material3.DropdownMenuItem(text = { Text(option) }, onClick = { open = false; onPick(option) })
            }
        }
    }
}
