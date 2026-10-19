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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.openApp
import one.aircast.android.stopApp
import one.aircast.map.optText
import org.json.JSONObject

internal const val JOYSTICK_VIEW = "view.joystick"
internal const val JOYSTICK_SETUP_VIEW = "view.joystickSetup"
internal const val JOYSTICK_SCREEN = "joystick"
internal const val JOYSTICK_SELECT = "joystick.select"
internal const val JOYSTICK_ENABLE = "joystick.enable"
internal const val JOYSTICK_SETTING = "joystick.setting"
internal const val JOYSTICK_CALIBRATION = "joystick.calibration"
internal const val JOYSTICK_BUTTON_ACTION = "joystick.buttonAction"
internal const val JOYSTICK_BUTTON_REPEAT = "joystick.buttonRepeat"
internal const val JOYSTICK_BUILT_IN_STICKS = "joystick.builtInSticks"
private const val TAKE_OVER_TRIES = 10
private const val TAKE_OVER_RETRY_MS = 200L
internal const val NO_ACTION = "No Action"
private const val AXIS_RANGE = 32767f

internal data class JoystickSetting(val name: String, val type: String, val label: String, val units: String, val value: Any?, val slider: one.aircast.android.bridge.FactSlider? = null)

internal data class JoystickAxis(val index: Int, val raw: Int?, val label: String)

internal data class JoystickButton(val index: Int, val action: String, val repeat: Boolean, val pressed: Boolean)

internal data class AssignableAction(val action: String, val canRepeat: Boolean)

internal data class BuiltInSticks(val status: String, val reason: String?, val owner: String?, val receiving: Boolean) {
    val live: Boolean get() = status == "live"
}

internal fun builtInSticks(json: JSONObject?): BuiltInSticks? = json?.takeIf { it.optBoolean("available") }?.let {
    BuiltInSticks(it.optText("status"), it.optText("reason").ifBlank { null }, it.optText("owner").ifBlank { null }, it.optBoolean("receiving"))
}

internal fun builtInSticksText(sticks: BuiltInSticks): String = when {
    sticks.live && sticks.receiving -> "The AX12's own sticks and switches are flying the aircraft over this app's link."
    sticks.live -> "Waiting for the sticks."
    else -> sticks.reason ?: "Fly with the AX12's own sticks and switches over any link, 4G included."
}

internal data class JoystickCalibration(
    val calibrating: Boolean,
    val status: StatusLines,
    val nextText: String,
    val nextEnabled: Boolean,
    val cancelEnabled: Boolean,
    val oneSidedVisible: Boolean,
    val stickPositions: List<Int> = List(4) { 0 },
    val singleStick: Boolean = false,
    val step: Int? = null,
    val steps: Int? = null,
    val holding: Double? = null,
    val checking: Boolean = false,
    val warning: String? = null,
    val live: Map<String, Double> = FUNCTION_LABELS.keys.associateWith { 0.0 },
    val livePositions: List<Double> = List(4) { 0.0 },
)

private val FUNCTION_LABELS = mapOf("roll" to "Roll", "pitch" to "Pitch", "yaw" to "Yaw", "throttle" to "Throttle")

internal fun joystickCalibration(json: JSONObject?): JoystickCalibration = JoystickCalibration(
    calibrating = json?.optBoolean("calibrating") == true,
    status = statusLines(json?.optJSONObject("status")),
    nextText = json?.optText("nextText")?.ifBlank { null } ?: "Calibrate",
    nextEnabled = json?.optBoolean("nextEnabled", true) ?: true,
    cancelEnabled = json?.optBoolean("cancelEnabled") == true,
    oneSidedVisible = json?.optBoolean("oneSidedVisible") == true,
    stickPositions = json?.optJSONArray("stickPositions")?.let { a -> (0 until 4).map { a.optInt(it) } } ?: List(4) { 0 },
    singleStick = json?.optBoolean("singleStickDisplay") == true,
    step = json?.takeUnless { it.isNull("step") }?.optInt("step"),
    steps = json?.takeUnless { it.isNull("steps") }?.optInt("steps"),
    holding = json?.takeUnless { it.isNull("holding") }?.optDouble("holding", 0.0),
    checking = json?.optBoolean("checking") == true,
    warning = json?.optText("warning")?.ifBlank { null },
    live = json?.optJSONObject("live").let { live -> FUNCTION_LABELS.keys.associateWith { live?.optDouble(it, 0.0) ?: 0.0 } },
    livePositions = json?.optJSONArray("livePositions")?.let { a -> (0 until 4).map { a.optDouble(it, 0.0) } } ?: List(4) { 0.0 },
)

