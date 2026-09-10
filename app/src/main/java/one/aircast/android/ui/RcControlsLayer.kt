package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcString

private const val RC_CONTROLS_FACT = "settings.flyViewSettings.rcControls"

private fun sendOverride(channel: Int, pwm: Int) {
    offMainDetached { Qgc.invoke("vehicle.setRcChannelOverride", channel, pwm) }
}

private fun releaseOverrides() {
    offMainDetached { Qgc.invoke("vehicle.clearRcChannelOverrides") }
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
private fun RcSlider(control: RcControl) {
    var pwm by remember(control.channel) { mutableIntStateOf(PWM_CENTER) }
    var lastSent by remember(control.channel) { mutableLongStateOf(0L) }

    fun send(value: Int, finished: Boolean) {
        val now = SystemClock.uptimeMillis()
        if (rcSendDue(now, lastSent, finished)) {
            lastSent = now
            sendOverride(control.channel, value)
        }
    }

    Row(verticalAlignment = Alignment.CenterVertically) {
        ControlLabel(control.label)
        Slider(
            value = pwm.toFloat(),
            onValueChange = { raw ->
                val next = raw.toInt()
                if (next != pwm) {
                    pwm = next
                    send(next, finished = false)
                }
            },
            onValueChangeFinished = { send(pwm, finished = true) },
            valueRange = PWM_MIN.toFloat()..PWM_MAX.toFloat(),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun RcButton(control: RcControl) {
    var on by remember(control.channel) { mutableStateOf(false) }

    FilterChip(
        selected = on,
        onClick = {
            on = !on
            sendOverride(control.channel, if (on) PWM_MAX else PWM_MIN)
        },
        label = { Text(control.label) },
    )
}

@Composable
private fun RcSwitch3(control: RcControl) {
    var position by remember(control.channel) { mutableIntStateOf(1) }

    Row(verticalAlignment = Alignment.CenterVertically) {
        ControlLabel(control.label)
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            switch3Pwms().forEachIndexed { index, value ->
                FilterChip(
                    selected = position == index,
                    onClick = {
                        position = index
                        sendOverride(control.channel, value)
                    },
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
                sendOverride(control.channel, PWM_MAX)
            }
            everPressed -> sendOverride(control.channel, PWM_MIN)
        }
    }

    Button(onClick = {}, interactionSource = interactions) { Text(control.label) }
}

@Composable
fun RcControlsLayer(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val configured by qgcString(RC_CONTROLS_FACT)
    val controls = remember(configured) { parseRcControls(configured) }
    val overriding by qgcBool("vehicle.rcChannelOverrideActive")

    DisposableEffect(Unit) { onDispose { releaseOverrides() } }

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
            controls.forEach { control ->
                when (control.type) {
                    RcControlType.Slider -> RcSlider(control)
                    RcControlType.Button -> RcButton(control)
                    RcControlType.Switch3 -> RcSwitch3(control)
                    RcControlType.Momentary -> RcMomentary(control)
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
