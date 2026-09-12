package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
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
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDouble

private const val TIMEOUT_SECONDS = 3
private const val UNKNOWN_MOTOR_COUNT = 8

internal fun motorCount(reported: Double): Int =
    if (reported.isNaN() || reported < 1) UNKNOWN_MOTOR_COUNT else reported.toInt()

internal fun motorCountNotice(reported: Double): String? =
    if (reported.isNaN() || reported < 1) {
        "The vehicle did not say how many motors it has, so eight are offered."
    } else {
        null
    }

internal fun spin(motor: Int, percent: Int) {
    val seconds = if (percent == 0) 0 else TIMEOUT_SECONDS
    Qgc.invoke("vehicle.motorTest", motor, percent, seconds, true)
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun MotorsScreen(modifier: Modifier = Modifier) {
    val reported by qgcDouble("vehicle.motorCount", Double.NaN)
    val motors = motorCount(reported)
    var propsOff by remember { mutableStateOf(false) }
    var throttle by remember { mutableFloatStateOf(20f) }

    Column(modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Motor test", style = MaterialTheme.typography.titleMedium)
        Text(
            "Each motor spins for $TIMEOUT_SECONDS seconds at the throttle below. " +
                "Take the propellers off first — a spinning propeller will cut you.",
            style = MaterialTheme.typography.bodySmall,
        )
        motorCountNotice(reported)?.let {
            Text(it, style = MaterialTheme.typography.bodySmall)
        }

        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Switch(checked = propsOff, onCheckedChange = { propsOff = it })
            Text(
                if (propsOff) "Motors are live" else "Propellers are off — turn on to enable",
                style = MaterialTheme.typography.bodyMedium,
            )
        }

        Text("Throttle ${throttle.toInt()}%", style = MaterialTheme.typography.bodyMedium)
        Slider(
            value = throttle,
            onValueChange = { throttle = it },
            valueRange = 0f..100f,
            enabled = propsOff,
            modifier = Modifier.fillMaxWidth(),
        )

        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            (1..motors).forEach { motor ->
                Button(onClick = { spin(motor, throttle.toInt()) }, enabled = propsOff) {
                    Text("$motor")
                }
            }
        }

        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = { (1..motors).forEach { spin(it, throttle.toInt()) } },
                enabled = propsOff,
            ) { Text("All") }
            OutlinedButton(
                onClick = { (1..motors).forEach { spin(it, 0) } },
                enabled = propsOff,
            ) { Text("Stop") }
        }
    }
}