internal fun functionLabel(id: String): String = FUNCTION_LABELS[id] ?: id

internal data class StatusLines(val bullets: Boolean, val lines: List<String>)

internal fun statusLines(json: JSONObject?): StatusLines =
    StatusLines(json?.optBoolean("bullets") == true, json?.optJSONArray("lines")?.let { a -> (0 until a.length()).map { a.optText(it) } }.orEmpty())

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
    val commanding: Boolean = false,
    val sendingText: String? = null,
    val transmitterMode: Int = 2,
    val transmitterModeText: String? = null,
    val buttons: List<JoystickButton> = emptyList(),
    val actions: List<AssignableAction> = emptyList(),
    val builtInSticks: BuiltInSticks? = null,
    val standbyText: String? = null,
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
            settings!!.optJSONObject(at)?.takeIf { s -> s.optBoolean("visible", true) }?.let { s -> JoystickSetting(s.optText("name"), s.optText("type"), s.optText("label"), s.optText("units"), s.opt("value"), s.optJSONObject("slider")?.let { range -> factSlider(range, "") }) }
        },
        axes = (0 until (axes?.length() ?: 0)).mapNotNull { at ->
            axes!!.optJSONObject(at)?.let { a -> JoystickAxis(a.optInt("index"), if (a.isNull("raw")) null else a.optInt("raw"), a.optText("label")) }
        },
        armed = it.optBoolean("armed"),
        calibration = joystickCalibration(it.optJSONObject("calibration")),
        commanding = it.optJSONObject("state")?.optBoolean("commanding") == true,
        sendingText = it.optText("sendingText").ifBlank { null },
        transmitterMode = it.optInt("transmitterMode", 2),
        transmitterModeText = it.optText("transmitterModeText").ifBlank { null },
        buttons = it.optJSONObject("state")?.optJSONArray("buttons")?.let { list ->
            (0 until list.length()).mapNotNull { at ->
                list.optJSONObject(at)?.let { b -> JoystickButton(b.optInt("index"), if (b.isNull("action")) NO_ACTION else b.optText("action"), b.optBoolean("repeat"), b.optText("event").let { e -> e == "down" || e == "repeat" }) }
            }
        }.orEmpty(),
        actions = it.optJSONArray("assignableActions")?.let { list ->
            (0 until list.length()).mapNotNull { at -> list.optJSONObject(at)?.let { a -> AssignableAction(a.optText("action"), a.optBoolean("canRepeat")) } }
        }.orEmpty(),
        builtInSticks = builtInSticks(it.optJSONObject("builtInSticks")),
        standbyText = it.optJSONObject("standby")?.optText("text")?.ifBlank { null },
    )
}

private val BASIC_SETTINGS = listOf("standby", "throttleModeCenterZero", "throttleSmoothing", "exponentialPct", "negativeThrust")
private val ADVANCED_SETTINGS = listOf("circleCorrection", "axisFrequencyHz", "buttonFrequencyHz", "useDeadband")
private val EXTENSION_SETTINGS = listOf("enableManualControlPitchExtension", "enableManualControlRollExtension")
private val ADDITIONAL_SETTINGS = (1..6).map { "enableAdditionalAxis$it" }

