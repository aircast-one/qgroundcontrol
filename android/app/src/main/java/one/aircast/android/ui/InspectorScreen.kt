package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.TextButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import java.util.Locale
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText

private const val INSPECTOR_MESSAGES = "mavlinkInspector.activeSystem.messages"

internal fun inspectorSystemText(view: JSONObject?): String? {
    val system = view?.takeIf { it.optBoolean("available") }?.optInt("systemId", 0) ?: 0
    return if (system > 0) "System $system" else null
}
private const val INSPECTOR_VIEW = "view.inspector"

internal fun inspectorEmptyText(view: JSONObject?): String? =
    view?.optText("emptyText")?.ifBlank { null }

internal data class InspectorMessage(
    val index: Int,
    val id: Int,
    val name: String,
    val rateText: String,
    val count: Long,
    val path: String,
    val compId: Int,
    val title: String,
    val targetRateTitle: String = "",
)

internal data class InspectorRate(val rate: Int, val title: String)

internal fun inspectorRateChoices(view: JSONObject?): List<InspectorRate> {
    val items = view?.optJSONArray("rateChoices") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { InspectorRate(it.optInt("rate"), it.optText("title")) }
    }.filter { it.title.isNotBlank() }
}

internal data class InspectorField(
    val name: String,
    val type: String,
    val value: String,
)

internal fun inspectorMessages(view: JSONObject?): List<InspectorMessage> {
    val items = view?.optJSONArray("messages") ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { message ->
            InspectorMessage(
                index = message.optInt("index", index),
                id = message.optInt("id"),
                name = message.optText("name"),
                rateText = message.optText("rateText"),
                count = message.optLong("count"),
                path = message.optText("path"),
                compId = message.optInt("compId"),
                title = message.optText("title").ifBlank { message.optText("name") },
                targetRateTitle = message.optText("targetRateTitle"),
            )
        }
    }.sortedBy { it.name }
}

// view.inspector serves the SELECTED message's fields. Selecting is a write this screen has just
// made, so until the view names the message on screen as selected its fields describe the one before;
// those are refused rather than drawn under the wrong name.
internal fun parseInspectorFields(view: JSONObject?, messagePath: String): List<InspectorField> {
    val messages = view?.optJSONArray("messages") ?: return emptyList()
    val shown = (0 until messages.length()).any { index ->
        messages.optJSONObject(index)?.let { it.optBoolean("selected") && it.optText("path") == messagePath } == true
    }
    if (!shown) return emptyList()
    val fields = view.optJSONArray("fields") ?: return emptyList()
    return (0 until fields.length()).mapNotNull { index ->
        fields.optJSONObject(index)?.let { field ->
            InspectorField(
                name = field.optText("name"),
                type = field.optText("type"),
                value = field.optText("value"),
            )
        }
    }
}

private const val INSPECTOR_SELECTED = "mavlinkInspector.activeSystem.selected"

internal fun selectedPathFor(messagePath: String): String =
    messagePath.substringBefore(".messages.") + ".selected"

internal fun messageIndexIn(messagePath: String): Int =
    messagePath.substringAfterLast('.').toIntOrNull() ?: -1

private fun selectMessage(messagePath: String) {
    offMainDetached { Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath)) }
}

private fun requestRate(messagePath: String, rate: Int) {
    offMainDetached {
        Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath))
        Qgc.invoke("mavlinkInspector.setMessageInterval", rate)
    }
}

@Composable
private fun RatePicker(messagePath: String, current: String, choices: List<InspectorRate>) {
    var open by remember { mutableStateOf(false) }
    if (choices.isEmpty()) return
    Box {
        TextButton(onClick = { open = true }) {
            Text(if (current.isBlank()) "Set rate" else "Rate: $current")
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            choices.forEach { choice ->
                DropdownMenuItem(
                    text = { Text(choice.title) },
                    onClick = {
                        open = false
                        requestRate(messagePath, choice.rate)
                    },
                )
            }
        }
    }
}

internal fun openMessageIn(messages: List<InspectorMessage>, path: String?): InspectorMessage? =
    path?.let { wanted -> messages.firstOrNull { it.path == wanted } }

@Composable
private fun InspectorNotice(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

@Composable
private fun FieldList(messagePath: String, modifier: Modifier = Modifier) {
    LaunchedEffect(messagePath) { selectMessage(messagePath) }

    val inspectorJson by qgcPath(INSPECTOR_VIEW)
    val rows = parseInspectorFields(inspectorJson, messagePath)

    if (rows.isEmpty()) {
        InspectorNotice("Waiting for this message to arrive again.", modifier)
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        items(rows) { field ->
            ListItem(
                headlineContent = {
                    Text(field.name, fontFamily = FontFamily.Monospace)
                },
                supportingContent = { Text(field.type) },
                trailingContent = {
                    Text(field.value, fontFamily = FontFamily.Monospace)
                },
            )
            HorizontalDivider()
        }
    }
}

@Composable
fun InspectorScreen(modifier: Modifier = Modifier) {
    val inspectorJson by qgcPath(INSPECTOR_VIEW)
    var openPath by rememberSaveable { mutableStateOf<String?>(null) }
    var filter by rememberSaveable { mutableStateOf("") }
    val messages = inspectorMessages(inspectorJson)
    val open = openMessageIn(messages, openPath)

    BackHandler(enabled = open != null) { openPath = null }

    val emptyText = inspectorEmptyText(inspectorJson)
    if (emptyText != null) {
        InspectorNotice(emptyText, modifier)
        return
    }

    if (open != null) {
        Column(modifier.fillMaxSize()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { openPath = null }
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = open.name,
                    style = MaterialTheme.typography.titleSmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.weight(1f),
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                RatePicker(open.path, open.targetRateTitle, inspectorRateChoices(inspectorJson))
            }
            HorizontalDivider()
            FieldList(open.path, Modifier.weight(1f))
        }
        return
    }



    val shown = messages.filter { it.name.contains(filter, ignoreCase = true) }

    Column(modifier.fillMaxSize()) {
        inspectorSystemText(inspectorJson)?.let { line ->
            Text(
                line,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
            )
        }
        OutlinedTextField(
            value = filter,
            onValueChange = { filter = it },
            label = { Text("Filter messages") },
            singleLine = true,
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp, vertical = 8.dp),
        )

        if (shown.isEmpty()) {
            InspectorNotice("No message matches \"$filter\".")
            return@Column
        }

        LazyColumn(Modifier.fillMaxSize()) {
            items(shown, key = { it.path }) { message ->
                SetupRow(
                    title = message.title,
                    status = message.rateText,
                    onClick = { openPath = message.path },
                )
            }
        }
    }
}
