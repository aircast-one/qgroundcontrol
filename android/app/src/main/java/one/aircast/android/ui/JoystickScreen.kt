package one.aircast.android.ui

import one.aircast.android.R

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val JOYSTICK_VIEW = "view.joystick"
internal const val JOYSTICK_SCREEN = "joystick"
internal const val JOYSTICK_SELECT = "joystick.select"
internal const val JOYSTICK_ENABLE = "joystick.enable"
internal const val JOYSTICK_SETTING = "joystick.setting"
internal const val JOYSTICK_CALIBRATION = "joystick.calibration"
internal const val JOYSTICK_BUTTON_ACTION = "joystick.buttonAction"
internal const val JOYSTICK_BUTTON_REPEAT = "joystick.buttonRepeat"
internal const val NO_ACTION = "No Action"
private const val JOYSTICK_POLL_MS = 100L
private const val AXIS_RANGE = 32767f

internal data class JoystickSetting(val name: String, val type: String, val label: String, val units: String, val value: Any?)

internal data class JoystickAxis(val index: Int, val raw: Int?, val function: String)

internal data class JoystickButton(val index: Int, val action: String, val repeat: Boolean, val pressed: Boolean)

internal data class AssignableAction(val action: String, val canRepeat: Boolean)

internal data class JoystickCalibration(
    val calibrating: Boolean,
    val statusText: String,
    val nextText: String,
    val nextEnabled: Boolean,
    val cancelEnabled: Boolean,
    val oneSidedVisible: Boolean,
    val stickPositions: List<Int> = List(4) { 0 },
    val singleStick: Boolean = false,
)

internal fun joystickCalibration(json: JSONObject?): JoystickCalibration = JoystickCalibration(
    calibrating = json?.optBoolean("calibrating") == true,
    statusText = json?.optText("statusText").orEmpty(),
    nextText = json?.optText("nextText")?.ifBlank { null } ?: "Calibrate",
    nextEnabled = json?.optBoolean("nextEnabled", true) ?: true,
    cancelEnabled = json?.optBoolean("cancelEnabled") == true,
    oneSidedVisible = json?.optBoolean("oneSidedVisible") == true,
    stickPositions = json?.optJSONArray("stickPositions")?.let { a -> (0 until 4).map { a.optInt(it) } } ?: List(4) { 0 },
    singleStick = json?.optBoolean("singleStickDisplay") == true,
)

internal data class JoystickPage(
    val names: List<String>,
    val active: String?,
    val vehicle: Boolean,
    val enabled: Boolean,
    val calibrated: Boolean,
    val settings: List<JoystickSetting>,
    val axes: List<JoystickAxis>,
    val armed: Boolean = false,
    val calibration: JoystickCalibration = joystickCalibration(null),
    val transmitterMode: Int = 2,
    val buttons: List<JoystickButton> = emptyList(),
    val actions: List<AssignableAction> = emptyList(),
)

internal fun joystickPage(view: JSONObject?): JoystickPage? = view?.takeIf { it.optBoolean("available") }?.let {
    val names = it.optJSONArray("names")
    val settings = it.optJSONArray("settings")
    val axes = it.optJSONObject("state")?.optJSONArray("axes")
    JoystickPage(
        names = (0 until (names?.length() ?: 0)).map { at -> names!!.optString(at) },
        active = if (it.isNull("active")) null else it.optText("active"),
        vehicle = it.optBoolean("vehicle"),
        enabled = it.optBoolean("enabled"),
        calibrated = it.optBoolean("calibrated"),
        settings = (0 until (settings?.length() ?: 0)).mapNotNull { at ->
            settings!!.optJSONObject(at)?.takeIf { s -> s.optBoolean("visible", true) }?.let { s -> JoystickSetting(s.optText("name"), s.optText("type"), s.optText("label"), s.optText("units"), s.opt("value")) }
        },
        axes = (0 until (axes?.length() ?: 0)).mapNotNull { at ->
            axes!!.optJSONObject(at)?.let { a -> JoystickAxis(a.optInt("index"), if (a.isNull("raw")) null else a.optInt("raw"), a.optText("function")) }
        },
        armed = it.optBoolean("armed"),
        calibration = joystickCalibration(it.optJSONObject("calibration")),
        transmitterMode = it.optInt("transmitterMode", 2),
        buttons = it.optJSONObject("state")?.optJSONArray("buttons")?.let { list ->
            (0 until list.length()).mapNotNull { at ->
                list.optJSONObject(at)?.let { b -> JoystickButton(b.optInt("index"), if (b.isNull("action")) NO_ACTION else b.optText("action"), b.optBoolean("repeat"), b.optText("event").let { e -> e == "down" || e == "repeat" }) }
            }
        }.orEmpty(),
        actions = it.optJSONArray("assignableActions")?.let { list ->
            (0 until list.length()).mapNotNull { at -> list.optJSONObject(at)?.let { a -> AssignableAction(a.optText("action"), a.optBoolean("canRepeat")) } }
        }.orEmpty(),
    )
}