@Composable
fun JoystickScreen(modifier: Modifier = Modifier) {
    val page by qgcDerived(JOYSTICK_SETUP_VIEW, ::joystickPage)
    var refusal by remember { mutableStateOf<String?>(null) }
    var advanced by remember { mutableStateOf(false) }
    var offerEnable by remember { mutableStateOf<String?>(null) }
    var confirmTakeOver by remember { mutableStateOf<String?>(null) }
    var stoppedOwner by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun act(path: String, vararg args: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) } }
    }

    val read = page ?: return
    val context = LocalContext.current
    val sticksSection: @Composable () -> Unit = {
        read.builtInSticks?.let { sticks ->
            BuiltInSticksSection(
                sticks,
                stopped = stoppedOwner != null,
                onToggle = { act(JOYSTICK_BUILT_IN_STICKS, it) },
                onTakeOver = { confirmTakeOver = sticks.owner },
                onOpenOwner = {
                    stoppedOwner?.let { owner -> openApp(context, owner) }
                    stoppedOwner = null
                },
            )
        }
    }
    confirmTakeOver?.let { owner ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { confirmTakeOver = null },
            title = { Text("Stop RadioMaster's radio app?") },
            text = { Text("The AX12's sticks can only be read while RadioMaster's radio app is closed, and stopping it may stop the AX12's own radio link to the aircraft. Have the aircraft on 4G or another link first. To get the radio link back, turn the built-in sticks off and open RadioMaster's app.") },
            confirmButton = {
                androidx.compose.material3.TextButton(onClick = {
                    confirmTakeOver = null
                    stoppedOwner = owner
                    scope.launch { refusal = takeOverSticks(owner) }
                }) { Text("Stop it") }
            },
            dismissButton = { androidx.compose.material3.TextButton(onClick = { confirmTakeOver = null }) { Text("Cancel") } },
        )
    }
    if (read.names.isEmpty() || read.active == null) {
        if (read.builtInSticks == null) {
            EmptyState(R.drawable.ic_gamepad, "No joystick", "No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB.", modifier)
        } else {
            Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                sticksSection()
                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        }
        return
    }
    val setting = { name: String -> read.settings.find { it.name == name } }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if (read.names.size > 1) JoystickPicker(read) { act(JOYSTICK_SELECT, it) }
        sticksSection()
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Checkbox(checked = read.enabled, enabled = read.vehicle && (read.calibrated || read.enabled), onCheckedChange = { act(JOYSTICK_ENABLE, it) })
            Text("Enable")
            if (!read.vehicle) Text("Not currently available", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Text(if (read.calibrated) "Calibrated · saved on this device" else "Calibrate before enabling", color = if (read.calibrated) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.error)
        read.sendingText?.let { Text(it, color = if (read.commanding) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error) }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }

        if (!read.armed) {
            SectionHeader("Calibration")
            if (!read.calibration.calibrating) TransmitterModeRow(read.transmitterMode, read.transmitterModeText) { act(JOYSTICK_SETTING, "transmitterMode", it) }
            CalibrationPanel(read) { op ->
                scope.launch {
                    val answer = withContext(Dispatchers.Default) { Qgc.call(JOYSTICK_CALIBRATION, op) }
                    refusal = answer?.takeIf { !it.optBoolean("ok") }?.optText("reason")
                    if (answer?.optBoolean("completed") == true && !read.enabled) offerEnable = answer.optText("name")
                }
            }
        }

        if (read.calibration.calibrating) return@Column
        SectionHeader("Axis monitor")
        read.axes.forEach { axis ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(axis.label, modifier = Modifier.width(96.dp))
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
        read.standbyText?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
        OutlinedButton(onClick = { advanced = !advanced }) { Text("Advanced settings") }
        if (advanced) {
            ADVANCED_SETTINGS.mapNotNull(setting).forEach { SettingRow(it) { value -> act(JOYSTICK_SETTING, it.name, value) } }
            if (setting("useDeadband")?.value == true) {
                Text("Deadband can be set during the first step of calibration by gently wiggling each axis. ", style = MaterialTheme.typography.bodySmall)
            }
            Text("MANUAL_CONTROL Extensions", style = MaterialTheme.typography.titleSmall)
            EXTENSION_SETTINGS.mapNotNull(setting).zip(listOf("Pitch", "Roll")).forEach { (s, label) -> SettingRow(s.copy(label = label)) { value -> act(JOYSTICK_SETTING, s.name, value) } }
            Text("Additional axes", style = MaterialTheme.typography.titleSmall)
            val additionalAxes = setting("additionalAxesFunction")
            val viaRc = additionalAxes == null || (additionalAxes.value as? Number)?.toInt() == 1
            if (additionalAxes != null) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    androidx.compose.material3.RadioButton(selected = !viaRc, onClick = { act(JOYSTICK_SETTING, "additionalAxesFunction", 0) })
                    Text("Send using MANUAL_CONTROL")
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    androidx.compose.material3.RadioButton(selected = viaRc, onClick = { act(JOYSTICK_SETTING, "additionalAxesFunction", 1) })
                    Text("Send using RC_CHANNELS_OVERRIDE")
                }
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

private suspend fun takeOverSticks(owner: String): String? = withContext(Dispatchers.IO) {
    if (!stopApp(owner)) return@withContext "RadioMaster's radio app could not be stopped. That needs the AX12's factory root."
    repeat(TAKE_OVER_TRIES) {
        Qgc.refusalOf(JOYSTICK_BUILT_IN_STICKS, true) ?: return@withContext null
        delay(TAKE_OVER_RETRY_MS)
    }
    Qgc.refusalOf(JOYSTICK_BUILT_IN_STICKS, true)
}

@Composable
private fun BuiltInSticksSection(sticks: BuiltInSticks, stopped: Boolean, onToggle: (Boolean) -> Unit, onTakeOver: () -> Unit, onOpenOwner: () -> Unit) {
    SectionHeader("Built-in sticks")
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Switch(checked = sticks.live, onCheckedChange = onToggle)
        Text(builtInSticksText(sticks), modifier = Modifier.weight(1f))
    }
    if (sticks.status == "busy" && sticks.owner != null) OutlinedButton(onClick = onTakeOver) { Text("Stop RadioMaster's radio app") }
    if (stopped && !sticks.live) OutlinedButton(onClick = onOpenOwner) { Text("Open RadioMaster's radio app") }
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
private fun StatusText(status: StatusLines) {
    val (bullets, lines) = status
    when {
        lines.isEmpty() -> Unit
        bullets -> Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            lines.map { line ->
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("•", style = MaterialTheme.typography.bodyMedium)
                    Text(line, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
        else -> Text(lines.joinToString(" "), style = MaterialTheme.typography.bodyMedium)
    }
}

private val CHECK_ROWS = listOf(Triple("roll", "Left", "Right"), Triple("pitch", "Down", "Up"), Triple("yaw", "Left", "Right"), Triple("throttle", "Down", "Up"))

@Composable
private fun StickCheck(live: Map<String, Double>) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        CHECK_ROWS.map { (key, low, high) ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(functionLabel(key), modifier = Modifier.width(80.dp))
                Text(low, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.End, modifier = Modifier.width(40.dp))
                LinearProgressIndicator(progress = { (((live[key] ?: 0.0) + 1) / 2).toFloat() }, modifier = Modifier.weight(1f).semantics { contentDescription = "${functionLabel(key)} stick" })
                Text(high, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.width(40.dp))
            }
        }
    }
}

