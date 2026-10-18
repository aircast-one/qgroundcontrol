package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainInOrder
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import android.os.SystemClock
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath

internal class RcHolder(private val send: (() -> Unit) -> Unit = ::offMainInOrder) {
    private val sent = java.util.concurrent.ConcurrentHashMap.newKeySet<Int>()

    fun hold(channel: Int, pwm: Int) {
        if (channel <= 0) return
        sent.add(channel)
        send { VehicleCommands.overrideRcChannel(channel, pwm) }
    }

    fun release() {
        val held = sent.toList().sorted()
        sent.removeAll(held.toSet())
        if (held.isNotEmpty()) send { held.forEach { VehicleCommands.releaseRcChannel(it) } }
    }

    fun forget() = sent.clear()

    fun holding(): Set<Int> = sent.toSet()
}

internal val customRcControls = RcHolder()
internal val cameraRcControls = RcHolder()

private fun sendRcOverride(channel: Int, pwm: Int) = customRcControls.hold(channel, pwm)

private fun releaseOverrides() {
    customRcControls.forget()
    cameraRcControls.forget()
    offMainInOrder { VehicleCommands.clearRcOverrides() }
}

@Composable
private fun ControlLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        modifier = Modifier.width(84.dp),
    )
}

@Composable
private fun RcSlider(state: RcControlState) {
    val control = state.control
    var dragging by remember(control.channel) { mutableStateOf<Int?>(null) }
    var lastSent by remember(control.channel) { mutableLongStateOf(0L) }
    val shown = dragging ?: state.pwm ?: PWM_CENTER

    fun send(value: Int, finished: Boolean) {
        val now = SystemClock.uptimeMillis()
        if (rcSendDue(now, lastSent, finished)) {
            lastSent = now
            sendRcOverride(control.channel, value)
        }
    }

    Row(verticalAlignment = Alignment.CenterVertically) {
        ControlLabel(control.label)
        Slider(
            value = shown.toFloat(),
            onValueChange = { raw ->
                val next = raw.toInt()
                if (next != shown) {
                    dragging = next
                    send(next, finished = false)
                }
            },
            onValueChangeFinished = {
                send(shown, finished = true)
                dragging = null
            },
            valueRange = PWM_MIN.toFloat()..PWM_MAX.toFloat(),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun RcButton(state: RcControlState) {
    val control = state.control
    val on = state.on == true

    FilterChip(
        selected = on,
        onClick = { sendRcOverride(control.channel, if (on) PWM_MIN else PWM_MAX) },
        label = { Text(control.label) },
    )
}

@Composable
private fun RcSwitch3(state: RcControlState) {
    val control = state.control

    Row(verticalAlignment = Alignment.CenterVertically) {
        ControlLabel(control.label)
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            switch3Pwms().forEachIndexed { index, value ->
                FilterChip(
                    selected = state.position == index,
                    onClick = { sendRcOverride(control.channel, value) },
                    label = { Text(listOf("Low", "Mid", "High")[index]) },
                )
            }
        }
    }
}

@Composable
private fun RcMomentary(control: RcControl) {
    val interactions = remember(control.channel) { MutableInteractionSource() }
    val pressed by interactions.collectIsPressedAsState()
    var everPressed by remember(control.channel) { mutableStateOf(false) }

    LaunchedEffect(pressed) {
        when {
            pressed -> {
                everPressed = true
                sendRcOverride(control.channel, PWM_MAX)
            }
            everPressed -> sendRcOverride(control.channel, PWM_MIN)
        }
    }

    Button(onClick = {}, interactionSource = interactions) { Text(control.label) }
}

@Composable
fun RcControlsLayer(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
    val view by qgcPath(RC_CONTROLS_VIEW)
    val controls = remember(view) { parseRcControlStates(view) }
    val stateJson by qgcPath(FLY_STATE)
    val overriding = remember(stateJson) { flyState(stateJson)?.rcOverride == true }

    DisposableEffect(Unit) { onDispose { customRcControls.release() } }

    if (controls.isEmpty() || !hasVehicle) {
        return
    }

    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
    ) {
        Column(
            Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            controls.forEach { state ->
                when (state.control.type) {
                    RcControlType.Slider -> RcSlider(state)
                    RcControlType.Button -> RcButton(state)
                    RcControlType.Switch3 -> RcSwitch3(state)
                    RcControlType.Momentary -> RcMomentary(state.control)
                }
            }

            if (overriding) {
                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        text = "These channels are held by this tablet.",
                        style = MaterialTheme.typography.labelMedium,
                        modifier = Modifier.weight(1f),
                    )
                    Button(onClick = { releaseOverrides() }) { Text("Give back") }
                }
            }
        }
    }
}
