package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import org.json.JSONArray
import one.aircast.mapspike.optText

private const val PARAMETER_MANAGER = "vehicle.parameterManager"
private const val DEFAULT_COMPONENT = -1

internal fun parameterPath(name: String) = "$PARAMETER_MANAGER.getParameter($DEFAULT_COMPONENT,$name)"

@Composable
fun ParametersScreen(modifier: Modifier = Modifier) {
    val ready by qgcBool("$PARAMETER_MANAGER.parametersReady")
    var search by remember { mutableStateOf("") }
    var names by remember { mutableStateOf<List<String>>(emptyList()) }
    var descriptions by remember { mutableStateOf<Map<String, String>>(emptyMap()) }

    LaunchedEffect(ready) {
        names = if (!ready) emptyList() else withContext(Dispatchers.Default) { parameterNames() }
        descriptions = emptyMap()
    }

    LaunchedEffect(names) {
        if (names.isNotEmpty()) {
            descriptions = withContext(Dispatchers.Default) { parameterDescriptions(names) }
        }
    }

    val matches = remember(names, descriptions, search) {
        names.filter { parameterMatches(it, descriptions[it].orEmpty(), search) }
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

        Text(
            "${matches.size} parameter${if (matches.size == 1) "" else "s"}",
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )

        LazyColumn(Modifier.fillMaxSize()) {
            items(matches, key = { it }) { name ->
                ParameterRow(name)
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun ParameterRow(name: String) {
    var revision by remember { mutableStateOf(0) }
    val fact by produceState<Fact?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name) }
    }

    when (val loaded = fact) {
        null -> Text(
            text = name,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 20.dp),
        )
        else -> FactRow(
            fact = loaded,
            title = loaded.name,
            subtitle = parameterSubtitle(loaded.description, loaded.units),
            onWrite = { revision++ },
        )
    }
}

internal fun parameterMatches(name: String, description: String, search: String): Boolean =
    search.isBlank() ||
        name.contains(search, ignoreCase = true) ||
        description.contains(search, ignoreCase = true)

internal fun parameterSubtitle(description: String, units: String): String =
    listOf(description, units).filter { it.isNotBlank() }.joinToString(" · ")

private fun parameterNames(): List<String> {
    val result = Qgc.invokeResult("$PARAMETER_MANAGER.parameterNames", DEFAULT_COMPONENT) as? JSONArray
        ?: return emptyList()
    return (0 until result.length()).map { result.optText(it) }.sorted()
}

private fun parameterDescriptions(names: List<String>): Map<String, String> =
    names.mapNotNull { name ->
        parameterFact(name)?.description?.takeIf { it.isNotBlank() }?.let { name to it }
    }.toMap()

private fun parameterFact(name: String): Fact? =
    factFromParameter(name, Qgc.get(parameterPath(name)))
