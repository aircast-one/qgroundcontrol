package one.aircast.android.ui

import androidx.compose.ui.unit.dp
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import one.aircast.map.AircastSheet
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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
import androidx.compose.runtime.rememberUpdatedState
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
import one.aircast.map.aircast

private const val PARAM_SCHEME = "param://"

internal fun paramLinkName(url: String): String? = url.takeIf { it.startsWith(PARAM_SCHEME) }?.removePrefix(PARAM_SCHEME)?.takeIf { it.isNotBlank() }

internal const val SETUP_DIALOG_OPENED = "setup.dialogOpened"
internal const val SENSOR_SETTINGS_DIALOG = "sensorSettings"
private const val PARAMETER_EDITOR_DIALOG = "parameterEditor"

internal fun parameterLinkOpens(name: String): Boolean =
    Qgc.call(SETUP_DIALOG_OPENED, PARAMETER_EDITOR_DIALOG, name)?.optBoolean("exists", true) ?: true

@Composable
internal fun LinkedText(html: String, style: TextStyle, color: Color, onParameter: (String) -> Unit, modifier: Modifier = Modifier) {
    val uris = LocalUriHandler.current
    val scope = rememberCoroutineScope()
    val opener by rememberUpdatedState(onParameter)
    val text = remember(html) {
        AnnotatedString.fromHtml(html) { link ->
            val url = (link as? LinkAnnotation.Url)?.url ?: return@fromHtml
            paramLinkName(url)?.let { name ->
                scope.launch { if (withContext(Dispatchers.Default) { parameterLinkOpens(name) }) opener(name) }
            } ?: runCatching { uris.openUri(url) }
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

internal fun parameterRangeLine(fact: Fact): String {
    val units = fact.units.takeIf { it.isNotBlank() && !fact.isEnum }?.let { " $it" }.orEmpty()
    val min = fact.minString.takeIf { !fact.minIsDefaultForType && it.isNotBlank() }
    val max = fact.maxString.takeIf { !fact.maxIsDefaultForType && it.isNotBlank() }
    val bounds = when {
        min != null && max != null -> "Range $min\u2013$max$units"
        min != null -> "Min $min$units"
        max != null -> "Max $max$units"
        else -> null
    }
    return listOfNotNull(
        bounds,
        "default ${fact.defaultValueString}$units".takeIf { fact.defaultValueString.isNotBlank() },
    ).joinToString(" \u00b7 ")
}

internal fun parameterRebootNotes(fact: Fact): List<String> = listOfNotNull(
    "Vehicle reboot required after change".takeIf { fact.vehicleRebootRequired },
    "Application restart required after change".takeIf { fact.qgcRebootRequired },
)

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
internal fun ParameterEditDialog(name: String, title: String = name, onDismiss: () -> Unit) {
    var revision by remember { mutableIntStateOf(0) }
    var advanced by remember { mutableStateOf(false) }
    var forced by remember { mutableStateOf(false) }
    var manual by remember { mutableStateOf(false) }
    var forceSave by remember { mutableStateOf(false) }
    var rejected by remember { mutableStateOf(false) }
    var forcedText by remember { mutableStateOf("") }
    var forceRefusal by remember { mutableStateOf<String?>(null) }
    val forceAllowed = advancedUiShown()
    val scope = rememberCoroutineScope()
    val fact by produceState<Fact?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name) }
    }
    val default by produceState<Any?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterDefault(Qgc.get(parameterPath(name))) }
    }
    AircastSheet(onDismissRequest = onDismiss) {
            Column(
                Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(title, style = MaterialTheme.typography.titleLarge)
                fact?.let { loaded ->
                    loaded.longDescription.ifBlank { loaded.description }.takeIf { it.isNotBlank() }?.let {
                        Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    val editable = !loaded.readOnly || forced
                    forceEditNote(loaded.readOnly, forced)?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = if (forced) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurface)
                    }
                    val shown = loaded.let { if (manual) manualEntryFact(it) else it }
                    androidx.compose.runtime.CompositionLocalProvider(LocalBlockRebootNote provides factRebootNote(loaded)) {
                        FactRow(fact = shown, title = loaded.heading.ifBlank { "Value" }, subtitle = "", fieldModifier = Modifier.fillMaxWidth().padding(vertical = 8.dp), onWrite = { revision++ }, onRejected = { rejected = true })
                    }
                    parameterRangeLine(loaded).takeIf { it.isNotBlank() }?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    parameterRebootNotes(loaded).forEach { Text(it, style = MaterialTheme.typography.bodySmall) }
                    if (editable) Text(IN_FLIGHT_WARNING, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning)
                    val hasChoices = loaded.isEnum || loaded.isBitmask
                    if ((loaded.readOnly && forceAllowed) || (editable && hasChoices)) {
                        CheckRow("Advanced settings", advanced) { on ->
                            advanced = on
                            forced = forced && on
                            manual = manual && on
                        }
                        if (advanced && loaded.readOnly && forceAllowed) CheckRow("Force edit read-only param", forced) { forced = it }
                        if (advanced && editable && hasChoices) CheckRow("Manual entry", manual) { manual = it }
                    }
                    if (forced || (editable && (manual || !hasChoices) && !loaded.isString && !loaded.isBool)) {
                        if (!forced && rejected && forceAllowed) CheckRow("Force save (dangerous!)", forceSave) { forceSave = it }
                        if (forceSave || forced) {
                            OutlinedTextField(value = forcedText, onValueChange = { forcedText = it }, singleLine = true, label = { Text("Value") })
                            forceRefusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
                            TextButton(enabled = forcedText.isNotBlank(), onClick = {
                                scope.launch {
                                    val entered = if (loaded.isString) forcedText else forcedText.trim().toDoubleOrNull() ?: forcedText.trim()
                                    forceRefusal = withContext(Dispatchers.Default) { Qgc.writeForcedRefusal(loaded.path, entered) }
                                    if (forceRefusal == null) revision++
                                }
                            }) { Text("Save") }
                        }
                    }
                } ?: Text("$name is not a parameter on this vehicle.", style = MaterialTheme.typography.bodySmall)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End)) {
                    fact?.takeIf { (!it.readOnly || forced) && default != null }?.let { loaded ->
                        TextButton(onClick = {
                            scope.launch {
                                forceRefusal = withContext(Dispatchers.Default) { if (forced) Qgc.writeForcedRefusal(loaded.path, default) else Qgc.writeRefusal(loaded.path, default) }
                                if (forceRefusal == null) onDismiss()
                            }
                        }) { Text("Reset to default") }
                    }
                    Button(onClick = onDismiss) { Text("Done") }
                }
            }
    }
}

