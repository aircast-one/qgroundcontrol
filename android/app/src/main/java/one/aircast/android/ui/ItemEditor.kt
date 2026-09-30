package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONArray
import org.json.JSONObject

internal fun itemFactsPath(index: Int): String = "view.itemFacts($index)"

internal fun itemCommandPath(index: Int): String = "plan.missionController.visualItems.$index.command"

internal data class CommandChoice(val id: Int, val name: String, val description: String)

internal fun commandChoices(result: Any?): List<CommandChoice> {
    val listed = result as? JSONArray ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let {
            CommandChoice(it.optInt("command"), it.optText("friendlyName"), it.optText("description"))
        }
    }
}

internal fun categoryNames(result: Any?): List<String> {
    val listed = result as? JSONArray ?: return emptyList()
    return (0 until listed.length()).map { listed.optText(it) }
}

internal fun itemFields(view: JSONObject?): List<one.aircast.android.bridge.Fact> {
    val listed = view?.optJSONArray("fields") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it)?.let(::factFromControl) }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ItemEditor(index: Int, onDismiss: () -> Unit) {
    var revision by remember(index) { mutableIntStateOf(0) }
    var view by remember(index) { mutableStateOf<JSONObject?>(null) }
    var choosing by remember(index) { mutableStateOf(false) }
    var refusal by remember(index) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(index, revision) {
        view = withContext(Dispatchers.Default) { Qgc.get(itemFactsPath(index)) }
    }

    val fields = remember(view) { itemFields(view) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Item ${index}", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                if (view?.optBoolean("simple") == true) {
                    TextButton(onClick = { choosing = true }) { Text("Change command") }
                }
            }
            refusal?.let {
                Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 20.dp))
            }
            LazyColumn(Modifier.heightIn(max = 480.dp)) {
                items(fields, key = { it.path }) { fact ->
                    FactRow(fact) { revision++ }
                }
            }
        }
    }

    if (choosing) {
        CommandPicker(
            onDismiss = { choosing = false },
            onChosen = { command ->
                choosing = false
                scope.launch {
                    refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(itemCommandPath(index), command) }
                    revision++
                }
            },
        )
    }
}

@Composable
private fun CommandPicker(onDismiss: () -> Unit, onChosen: (Int) -> Unit) {
    var categories by remember { mutableStateOf<List<String>>(emptyList()) }
    var category by remember { mutableStateOf<String?>(null) }
    var commands by remember { mutableStateOf<List<CommandChoice>>(emptyList()) }

    LaunchedEffect(Unit) {
        categories = withContext(Dispatchers.Default) { categoryNames(Qgc.invokeResult("missionCommandTree.categoriesForVehicle")) }
        category = categories.firstOrNull()
    }
    LaunchedEffect(category) {
        val chosen = category ?: return@LaunchedEffect
        commands = withContext(Dispatchers.Default) {
            commandChoices(Qgc.invokeResult("missionCommandTree.getCommandsForCategory", null, chosen, true))
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select Mission Command") },
        text = {
            Column {
                Row(Modifier.horizontalScroll(rememberScrollState())) {
                    categories.forEach { name ->
                        FilterChip(
                            selected = name == category,
                            onClick = { category = name },
                            label = { Text(name) },
                            modifier = Modifier.padding(end = 4.dp),
                        )
                    }
                }
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    items(commands, key = { it.id }) { command ->
                        ListItem(
                            headlineContent = { Text(command.name) },
                            supportingContent = { Text(command.description, style = MaterialTheme.typography.bodySmall) },
                            modifier = Modifier.clickable { onChosen(command.id) },
                        )
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
