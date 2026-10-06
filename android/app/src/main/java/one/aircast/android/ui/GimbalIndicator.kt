package one.aircast.android.ui

import androidx.compose.foundation.clickable
import one.aircast.android.bridge.SetupCommands
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.TextButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import one.aircast.map.AircastSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.refusal
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

internal const val GIMBAL_INDICATOR_PATH = "view.gimbalIndicator"
internal const val OTHERS_HAVE_CONTROL = "othersHaveControl"

internal val gimbalAsksForControl = kotlinx.coroutines.flow.MutableStateFlow(false)

internal fun serialAsks(seen: Long?, serial: Long): Boolean = seen != null && serial > seen

internal fun gimbalRefusal(answer: JSONObject?): String? {
    if (answer?.optText("refusal") == OTHERS_HAVE_CONTROL) {
        gimbalAsksForControl.value = true
        return null
    }
    return refusal(answer)
}

internal data class GimbalChoice(val name: String, val managerCompid: Int, val deviceId: Int, val active: Boolean)

internal data class GimbalIndicatorState(
    val statusText: String,
    val pitchText: String,
    val yawText: String,
    val yawLockLabel: String,
    val yawLocked: Boolean,
    val yawLockOffered: Boolean,
    val retractOffered: Boolean,
    val controlOffered: Boolean,
    val controlLabel: String,
    val haveControl: Boolean,
    val gimbals: List<GimbalChoice>,
    val pitchDegrees: Double? = null,
)

internal fun gimbalIndicator(view: JSONObject?): GimbalIndicatorState? =
    view?.takeIf { it.optBoolean("shown") }?.let {
        val listed = it.optJSONArray("gimbals")
        GimbalIndicatorState(
            statusText = it.optText("statusText"),
            pitchText = it.optText("pitchText"),
            yawText = it.optText("yawText"),
            yawLockLabel = it.optText("yawLockLabel"),
            yawLocked = it.optBoolean("yawLocked"),
            yawLockOffered = it.optBoolean("yawLockOffered"),
            retractOffered = it.optBoolean("retractOffered"),
            controlOffered = it.optBoolean("controlOffered"),
            controlLabel = it.optText("controlLabel"),
            haveControl = it.optBoolean("haveControl"),
            pitchDegrees = it.optDouble("pitchDegrees", Double.NaN).takeIf(Double::isFinite),
            gimbals = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
                listed!!.optJSONObject(index)?.let { g ->
                    GimbalChoice(g.optText("name"), g.optInt("managerCompid"), g.optInt("deviceId"), g.optBoolean("active"))
                }
            },
        )
    }

internal fun gimbalCellText(state: GimbalIndicatorState): String =
    listOf(state.gimbals.takeIf { it.size > 1 }?.firstOrNull { it.active }?.name.orEmpty(), state.statusText, state.pitchText, state.yawText)
        .filter { it.isNotBlank() }
        .joinToString(" · ")

@Composable
fun GimbalTakeControlDialog() {
    val asking by gimbalAsksForControl.collectAsState()
    val scope = rememberCoroutineScope()
    val view by qgcPath(GIMBAL_INDICATOR_PATH)
    val serial = view?.optLong("askSerial", -1L) ?: -1L
    var seen by remember { mutableStateOf<Long?>(null) }
    LaunchedEffect(serial) {
        if (serialAsks(seen, serial)) gimbalAsksForControl.value = true
        if (serial >= 0) seen = serial
    }
    if (asking) {
        AlertDialog(
            onDismissRequest = { gimbalAsksForControl.value = false },
            title = { Text("Request Gimbal Control?") },
            text = { Text("Command not sent. Another user has control of the gimbal.") },
            confirmButton = {
                TextButton(onClick = {
                    gimbalAsksForControl.value = false
                    scope.launch(Dispatchers.Default) { SetupCommands.takeGimbalControlRefusal() }
                }) { Text("Yes") }
            },
            dismissButton = { TextButton(onClick = { gimbalAsksForControl.value = false }) { Text("No") } },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GimbalIndicatorCell() {
    val view by qgcPath(GIMBAL_INDICATOR_PATH)
    val state = remember(view) { gimbalIndicator(view) } ?: return
    var open by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var settingsOpen by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    fun act(path: String, vararg args: Any?) {
        scope.launch {
            val answer = withContext(Dispatchers.Default) { Qgc.call(path, *args) }
            refusal = gimbalRefusal(answer)
            if (refusal == null) open = false
        }
    }


    Text(
        gimbalCellText(state),
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = 1,
        modifier = Modifier.clickable { open = true },
    )

    if (open) {
        AircastSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("Gimbal", style = MaterialTheme.typography.titleMedium)
                if (state.gimbals.size > 1) {
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        state.gimbals.forEach { gimbal ->
                            FilterChip(
                                selected = gimbal.active,
                                onClick = { act("gimbal.select", gimbal.managerCompid, gimbal.deviceId) },
                                label = { Text(gimbal.name) },
                            )
                        }
                    }
                }
                Text(gimbalCellText(state), style = MaterialTheme.typography.bodyMedium)
                if (state.yawLockOffered) {
                    OutlinedButton(onClick = { act("gimbal.yawLock", !state.yawLocked) }, modifier = Modifier.fillMaxWidth()) { Text(state.yawLockLabel) }
                }
                OutlinedButton(onClick = { act("gimbal.center") }, modifier = Modifier.fillMaxWidth()) { Text("Center") }
                OutlinedButton(onClick = { act("gimbal.tilt90") }, modifier = Modifier.fillMaxWidth()) { Text("Tilt 90") }
                OutlinedButton(onClick = { act("gimbal.pointHome") }, modifier = Modifier.fillMaxWidth()) { Text("Point home") }
                if (state.retractOffered) {
                    OutlinedButton(onClick = { act("gimbal.retract") }, modifier = Modifier.fillMaxWidth()) { Text("Retract") }
                }
                if (state.controlOffered) {
                    Button(onClick = { act("gimbal.control", !state.haveControl) }, modifier = Modifier.fillMaxWidth()) { Text(state.controlLabel) }
                }
                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                TextButton(onClick = { open = false; settingsOpen = true }) { Text("Gimbal settings") }
            }
        }
    }

    if (settingsOpen) {
        AircastSheet(onDismissRequest = { settingsOpen = false }) {
            GimbalSettings()
        }
    }
}