internal const val VALUE_DETAILS_TITLE = "Value Details"

internal const val EDIT_PARAMETER_TITLE = "Edit parameter"

internal fun valueDetailsNotes(fact: Fact): List<String> = listOfNotNull(
    fact.longDescription.ifBlank { fact.valueDetails }.takeIf { it.isNotBlank() },
    parameterRangeLine(fact).takeIf { it.isNotBlank() },
) + parameterRebootNotes(fact)

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
internal fun ValueDetailsSheet(fact: Fact, onWrite: () -> Unit, onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }
    val default = fact.defaultValueString.toDoubleOrNull()?.takeIf { !fact.readOnly }
    AircastSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(VALUE_DETAILS_TITLE, style = MaterialTheme.typography.titleLarge)
            FactRow(fact = fact, title = fact.heading, subtitle = "", fieldModifier = Modifier.fillMaxWidth().padding(vertical = 8.dp), onWrite = onWrite)
            valueDetailsNotes(fact).forEach { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            if (!fact.readOnly) Text(IN_FLIGHT_WARNING, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.aircast.warning)
            refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End)) {
                default?.let { value ->
                    TextButton(onClick = {
                        scope.launch {
                            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, value) }
                            if (refusal == null) {
                                onWrite()
                                onDismiss()
                            }
                        }
                    }) { Text("Reset to default") }
                }
                Button(onClick = onDismiss) { Text("Done") }
            }
        }
    }
}