@Composable
private fun CalibrationPanel(page: JoystickPage, onStep: (String) -> Unit) {
    val cal = page.calibration
    if (cal.calibrating && cal.step != null && cal.steps != null) {
        Text("Step ${cal.step} of ${cal.steps}", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    if (cal.calibrating && !cal.checking) {
        StickDiagram(cal.stickPositions, cal.singleStick, cal.livePositions)
        Text("Ring: where to move the stick. Dot: where your stick is.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    StatusText(cal.status)
    cal.holding?.let { held ->
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            LinearProgressIndicator(progress = { held.toFloat() }, modifier = Modifier.width(160.dp).semantics { contentDescription = "Holding the stick" })
            Text("Got it, hold it there", style = MaterialTheme.typography.bodySmall)
        }
    }
    if (cal.checking) StickCheck(cal.live)
    cal.warning?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    if (cal.checking) Text("Save keeps this calibration on this device.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (cal.calibrating) OutlinedButton(enabled = cal.cancelEnabled, onClick = { onStep("cancel") }) { Text("Cancel") }
        if (cal.oneSidedVisible) OutlinedButton(onClick = { onStep("oneSided") }) { Text("One-Sided") }
        androidx.compose.material3.Button(enabled = cal.nextEnabled, onClick = { onStep("next") }) { Text(cal.nextText) }
    }
}

@Composable
private fun TransmitterModeRow(mode: Int, text: String?, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.weight(1f)) {
            Text("Stick mode")
            Text(text.orEmpty(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
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
    Column(Modifier.fillMaxWidth()) {
        SettingField(setting, onChange)
        setting.slider?.takeIf { setting.type != "bool" }?.let { slider -> FieldSlider((setting.value as? Number)?.toFloat(), slider, enabled = true) { onChange(it) } }
    }
}

@Composable
private fun SettingField(setting: JoystickSetting, onChange: (Any) -> Unit) {
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
internal fun StickDiagram(positions: List<Int>, single: Boolean, live: List<Double>? = null) {
    val ring = MaterialTheme.colorScheme.outline
    val knob = MaterialTheme.colorScheme.primary
    Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
        listOf(0, 2).take(if (single) 1 else 2).map { at ->
            androidx.compose.foundation.Canvas(Modifier.size(STICK_BOX)) {
                val half = size.minDimension / 2f
                val stick = { values: List<Number> -> center + Offset(values[at].toFloat() * half * 0.75f, -values[at + 1].toFloat() * half * 0.75f) }
                drawRect(ring, style = Stroke(1.dp.toPx()))
                if (live == null) {
                    drawCircle(knob, radius = half * 0.18f, center = stick(positions))
                } else {
                    drawCircle(knob, radius = half * 0.24f, center = stick(positions), style = Stroke(2.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 3.dp.toPx()))))
                    drawCircle(knob, radius = half * 0.14f, center = stick(live))
                }
            }
        }
    }
}
