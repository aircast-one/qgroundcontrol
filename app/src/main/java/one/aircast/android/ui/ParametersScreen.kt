package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import org.json.JSONArray

private const val PARAMETER_MANAGER = "vehicle.parameterManager"
private const val DEFAULT_COMPONENT = -1
private const val MAX_ROWS = 60

private fun parameterPath(name: String) = "$PARAMETER_MANAGER.getParameter($DEFAULT_COMPONENT,$name)"

@Composable
fun ParametersScreen(modifier: Modifier = Modifier) {
    val ready by qgcBool("$PARAMETER_MANAGER.parametersReady")
    var search by remember { mutableStateOf("") }
    var names by remember { mutableStateOf<List<String>>(emptyList()) }
    var facts by remember { mutableStateOf<List<Fact>>(emptyList()) }
    var revision by remember { mutableStateOf(0) }

    LaunchedEffect(ready) {
        names = if (!ready) emptyList() else withContext(Dispatchers.Default) { parameterNames() }
    }

    val matches = remember(names, search) {
        names.filter { search.isBlank() || it.contains(search, ignoreCase = true) }
    }
    val visible = remember(matches) { matches.take(MAX_ROWS) }

    LaunchedEffect(visible, revision) {
        facts = withContext(Dispatchers.Default) { visible.mapNotNull { parameterFact(it) } }
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
            "${matches.size} parameters" + if (matches.size > visible.size) ", showing first ${visible.size}" else "",
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )

        LazyColumn(Modifier.fillMaxSize()) {
            items(facts, key = { it.name }) { fact ->
                ParameterRow(fact) { revision++ }
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun ParameterRow(fact: Fact, onWritten: () -> Unit) {
    var editing by remember(fact.name, fact.valueString) { mutableStateOf<String?>(null) }
    val pending = editing

    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(Modifier.weight(1f).padding(end = 8.dp)) {
            Text(fact.name, style = MaterialTheme.typography.bodyLarge)
            Text(
                fact.description.ifBlank { fact.units },
                style = MaterialTheme.typography.bodySmall,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }

        OutlinedTextField(
            value = pending ?: fact.valueString,
            onValueChange = { editing = it },
            singleLine = true,
            readOnly = fact.readOnly,
            modifier = Modifier.width(150.dp),
            trailingIcon = {
                if (pending != null && pending != fact.valueString) {
                    TextButton(onClick = {
                        Qgc.set(parameterPath(fact.name), pending)
                        editing = null
                        onWritten()
                    }) { Text("Set") }
                }
            },
        )
    }
}

private fun parameterNames(): List<String> {
    val result = Qgc.invokeResult("$PARAMETER_MANAGER.parameterNames", DEFAULT_COMPONENT) as? JSONArray
        ?: return emptyList()
    return (0 until result.length()).map { result.optString(it) }.sorted()
}

private fun parameterFact(name: String): Fact? {
    val json = Qgc.get(parameterPath(name))
    if (json.optString("name").isBlank()) return null
    return Qgc.fact("", json)
}
