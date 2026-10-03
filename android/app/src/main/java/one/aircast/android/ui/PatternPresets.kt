package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

internal const val SAVE_PRESET = "core.plan.savePreset"
internal const val APPLY_PRESET = "core.plan.applyPreset"
internal const val DELETE_PRESET = "core.plan.deletePreset"

internal fun presetKind(view: JSONObject?): String? = view?.optString("presetKind")?.takeIf { it.isNotBlank() && it != "null" }

internal fun presetNames(view: JSONObject?): List<String> {
    val names = view?.optJSONArray("names") ?: return emptyList()
    return (0 until names.length()).map { names.optString(it) }
}

@Composable
internal fun PatternPresets(index: Int, kind: String, onApplied: () -> Unit) {
    var revision by remember { mutableIntStateOf(0) }
    var names by remember { mutableStateOf<List<String>>(emptyList()) }
    var chosen by remember { mutableStateOf<String?>(null) }
    var picking by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf<String?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(kind, revision) {
        names = withContext(Dispatchers.Default) { presetNames(Qgc.get("view.patternPresets($kind)")) }
        chosen = chosen?.takeIf { it in names } ?: names.firstOrNull()
    }
    fun act(path: String, vararg args: Any, then: () -> Unit = {}) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            revision++
            if (refusal == null) then()
        }
    }
    Column(Modifier.padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("Presets", style = MaterialTheme.typography.titleSmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Preset", modifier = Modifier.padding(top = 12.dp))
            Box {
                OutlinedButton(enabled = names.isNotEmpty(), onClick = { picking = true }) { Text(chosen ?: "None") }
                DropdownMenu(expanded = picking, onDismissRequest = { picking = false }) {
                    names.forEach { name ->
                        DropdownMenuItem(text = { Text(name) }, onClick = {
                            chosen = name
                            picking = false
                        })
                    }
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(enabled = chosen != null, onClick = { chosen?.let { act(APPLY_PRESET, index, it, then = onApplied) } }) { Text("Apply preset") }
            TextButton(enabled = chosen != null, onClick = { deleting = chosen }) { Text("Delete preset") }
        }
        TextButton(onClick = { saving = true }) { Text("Save settings as new preset") }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    }
    deleting?.let { name ->
        AlertDialog(
            onDismissRequest = { deleting = null },
            title = { Text("Delete preset") },
            text = { Text("Are you sure you want to delete '$name' preset?") },
            confirmButton = {
                TextButton(onClick = {
                    deleting = null
                    act(DELETE_PRESET, kind, name)
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { deleting = null }) { Text("Cancel") } },
        )
    }
    if (saving) {
        var typed by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { saving = false },
            title = { Text("Save preset") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Save the current settings as a named preset.")
                    OutlinedTextField(value = typed, onValueChange = { typed = it }, label = { Text("Preset name") }, placeholder = { Text("Enter preset name") }, singleLine = true)
                    presetNameError(typed)?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                }
            },
            confirmButton = {
                TextButton(enabled = presetNameError(typed) == null, onClick = {
                    saving = false
                    act(SAVE_PRESET, index, typed.trim()) { chosen = typed.trim() }
                }) { Text("Save") }
            },
            dismissButton = { TextButton(onClick = { saving = false }) { Text("Cancel") } },
        )
    }
}

internal fun presetNameError(typed: String): String? = when {
    typed.trim().isEmpty() -> "Preset name cannot be blank."
    typed.contains('/') -> "Preset name cannot include the \"/\" character."
    else -> null
}
