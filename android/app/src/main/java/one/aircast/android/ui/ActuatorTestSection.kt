package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val ACTUATOR_TEST_ACTIVE = "actuatorTest.setActive"
internal const val ACTUATOR_TEST_SET = "actuatorTest.setChannelTo"
internal const val ACTUATOR_TEST_STOP = "actuatorTest.stopControl"
private const val SEND_MS = 50L
private const val SNAP_FRACTION = 0.15

internal data class TestChannel(val label: String, val function: Int, val min: Double, val max: Double, val default: Double?, val isMotor: Boolean) {
    val snap: Boolean get() = default == null
    val snapRange: Double get() = (max - min) * SNAP_FRACTION
    val from: Double get() = if (snap) min - snapRange else min
    val rest: Double get() = default ?: from
}

internal data class ActuatorTesting(val actuators: List<TestChannel>, val allMotors: TestChannel?, val hadFailure: Boolean)
internal data class ActuatorActionChoice(val label: String, val function: Int)
internal data class ActuatorActionGroup(val label: String, val type: Int, val actions: List<ActuatorActionChoice>)

internal const val ACTUATOR_ACTION_TRIGGER = "actuatorAction.trigger"

internal fun actuatorActions(view: JSONObject?): List<ActuatorActionGroup> {
    val groups = view?.optJSONArray("actions") ?: return emptyList()
    return (0 until groups.length()).mapNotNull { index ->
        groups.optJSONObject(index)?.let { group ->
            val actions = group.optJSONArray("actions")
            ActuatorActionGroup(
                group.optText("label"),
                group.optInt("type"),
                (0 until (actions?.length() ?: 0)).mapNotNull { at -> actions?.optJSONObject(at)?.let { ActuatorActionChoice(it.optText("label"), it.optInt("function")) } },
            )
        }
    }
}

private fun testChannel(json: JSONObject?): TestChannel? = json?.let {
    TestChannel(it.optText("label"), it.optInt("function"), it.optDouble("min"), it.optDouble("max"), if (it.isNull("default")) null else it.optDouble("default"), it.optBoolean("isMotor"))
}

internal fun actuatorTesting(view: JSONObject?): ActuatorTesting? =
    view?.optJSONObject("testing")?.let { testing ->
        val list = testing.optJSONArray("actuators")
        ActuatorTesting(
            actuators = (0 until (list?.length() ?: 0)).mapNotNull { testChannel(list?.optJSONObject(it)) },
            allMotors = testChannel(testing.optJSONObject("allMotors")),
            hadFailure = testing.optBoolean("hadFailure"),
        )
    }

internal fun snapped(channel: TestChannel, value: Double): Double =
    when {
        !channel.snap || value >= channel.min -> value
        value < channel.min - channel.snapRange / 2 -> channel.min - channel.snapRange
        else -> channel.min
    }

internal fun sentValue(channel: TestChannel, value: Double): Double? =
    if (value < channel.min - channel.snapRange / 2) channel.default else value

@Composable
internal fun ActuatorTestSection(
    testing: ActuatorTesting,
    actions: List<ActuatorActionGroup> = emptyList(),
    enabled: Boolean,
    assigning: Boolean = false,
    onEnabled: (Boolean) -> Unit,
) {
    var values by remember(testing.actuators) { mutableStateOf(testing.actuators.associate { it.function to it.rest }) }
    var moved by remember(testing.actuators) { mutableStateOf(emptySet<Int>()) }
    var allMotors by remember(testing.allMotors) { mutableStateOf(testing.allMotors?.rest ?: 0.0) }

    DisposableEffect(Unit) { onDispose { offMainDetached { Qgc.invoke(ACTUATOR_TEST_ACTIVE, false) } } }

    fun setEnabled(on: Boolean) {
        onEnabled(on)
        if (!on) {
            values = testing.actuators.associate { it.function to it.rest }
            moved = emptySet()
            allMotors = testing.allMotors?.rest ?: 0.0
        }
        offMainDetached { Qgc.invoke(ACTUATOR_TEST_ACTIVE, on) }
    }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Actuator Testing", style = MaterialTheme.typography.titleMedium)
        if (actions.isNotEmpty()) {
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                actions.forEach { group -> ActionGroupButton(group, enabled = !enabled && !assigning) }
            }
        }
        if (testing.actuators.isEmpty()) {
            Text("Configure some outputs in order to test them.")
            return@Column
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Switch(checked = enabled && !testing.hadFailure, onCheckedChange = ::setEnabled, enabled = !testing.hadFailure && !assigning)
            Text(if (enabled) "Careful: Actuator sliders are enabled" else "Propellers are removed - Enable sliders")
        }
        testing.allMotors?.let { motors ->
            TestSlider(motors, allMotors, enabled) { value ->
                allMotors = value
                val motorFunctions = testing.actuators.filter { it.isMotor }.map { it.function }
                values = values + motorFunctions.associateWith { value }
                moved = moved + motorFunctions
            }
        }
        testing.actuators.forEach { channel ->
            TestSlider(channel, values[channel.function] ?: channel.rest, enabled) { value ->
                values = values + (channel.function to value)
                moved = moved + channel.function
            }
            if (enabled && channel.function in moved) {
                ChannelSender(channel, values[channel.function] ?: channel.rest) {
                    values = values + (channel.function to channel.rest)
                    moved = moved - channel.function
                }
            }
        }
    }
}

@Composable
private fun ActionGroupButton(group: ActuatorActionGroup, enabled: Boolean) {
    var open by remember { mutableStateOf(false) }
    androidx.compose.foundation.layout.Box {
        androidx.compose.material3.OutlinedButton(onClick = { open = true }, enabled = enabled) { Text(group.label) }
        androidx.compose.material3.DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            group.actions.forEach { action ->
                androidx.compose.material3.DropdownMenuItem(text = { Text(action.label) }, onClick = {
                    open = false
                    offMainDetached { Qgc.invoke(ACTUATOR_ACTION_TRIGGER, group.type, action.function) }
                })
            }
        }
    }
}

@Composable
private fun ChannelSender(channel: TestChannel, value: Double, onStopped: () -> Unit) {
    val latest by rememberUpdatedState(value)
    val stopped by rememberUpdatedState(onStopped)
    LaunchedEffect(channel.function) {
        while (isActive) {
            val send = sentValue(channel, latest)
            withContext(Dispatchers.IO) {
                if (send == null) Qgc.invoke(ACTUATOR_TEST_STOP, channel.function) else Qgc.invoke(ACTUATOR_TEST_SET, channel.function, send)
            }
            if (send == null) {
                stopped()
                break
            }
            delay(SEND_MS)
        }
    }
}

@Composable
private fun TestSlider(channel: TestChannel, value: Double, enabled: Boolean, onChange: (Double) -> Unit) {
    Column(Modifier.fillMaxWidth()) {
        Text(channel.label, style = MaterialTheme.typography.labelMedium)
        Slider(
            value = value.toFloat(),
            onValueChange = { onChange(snapped(channel, it.toDouble())) },
            valueRange = channel.from.toFloat()..channel.max.toFloat(),
            enabled = enabled,
        )
    }
}
