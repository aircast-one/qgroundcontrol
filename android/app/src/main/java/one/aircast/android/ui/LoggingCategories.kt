package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
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
import org.json.JSONObject

internal const val LOG_CATEGORIES_VIEW = "view.logCategories"
internal const val LOG_SET_CATEGORY = "appLog.setCategory"
internal const val LOG_RESET_CATEGORIES = "appLog.resetCategories"
private const val INDENT_PER_DEPTH = 12

internal data class LogCategory(val name: String, val shortName: String, val depth: Int, val enabled: Boolean)

internal data class LogCategories(val active: List<String>, val categories: List<LogCategory>)

internal fun logCategories(view: JSONObject?): LogCategories? = view?.let {
    val active = it.optJSONArray("active")
    val listed = it.optJSONArray("categories")
    LogCategories(
        active = active?.let { list -> (0 until list.length()).map { index -> list.optString(index) } }.orEmpty(),
        categories = listed?.let { list ->
            (0 until list.length()).mapNotNull { index -> list.optJSONObject(index) }.map { category ->
                LogCategory(category.optText("name"), category.optText("shortName"), category.optInt("depth"), category.optBoolean("enabled"))
            }
        }.orEmpty(),
    )
}

internal fun filteredCategories(categories: List<LogCategory>, search: String): List<LogCategory> =
    if (search.isBlank()) categories else categories.filter { it.name.contains(search.trim(), ignoreCase = true) }

@Composable
internal fun LoggingCategoriesDialog(onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    var read by remember { mutableStateOf<LogCategories?>(null) }
    var revision by remember { mutableIntStateOf(0) }
    var search by remember { mutableStateOf("") }

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { logCategories(Qgc.get(LOG_CATEGORIES_VIEW)) }
    }

    fun act(path: String, vararg args: Any) {
        scope.launch {
            withContext(Dispatchers.Default) { Qgc.invoke(path, *args) }
            revision++
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Logging categories") },
        text = {
            Column(Modifier.heightIn(max = 520.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = search,
                        onValueChange = { search = it },
                        placeholder = { Text("Filter categories…") },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = { search = "" }) { Text("Clear") }
                }
                Text("Active categories", style = MaterialTheme.typography.titleSmall)
                read?.active?.forEach { name ->
                    CategorySwitch(name, checked = true, depth = 0) { act(LOG_SET_CATEGORY, name, false) }
                }
                OutlinedButton(onClick = { act(LOG_RESET_CATEGORIES) }) { Text("Reset all") }
                Text("Categories", style = MaterialTheme.typography.titleSmall)
                filteredCategories(read?.categories.orEmpty(), search).forEach { category ->
                    val label = if (search.isBlank()) category.shortName else category.name
                    CategorySwitch(label, category.enabled, if (search.isBlank()) category.depth else 0) { act(LOG_SET_CATEGORY, category.name, it) }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}

@Composable
private fun CategorySwitch(label: String, checked: Boolean, depth: Int, onChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(start = (depth * INDENT_PER_DEPTH).dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Switch(checked = checked, onCheckedChange = onChange)
    }
}