private val BASIC_SETTINGS = listOf("throttleModeCenterZero", "throttleSmoothing", "exponentialPct", "negativeThrust")
private val ADVANCED_SETTINGS = listOf("circleCorrection", "axisFrequencyHz", "buttonFrequencyHz", "useDeadband")
private val EXTENSION_SETTINGS = listOf("enableManualControlPitchExtension", "enableManualControlRollExtension")
private val ADDITIONAL_SETTINGS = (1..6).map { "enableAdditionalAxis$it" }

@Composable
fun JoystickScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var page by remember { mutableStateOf<JoystickPage?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var advanced by remember { mutableStateOf(false) }
    var offerEnable by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(revision) {
        page = withContext(Dispatchers.Default) { joystickPage(Qgc.get(JOYSTICK_VIEW)) }
        delay(JOYSTICK_POLL_MS)
        revision++
    }

    fun act(path: String, vararg args: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) } }
    }

    val read = page ?: return
    if (read.names.isEmpty() || read.active == null) {
        EmptyState(R.drawable.ic_gamepad, "No joystick", "No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB.", modifier)
        return
    }
    val setting = { name: String -> read.settings.find { it.name == name } }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (read.names.size > 1) JoystickPicker(read) { act(JOYSTICK_SELECT, it) }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Checkbox(checked = read.enabled, enabled = read.vehicle, onCheckedChange = { act(JOYSTICK_ENABLE, it) })
            Text("Enable")
            if (!read.vehicle) Text("Not currently available", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Text(if (read.calibrated) "Calibrated" else "Requires Calibration", color = if (read.calibrated) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.error)
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }

        if (!read.armed) {
            SectionHeader("Calibration")
            CalibrationPanel(read) { op ->
                scope.launch {
                    val answer = withContext(Dispatchers.Default) { Qgc.call(JOYSTICK_CALIBRATION, op) }
                    refusal = answer?.takeIf { !it.optBoolean("ok") }?.optText("reason")
                    if (answer?.optBoolean("completed") == true && !read.enabled) offerEnable = answer.optText("name")
                }
            }
            TransmitterModeRow(read.transmitterMode) { act(JOYSTICK_SETTING, "transmitterMode", it) }
        }

        SectionHeader("Axis Monitor")
        read.axes.forEach { axis ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(axis.function.ifBlank { "Axis ${axis.index + 1}" }, modifier = Modifier.width(96.dp))
                LinearProgressIndicator(progress = { ((axis.raw ?: 0) / AXIS_RANGE + 1f) / 2f }, modifier = Modifier.weight(1f))
            }
        }

        SectionHeader("Buttons")
        Text("Multiple buttons that have the same action must be pressed simultaneously to invoke the action.", style = MaterialTheme.typography.bodySmall)
        read.buttons.forEach { button ->
            ButtonRow(button, read.actions, read.calibrated, onAction = { act(JOYSTICK_BUTTON_ACTION, button.index, it) }, onRepeat = { act(JOYSTICK_BUTTON_REPEAT, button.index, it) })
        }

        SectionHeader("Settings")
        BASIC_SETTINGS.mapNotNull(setting).forEach { SettingRow(it) { value -> act(JOYSTICK_SETTING, it.name, value) } }
        OutlinedButton(onClick = { advanced = !advanced }) { Text("Advanced settings") }
        if (advanced) {
            ADVANCED_SETTINGS.mapNotNull(setting).forEach { SettingRow(it) { value -> act(JOYSTICK_SETTING, it.name, value) } }
            if (setting("useDeadband")?.value == true) {
                Text("Deadband can be set during the first step of calibration by gently wiggling each axis. ", style = MaterialTheme.typography.bodySmall)
            }
            Text("MANUAL_CONTROL Extensions", style = MaterialTheme.typography.titleSmall)
            EXTENSION_SETTINGS.mapNotNull(setting).zip(listOf("Pitch", "Roll")).forEach { (s, label) -> SettingRow(s.copy(label = label)) { value -> act(JOYSTICK_SETTING, s.name, value) } }
            Text("Additional axes", style = MaterialTheme.typography.titleSmall)
            val viaRc = (setting("additionalAxesFunction")?.value as? Number)?.toInt() == 1
            Row(verticalAlignment = Alignment.CenterVertically) {
                androidx.compose.material3.RadioButton(selected = !viaRc, onClick = { act(JOYSTICK_SETTING, "additionalAxesFunction", 0) })
                Text("Send using MANUAL_CONTROL")
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                androidx.compose.material3.RadioButton(selected = viaRc, onClick = { act(JOYSTICK_SETTING, "additionalAxesFunction", 1) })
                Text("Send using RC_CHANNELS_OVERRIDE")
            }
            ADDITIONAL_SETTINGS.mapNotNull(setting).forEachIndexed { index, s ->
                SettingRow(s.copy(label = if (viaRc) "Channel ${index + 5}" else "Aux${index + 1}")) { value -> act(JOYSTICK_SETTING, s.name, value) }
            }
        }
    }
    offerEnable?.let { name ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { offerEnable = null },
            title = { Text("Enable joystick") },
            text = { Text("$name calibration is complete. Enable it now?") },
            confirmButton = {
                androidx.compose.material3.TextButton(onClick = {
                    offerEnable = null
                    act(JOYSTICK_ENABLE, true)
                }) { Text("Yes") }
            },
            dismissButton = { androidx.compose.material3.TextButton(onClick = { offerEnable = null }) { Text("No") } },
        )
    }
}

