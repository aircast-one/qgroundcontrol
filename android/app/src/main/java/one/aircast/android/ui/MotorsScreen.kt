package one.aircast.android.ui

import one.aircast.mapspike.aircast
import one.aircast.android.R
import androidx.compose.ui.res.painterResource
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Surface
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath

private const val FRAME_VIEW = "view.frame"

private const val TIMEOUT_SECONDS = 3
private const val UNKNOWN_MOTOR_COUNT = 8

internal fun reportedMotors(view: JSONObject?): Int? =
    view?.takeIf { !it.isNull("motorCount") }?.optInt("motorCount")?.takeIf { it >= 1 }

internal fun motorCount(reported: Int?): Int = reported ?: UNKNOWN_MOTOR_COUNT

internal fun motorCountNotice(reported: Int?): String? =
    if (reported == null) {
        "No motor layout is published for this airframe, so eight are offered. " +
            "Test only the motors it actually has."
    } else {
        null
    }

internal data class MotorGate(
    val connected: Boolean,
    val armed: Boolean,
    val contactKnownLost: Boolean,
)

internal fun motorGate(view: JSONObject?): MotorGate = MotorGate(
    connected = view?.optBoolean("connected") == true,
    armed = view?.optBoolean("armed") == true,
    contactKnownLost = view?.takeIf { !it.isNull("contactLost") }?.optBoolean("contactLost") == true,
)

internal fun canTest(gate: MotorGate, propsOff: Boolean): Boolean =
    gate.connected && propsOff && !gate.armed && !gate.contactKnownLost

internal fun canStop(gate: MotorGate): Boolean = gate.connected

internal fun motorRefusal(gate: MotorGate): String? = when {
    !gate.connected -> "No vehicle is connected, so nothing will answer a motor test."
    gate.armed -> "The vehicle is armed. Disarm it before testing a motor."
    gate.contactKnownLost -> "The vehicle has stopped answering. Check the link before testing a motor."
    else -> null
}

internal fun spin(motor: Int, percent: Int) {
    val seconds = if (percent == 0) 0 else TIMEOUT_SECONDS
    Qgc.invoke("vehicle.motorTest", motor, percent, seconds, true)
}

internal fun motorLabel(motor: Int, letters: Boolean): String =
    if (letters) ('A' + (motor - 1)).toString() else motor.toString()

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun MotorsScreen(modifier: Modifier = Modifier) {
    val frameJson by qgcPath(FRAME_VIEW)
    val reported = remember(frameJson) { reportedMotors(frameJson) }
    val gate = remember(frameJson) { motorGate(frameJson) }
    val motors = motorCount(reported)
    var propsOff by remember { mutableStateOf(false) }
    var throttle by remember { mutableFloatStateOf(0f) }
    val setupJson by qgcPath(SETUP)
    val letters = remember(setupJson) { setupReadiness(setupJson)?.firmware == "apm" }

    Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Surface(color = MaterialTheme.aircast.warningContainer, contentColor = MaterialTheme.aircast.warning, shape = CircleShape) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Icon(painterResource(R.drawable.ic_warning), null)
                Text("Take the propellers off first", style = MaterialTheme.typography.labelLarge)
            }
        }
        Text(
            "Each motor spins for $TIMEOUT_SECONDS seconds at the throttle below.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        motorCountNotice(reported)?.let {
            Text(it, style = MaterialTheme.typography.bodySmall)
        }
        motorRefusal(gate)?.let {
            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
        }

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Switch(checked = propsOff, onCheckedChange = {
                propsOff = it
                if (!it) throttle = 0f
            })
            Text(
                if (propsOff) "Careful: motors are live" else "Propellers are off, so enable the motor test",
                style = MaterialTheme.typography.bodyMedium,
            )
        }

        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("Test throttle", style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
            Text("${throttle.toInt()}%", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
        }
        Slider(
            value = throttle,
            onValueChange = { throttle = it },
            valueRange = 0f..100f,
            enabled = canTest(gate, propsOff),
            modifier = Modifier.fillMaxWidth(),
        )

        (1..motors).chunked(2).forEach { pair ->
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                pair.forEach { motor ->
                    Surface(
                        onClick = { spin(motor, throttle.toInt()) },
                        enabled = canTest(gate, propsOff),
                        shape = MaterialTheme.shapes.large,
                        color = MaterialTheme.colorScheme.surfaceContainer,
                        modifier = Modifier.weight(1f),
                    ) {
                        Column(Modifier.padding(vertical = 20.dp).fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("Motor ${motorLabel(motor, letters)}", style = MaterialTheme.typography.bodyLarge)
                            Text(
                                if (canTest(gate, propsOff)) "Tap to spin" else "Locked",
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
                if (pair.size == 1) Spacer(Modifier.weight(1f))
            }
        }

        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilledTonalButton(
                onClick = { (1..motors).forEach { spin(it, throttle.toInt()) } },
                enabled = canTest(gate, propsOff),
            ) { Text("Spin all for $TIMEOUT_SECONDS s") }
            OutlinedButton(
                onClick = { (1..motors).forEach { spin(it, 0) } },
                enabled = canStop(gate),
            ) { Text("Stop") }
        }
    }
}