private const val GIMBAL_CONTROLLER_SETTINGS = "gimbalControllerSettings"
private const val GIMBAL_SETTINGS_PAGE = "Gimbal Controller"
private const val JOYSTICK_BUTTONS_SPEED = "joystickButtonsSpeed"

internal fun joystickButtonsAvailable(view: JSONObject?): Boolean =
    view != null && !view.isNull("active") && view.optBoolean("vehicle") && view.optBoolean("enabled")

internal fun gimbalSettingsBlocks(sections: List<SettingsSectionRows>, joystickButtons: Boolean): List<SettingsBlock> =
    sections.filter { it.group == GIMBAL_CONTROLLER_SETTINGS }.flatMap { it.blocks }.map { block ->
        block.copy(facts = block.facts.map { fact ->
            if (fact.name != JOYSTICK_BUTTONS_SPEED || joystickButtons) fact
            else fact.copy(enabled = false, disabledReason = "No joystick is enabled for this vehicle.")
        })
    }

@Composable
private fun GimbalSettings() {
    val joystick by qgcPath(JOYSTICK_VIEW)
    var reloads by remember { mutableIntStateOf(0) }
    var sections by remember { mutableStateOf(emptyList<SettingsSectionRows>()) }
    LaunchedEffect(reloads) {
        sections = withContext(Dispatchers.Default) { settingsSections(Qgc.get(settingsPagePath(GIMBAL_SETTINGS_PAGE))) }
    }
    Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
        gimbalSettingsBlocks(sections, joystickButtonsAvailable(joystick)).forEach { block ->
            if (block.title.isNotBlank()) {
                Text(sentenceCase(block.title), style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
            }
            block.facts.forEach { FactRow(it, onWrite = { reloads++ }) }
        }
    }
}

internal const val GIMBAL_DRAG_REPEAT_MS = 100L

internal fun screenFraction(x: Float, y: Float, width: Int, height: Int): Pair<Float, Float> =
    ((x / width) * 2f - 1f) to -((y / height) * 2f - 1f)

internal data class OnScreenGimbal(val enabled: Boolean, val clickAndDrag: Boolean)

internal fun onScreenGimbal(view: JSONObject?): OnScreenGimbal? =
    view?.takeIf { it.optBoolean("shown") }?.optJSONObject("onScreen")?.let {
        OnScreenGimbal(it.optBoolean("enabled"), it.optBoolean("clickAndDrag"))
    }

internal fun aimFraction(delta: Float, width: Int): Float = delta / (maxOf(width, 1) / 2f)

@Composable
internal fun GimbalScreenControl(modifier: Modifier = Modifier) {
    val view by qgcPath(GIMBAL_INDICATOR_PATH)
    val control = remember(view) { onScreenGimbal(view) } ?: return
    val scope = rememberCoroutineScope()
    val send = { pan: Float, tilt: Float, point: Boolean ->
        scope.launch(Dispatchers.Default) { gimbalRefusal(Qgc.call("gimbal.onScreen", pan.toDouble(), tilt.toDouble(), point)) }
        Unit
    }
    if (!control.enabled) {
        Box(
            modifier.pointerInput(Unit) {
                detectDragGestures { change, drag ->
                    change.consume()
                    send(aimFraction(drag.x, size.width), -aimFraction(drag.y, size.width), false)
                }
            },
        )
        return
    }
    Box(
        modifier.pointerInput(control) {
            when (control.clickAndDrag) {
                false -> detectTapGestures { at ->
                    val (pan, tilt) = screenFraction(at.x, at.y, size.width, size.height)
                    send(pan, tilt, true)
                }
                true -> awaitEachGesture {
                    val down = awaitFirstDown()
                    val start = screenFraction(down.position.x, down.position.y, size.width, size.height)
                    var latest = down.position
                    val repeating = scope.launch {
                        while (isActive) {
                            delay(GIMBAL_DRAG_REPEAT_MS)
                            val now = screenFraction(latest.x, latest.y, size.width, size.height)
                            send(now.first - start.first, now.second - start.second, false)
                        }
                    }
                    do {
                        val event = awaitPointerEvent()
                        event.changes.firstOrNull()?.let { latest = it.position }
                    } while (event.changes.any { it.pressed })
                    repeating.cancel()
                }
            }
        },
    )
}
