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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import java.util.Locale
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath

private const val INSPECTOR_MESSAGES = "mavlinkInspector.activeSystem.messages"

internal data class InspectorMessage(
    val index: Int,
    val id: Int,
    val name: String,
    val rateHz: Double,
    val count: Long,
)

internal data class InspectorField(
    val name: String,
    val type: String,
    val value: String,
)

internal fun parseInspectorMessages(model: JSONObject?): List<InspectorMessage> {
    val elements = model?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        elements.optJSONObject(index)?.let { message ->
            InspectorMessage(
                index = index,
                id = message.optInt("id"),
                name = message.optString("name"),
                rateHz = message.optDouble("actualRateHz", 0.0),
                count = message.optLong("count"),
            )
        }
    }.sortedBy { it.name }
}

internal fun parseInspectorFields(model: JSONObject?): List<InspectorField> {
    val elements = model?.optJSONArray("elements") ?: return emptyList()
    return (0 until elements.length()).mapNotNull { index ->
        elements.optJSONObject(index)?.let { field ->
            InspectorField(
                name = field.optString("name"),
                type = field.optString("type"),
                value = field.optString("value"),
            )
        }
    }
}

private fun setMessageSelected(index: Int, selected: Boolean) {
    Thread { Qgc.set("$INSPECTOR_MESSAGES.$index.selected", selected) }.start()
}

internal fun formatRate(rateHz: Double): String =
    if (rateHz.isNaN()) "--" else String.format(Locale.US, "%.1f Hz", rateHz)

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
private fun FieldList(messageIndex: Int, modifier: Modifier = Modifier) {
    DisposableEffect(messageIndex) {
        setMessageSelected(messageIndex, true)
        onDispose { setMessageSelected(messageIndex, false) }
    }

    val fields by qgcPath("$INSPECTOR_MESSAGES.$messageIndex.fields")
    val rows = parseInspectorFields(fields)

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
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val messagesModel by qgcPath(INSPECTOR_MESSAGES)
    var openMessage by remember { mutableStateOf<InspectorMessage?>(null) }
    val messages = parseInspectorMessages(messagesModel)

    BackHandler(enabled = openMessage != null) { openMessage = null }

    if (!hasVehicle) {
        InspectorNotice("Connect a vehicle to inspect its MAVLink traffic.", modifier)
        return
    }

    val open = openMessage
    if (open != null) {
        Column(modifier.fillMaxSize()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { openMessage = null }
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = open.name,
                    style = MaterialTheme.typography.titleSmall,
                    fontFamily = FontFamily.Monospace,
                )
            }
            HorizontalDivider()
            FieldList(open.index, Modifier.weight(1f))
        }
        return
    }

    if (messages.isEmpty()) {
        InspectorNotice("No MAVLink messages seen yet.", modifier)
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        items(messages, key = { it.index }) { message ->
            ListItem(
                headlineContent = {
                    Text(message.name, fontFamily = FontFamily.Monospace)
                },
                supportingContent = { Text("id ${message.id} · ${message.count} received") },
                trailingContent = { Text(formatRate(message.rateHz)) },
                modifier = Modifier.clickable { openMessage = message },
            )
            HorizontalDivider()
        }
    }
}
