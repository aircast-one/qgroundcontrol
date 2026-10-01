package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.fromHtml
import androidx.compose.ui.graphics.Color
import kotlinx.coroutines.Dispatchers
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import kotlinx.coroutines.launch
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.mapspike.aircast

private const val PARAM_SCHEME = "param://"

internal fun paramLinkName(url: String): String? = url.takeIf { it.startsWith(PARAM_SCHEME) }?.removePrefix(PARAM_SCHEME)?.takeIf { it.isNotBlank() }

@Composable
internal fun LinkedText(html: String, style: TextStyle, color: Color, onParameter: (String) -> Unit, modifier: Modifier = Modifier) {
    val uris = LocalUriHandler.current
    val text = remember(html) {
        AnnotatedString.fromHtml(html) { link ->
            val url = (link as? LinkAnnotation.Url)?.url ?: return@fromHtml
            paramLinkName(url)?.let(onParameter) ?: runCatching { uris.openUri(url) }
        }
    }
    Text(text, style = style, color = color, modifier = modifier)
}

internal const val READ_ONLY_NOTE = "This parameter is read-only and cannot be modified."
internal const val FORCE_EDIT_NOTE = "Warning: This parameter is read-only. Force edit is enabled."

internal fun forceEditNote(readOnly: Boolean, forced: Boolean): String? = when {
    !readOnly -> null
    forced -> FORCE_EDIT_NOTE
    else -> READ_ONLY_NOTE
}

@Composable
private fun CheckRow(text: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, onCheckedChange = onChecked)
        Text(text, style = MaterialTheme.typography.bodySmall)
    }
}

internal const val IN_FLIGHT_WARNING = "Warning: Modifying values while vehicle is in flight can lead to vehicle instability and possible vehicle loss. Make sure you know what you are doing and double-check your values before Save!"

internal fun parameterDefault(json: JSONObject?): Any? =
    json?.takeIf { it.optBoolean("defaultValueAvailable") && it.has("defaultValue") && !it.isNull("defaultValue") }?.opt("defaultValue")

internal fun manualEntryFact(fact: Fact): Fact = fact.copy(enumStrings = emptyList(), enumValues = emptyList(), bitmaskStrings = emptyList(), bitmaskValues = emptyList())

@Composable
internal fun ParameterEditDialog(name: String, onDismiss: () -> Unit) {
    var revision by remember { mutableIntStateOf(0) }
    var advanced by remember { mutableStateOf(false) }
    var forced by remember { mutableStateOf(false) }
    var manual by remember { mutableStateOf(false) }
    var forceSave by remember { mutableStateOf(false) }
    var forcedText by remember { mutableStateOf("") }
    var forceRefusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val fact by produceState<Fact?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name) }
    }
    val default by produceState<Any?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterDefault(Qgc.get(parameterPath(name))) }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Edit Parameter") },
        text = {
            Column {
                fact?.let { loaded ->
                    val editable = !loaded.readOnly || forced
                    forceEditNote(loaded.readOnly, forced)?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = if (forced) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurface)
                    }
                    val shown = loaded.copy(readOnly = !editable).let { if (manual) manualEntryFact(it) else it }
                    FactRow(fact = shown, title = loaded.name, subtitle = parameterSubtitle(loaded.description, loaded.units), onWrite = { revision++ })
                    if (editable && default != null) {
                        TextButton(onClick = {
                            scope.launch {
                                withContext(Dispatchers.Default) { Qgc.writeRefusal(loaded.path, default) }
                                onDismiss()
                            }
                        }) { Text("Reset To Default") }
                    }
                    if (loaded.qgcRebootRequired) Text("Application restart required after change", style = MaterialTheme.typography.bodySmall)
                    if (editable) Text(IN_FLIGHT_WARNING, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning)
                    val hasChoices = loaded.isEnum || loaded.isBitmask
                    if (loaded.readOnly || (editable && hasChoices)) {
                        CheckRow("Advanced settings", advanced) { on ->
                            advanced = on
                            forced = forced && on
                            manual = manual && on
                        }
                        if (advanced && loaded.readOnly) CheckRow("Force edit read-only param", forced) { forced = it }
                        if (advanced && editable && hasChoices) CheckRow("Manual Entry", manual) { manual = it }
                    }
                    if (editable && !hasChoices && !loaded.isString && !loaded.isBool) {
                        CheckRow("Force save (dangerous!)", forceSave) { forceSave = it }
                        if (forceSave) {
                            OutlinedTextField(value = forcedText, onValueChange = { forcedText = it }, singleLine = true, label = { Text("Value") })
                            forceRefusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
                            TextButton(enabled = forcedText.isNotBlank(), onClick = {
                                scope.launch {
                                    val entered = forcedText.trim().toDoubleOrNull() ?: forcedText.trim()
                                    forceRefusal = withContext(Dispatchers.Default) { Qgc.writeForcedRefusal(loaded.path, entered) }
                                    if (forceRefusal == null) revision++
                                }
                            }) { Text("Save") }
                        }
                    }
                } ?: Text("$name is not a parameter on this vehicle.", style = MaterialTheme.typography.bodySmall)
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