@Composable
private fun ButtonRow(button: JoystickButton, actions: List<AssignableAction>, calibrated: Boolean, onAction: (String) -> Unit, onRepeat: (Boolean) -> Unit) {
    var open by remember { mutableStateOf(false) }
    val canRepeat = actions.find { it.action == button.action }?.canRepeat == true
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            button.index.toString(),
            color = if (button.pressed) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.titleSmall,
            modifier = Modifier.width(32.dp),
        )
        Box(Modifier.weight(1f)) {
            OutlinedButton(onClick = { open = true }, modifier = Modifier.fillMaxWidth()) { Text(button.action, maxLines = 1) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                actions.forEach { option ->
                    DropdownMenuItem(text = { Text(option.action) }, onClick = {
                        open = false
                        onAction(option.action)
                    })
                }
            }
        }
        Checkbox(checked = button.repeat, enabled = canRepeat && calibrated, onCheckedChange = onRepeat)
        Text("Repeat", style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun CalibrationPanel(page: JoystickPage, onStep: (String) -> Unit) {
    val cal = page.calibration
    if (cal.calibrating) StickDiagram(cal.stickPositions, cal.singleStick)
    if (cal.statusText.isNotBlank()) Text(cal.statusText, style = MaterialTheme.typography.bodyMedium)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(enabled = cal.cancelEnabled, onClick = { onStep("cancel") }) { Text("Cancel") }
        if (cal.oneSidedVisible) OutlinedButton(onClick = { onStep("oneSided") }) { Text("One-Sided") }
        androidx.compose.material3.Button(enabled = cal.nextEnabled, onClick = { onStep("next") }) { Text(cal.nextText) }
    }
}

@Composable
private fun TransmitterModeRow(mode: Int, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Mode", modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(onClick = { open = true }) { Text("Mode $mode") }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                (1..4).forEach { m ->
                    DropdownMenuItem(text = { Text("Mode $m") }, onClick = {
                        open = false
                        onPick(m)
                    })
                }
            }
        }
    }
}

@Composable
private fun JoystickPicker(page: JoystickPage, onPick: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        OutlinedButton(onClick = { open = true }) { Text(page.active.orEmpty()) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            page.names.forEach { name ->
                DropdownMenuItem(text = { Text(name) }, onClick = {
                    open = false
                    onPick(name)
                })
            }
        }
    }
}

@Composable
private fun SettingRow(setting: JoystickSetting, onChange: (Any) -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(setting.label, modifier = Modifier.weight(1f))
        when (setting.type) {
            "bool" -> Switch(checked = setting.value == true, onCheckedChange = onChange)
            else -> {
                var typed by remember(setting.value) { mutableStateOf((setting.value as? Number)?.toString().orEmpty()) }
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typed = it },
                    singleLine = true,
                    suffix = { Text(setting.units) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    keyboardActions = KeyboardActions(onDone = { typed.toDoubleOrNull()?.let(onChange) }),
                    modifier = Modifier.width(140.dp),
                )
            }
        }
    }
}

private val STICK_BOX = 72.dp

@Composable
internal fun StickDiagram(positions: List<Int>, single: Boolean) {
    val ring = MaterialTheme.colorScheme.outline
    val knob = MaterialTheme.colorScheme.primary
    Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
        positions.chunked(2).take(if (single) 1 else 2).forEach { (x, y) ->
            androidx.compose.foundation.Canvas(Modifier.size(STICK_BOX)) {
                val half = size.minDimension / 2f
                drawRect(ring, style = androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))
                drawCircle(knob, radius = half * 0.18f, center = center + androidx.compose.ui.geometry.Offset(x * half * 0.75f, -y * half * 0.75f))
            }
        }
    }
}
