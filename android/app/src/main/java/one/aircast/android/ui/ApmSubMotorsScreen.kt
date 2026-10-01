package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
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

internal const val APM_SUB_MOTORS_SCREEN = "apmSubMotors"
internal const val APM_SUB_MOTORS_VIEW = "view.apmSubMotors"
private const val SUB_MOTORS_POLL_MS = 500L
private const val SUB_MOTOR_TEST_MS = 50L
private const val SUB_COOL_DOWN_MS = 11_000L
internal const val SUB_NEUTRAL = 50f

internal data class SubMotor(val motor: Int, val reversed: Boolean)

internal data class SubMotors(
    val armed: Boolean,
    val detecting: Boolean,
    val canRunManualTest: Boolean,
    val motors: List<SubMotor>,
    val warning: String,
    val offersAutoDetect: Boolean,
    val autoDetectHelp: String,
    val detectionMessages: String,
)

internal fun subMotors(view: JSONObject?): SubMotors? = view?.takeIf { it.optBoolean("available") }?.let {
    val motors = it.optJSONArray("motors")
    SubMotors(
        armed = it.optBoolean("armed"),
        detecting = it.optBoolean("detecting"),
        canRunManualTest = it.optBoolean("canRunManualTest"),
        motors = (0 until (motors?.length() ?: 0)).mapNotNull { at -> motors!!.optJSONObject(at)?.let { m -> SubMotor(m.optInt("motor"), m.optBoolean("reversed")) } },
        warning = it.optText("warning"),
        offersAutoDetect = it.optBoolean("offersAutoDetect"),
        autoDetectHelp = it.optText("autoDetectHelp"),
        detectionMessages = it.optText("detectionMessages"),
    )
}

@Composable
fun ApmSubMotorsScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<SubMotors?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    var shouldRunManualTest by remember { mutableStateOf(false) }
    var coolingDown by remember { mutableStateOf(false) }
    var lastIndex by remember { mutableIntStateOf(0) }
    val sliders = remember { mutableStateMapOf<Int, Float>() }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) { withContext(Dispatchers.Default) { Qgc.invoke("apmSubMotors.arm", false) } }
    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { subMotors(Qgc.get(APM_SUB_MOTORS_VIEW)) }
        delay(SUB_MOTORS_POLL_MS)
        revision++
    }
    val armed = read?.armed == true
    LaunchedEffect(armed) {
        sliders.clear()
        shouldRunManualTest = armed
        if (!armed && read != null) {
            coolingDown = true
            delay(SUB_COOL_DOWN_MS)
            coolingDown = false
        }
    }
    val testing = read?.canRunManualTest == true && shouldRunManualTest
    LaunchedEffect(testing) {
        while (testing) {
            val value = sliders[lastIndex] ?: SUB_NEUTRAL
            withContext(Dispatchers.Default) { Qgc.invoke("apmSubMotors.test", lastIndex, value.toDouble()) }
            delay(SUB_MOTOR_TEST_MS)
        }
    }
    fun act(path: String, vararg args: Any) {
        scope.launch { refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) } }
    }
    val state = read ?: run {
        Text("This page is for an ArduSub vehicle.", modifier.padding(16.dp))
        return
    }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        state.motors.forEachIndexed { index, motor ->
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("${motor.motor}", modifier = Modifier.width(24.dp))
                Slider(
                    value = sliders[index] ?: SUB_NEUTRAL,
                    onValueChange = {
                        lastIndex = index
                        sliders[index] = it
                    },
                    onValueChangeFinished = { sliders[index] = SUB_NEUTRAL },
                    valueRange = 0f..100f,
                    enabled = state.canRunManualTest,
                    modifier = Modifier.weight(1f),
                )
                Checkbox(checked = motor.reversed, onCheckedChange = {
                    sliders[index] = SUB_NEUTRAL
                    act("apmSubMotors.reverse", motor.motor, it)
                })
            }
        }
        Text("Reverse Motor Direction", style = MaterialTheme.typography.bodySmall)
        Text(state.warning, style = MaterialTheme.typography.bodySmall)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Switch(checked = state.armed, enabled = !coolingDown, onCheckedChange = { act("apmSubMotors.arm", it) })
            Text(
                if (coolingDown) "A 10 second coooldown is required before testing again, please stand by..." else "Slide this switch to arm the vehicle and enable the motor test (CAUTION!)",
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (state.offersAutoDetect) {
            Text("Automatic Motor Direction Detection", style = MaterialTheme.typography.titleMedium)
            Text(state.autoDetectHelp, style = MaterialTheme.typography.bodySmall)
            Button(enabled = !state.detecting, onClick = { act("apmSubMotors.autoDetect") }) { Text("Auto-Detect Directions") }
            if (state.detectionMessages.isNotBlank()) Text(state.detectionMessages, style = MaterialTheme.typography.bodySmall)
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    }
}
