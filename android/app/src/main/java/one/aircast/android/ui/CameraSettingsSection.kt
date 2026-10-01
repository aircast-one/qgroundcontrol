package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val CAMERA_SETTINGS_VIEW = "view.cameraSettings"
internal const val CAMERA_SETTING_SET = "cameraSettings.set"
private const val CAMERA_SETTINGS_POLL_MS = 1000L

internal data class CameraOption(val label: String, val value: Any)

internal sealed interface CameraSettingControl {
    data class Toggle(val on: Boolean) : CameraSettingControl
    data class Choice(val options: List<CameraOption>, val selected: Int) : CameraSettingControl
    data class Range(val min: Double, val max: Double, val step: Double, val value: Double) : CameraSettingControl
    data class Entry(val text: String) : CameraSettingControl
}

internal data class CameraSetting(val name: String, val label: String, val readOnly: Boolean, val control: CameraSettingControl)

private fun JSONObject.number(key: String): Double? = if (isNull(key) || !has(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun cameraSettingControl(parameter: JSONObject): CameraSettingControl {
    val options = parameter.optJSONArray("options")
    val listed = (0 until (options?.length() ?: 0)).mapNotNull { at -> options!!.optJSONObject(at)?.let { CameraOption(it.optText("label"), it.opt("value")) } }
    val value = parameter.opt("value")
    val step = parameter.number("step")
    val min = parameter.number("min")
    val max = parameter.number("max")
    return when {
        parameter.optBoolean("isBool") -> CameraSettingControl.Toggle(value == true || (value as? Number)?.toInt() == 1)
        listed.isNotEmpty() -> CameraSettingControl.Choice(listed, parameter.optInt("selected", -1))
        step != null && min != null && max != null -> CameraSettingControl.Range(min, max, step, (value as? Number)?.toDouble() ?: min)
        else -> CameraSettingControl.Entry(if (value == null || value == JSONObject.NULL) "" else value.toString())
    }
}

internal fun cameraSettings(view: JSONObject?): List<CameraSetting> {
    if (view?.optText("state") != "ready") return emptyList()
    val parameters = view.optJSONArray("parameters") ?: return emptyList()
    val byName = (0 until parameters.length()).mapNotNull { parameters.optJSONObject(it) }.associateBy { it.optText("name") }
    val active = view.optJSONArray("activeSettings") ?: return emptyList()
    return (0 until active.length()).mapNotNull { at ->
        byName[active.optString(at)]?.let { p -> CameraSetting(p.optText("name"), p.optText("description"), p.optBoolean("readOnly"), cameraSettingControl(p)) }
    }
}

@Composable
internal fun CameraDefinitionSettings() {
    var revision by remember { mutableIntStateOf(0) }
    var settings by remember { mutableStateOf<List<CameraSetting>>(emptyList()) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) {
        settings = withContext(Dispatchers.Default) { cameraSettings(Qgc.get(CAMERA_SETTINGS_VIEW)) }
        delay(CAMERA_SETTINGS_POLL_MS)
        revision++
    }
    fun write(name: String, value: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(CAMERA_SETTING_SET, name, value) } }
    }
    Column(Modifier.padding(horizontal = 20.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        settings.forEach { setting -> CameraSettingRow(setting) { write(setting.name, it) } }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
    }
}

@Composable
private fun CameraSettingRow(setting: CameraSetting, onWrite: (Any) -> Unit) {
    val enabled = !setting.readOnly
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(setting.label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        when (val control = setting.control) {
            is CameraSettingControl.Toggle -> Switch(checked = control.on, enabled = enabled, onCheckedChange = { onWrite(if (it) 1 else 0) })
            is CameraSettingControl.Choice -> {
                var open by remember { mutableStateOf(false) }
                Box {
                    OutlinedButton(enabled = enabled, onClick = { open = true }) { Text(control.options.getOrNull(control.selected)?.label.orEmpty()) }
                    DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                        control.options.forEach { option ->
                            DropdownMenuItem(text = { Text(option.label) }, onClick = {
                                open = false
                                onWrite(option.value)
                            })
                        }
                    }
                }
            }
            is CameraSettingControl.Range -> {
                var dragged by remember(control.value) { mutableFloatStateOf(control.value.toFloat()) }
                val steps = ((control.max - control.min) / control.step).toInt() - 1
                Slider(
                    value = dragged,
                    onValueChange = { dragged = it },
                    onValueChangeFinished = { onWrite(dragged.toDouble()) },
                    valueRange = control.min.toFloat()..control.max.toFloat(),
                    steps = steps.coerceIn(0, 1000),
                    enabled = enabled,
                    modifier = Modifier.width(180.dp),
                )
            }
            is CameraSettingControl.Entry -> {
                var typed by remember(control.text) { mutableStateOf(control.text) }
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typed = it },
                    singleLine = true,
                    enabled = enabled,
                    keyboardActions = KeyboardActions(onDone = { onWrite(typed.toDoubleOrNull() ?: typed) }),
                    modifier = Modifier.width(180.dp),
                )
            }
        }
    }
}
