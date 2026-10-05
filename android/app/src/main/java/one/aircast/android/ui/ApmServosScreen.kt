package one.aircast.android.ui

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

internal const val APM_SERVOS_VIEW = "view.apmServos"
internal const val APM_SERVOS_SCREEN = "apmServos"
private const val POLL_MS = 200L
private const val REPEAT_DELAY_MS = 350L
private const val REPEAT_MS = 80L

internal data class ApmServo(
    val index: Int,
    val pwm: Int?,
    val position: Float?,
    val function: Fact?,
    val min: Fact?,
    val trim: Fact?,
    val max: Fact?,
    val reversed: Fact?,
)

internal fun apmServos(view: JSONObject?): List<ApmServo> {
    val servos = view?.optJSONArray("servos") ?: return emptyList()
    return (0 until servos.length()).mapNotNull { at ->
        servos.optJSONObject(at)?.let { servo ->
            val fact = { key: String -> servo.optJSONObject(key)?.let(::factFromControl) }
            ApmServo(
                index = servo.optInt("index"),
                pwm = if (servo.isNull("pwm")) null else servo.optInt("pwm"),
                position = if (servo.isNull("position")) null else servo.optDouble("position").toFloat(),
                function = fact("function"),
                min = fact("min"),
                trim = fact("trim"),
                max = fact("max"),
                reversed = fact("reversed"),
            )
        }
    }
}

internal fun stepped(fact: Fact, direction: Int): Double? =
    ((fact.value as? Number)?.toDouble() ?: fact.valueString.toDoubleOrNull())?.plus(direction)

@Composable
fun ApmServosScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var servos by remember { mutableStateOf<List<ApmServo>?>(null) }

    LaunchedEffect(revision) {
        servos = withContext(Dispatchers.Default) { apmServos(Qgc.get(APM_SERVOS_VIEW)) }
        delay(POLL_MS)
        revision++
    }
    val read = servos ?: run {
        Text("Reading servo outputs.", modifier.padding(16.dp))
        return
    }
    if (read.isEmpty()) {
        Text("This vehicle exposes no servo outputs.", modifier.padding(16.dp))
        return
    }
    LazyColumn(modifier.fillMaxSize()) {
        item(key = "intro") {
            Text(
                "Configure ArduPilot servo outputs.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 12.dp),
            )
        }
        items(read, key = { it.index }) { servo -> ServoCard(servo) }
    }
}

@Composable
private fun ServoCard(servo: ApmServo) {
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        SectionHeader("Servo ${servo.index}")
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Position", style = MaterialTheme.typography.bodyMedium)
            LinearProgressIndicator(progress = { servo.position ?: 0f }, modifier = Modifier.weight(1f))
            Text(servo.pwm?.toString() ?: "-", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.width(48.dp))
        }
        servo.function?.let { FactRow(it, title = "Function") }
        servo.min?.let { Stepper("Min", it) }
        servo.trim?.let { Stepper("Trim", it) }
        servo.max?.let { Stepper("Max", it) }
        servo.reversed?.let { FactRow(it, title = "Reversed") }
    }
}

@Composable
private fun Stepper(title: String, fact: Fact) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }
    val latest = rememberUpdatedState(fact)
    val pending = remember(fact.path) { mutableStateOf<Double?>(null) }

    fun write(next: Double) {
        pending.value = next
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, next) }
            if (refusal != null) pending.value = null
        }
    }

    fun step(direction: Int) {
        val next = (pending.value ?: latest.value.let { stepped(it, 0) })?.plus(direction) ?: return
        write(next)
    }

    val shown = pending.value?.toLong()?.toString() ?: fact.valueString
    var typed by remember(shown) { mutableStateOf(shown) }
    fun commit() {
        val number = one.aircast.map.typedNumber(typed)?.takeIf { it == kotlin.math.floor(it) }
        when {
            number == null -> {
                refusal = "Enter a whole number."
                typed = shown
            }
            number != one.aircast.map.typedNumber(shown) -> write(number)
        }
    }

    Column {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(title, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
            RepeatButton("-") { step(-1) }
            androidx.compose.material3.OutlinedTextField(
                value = typed,
                onValueChange = { typed = it },
                singleLine = true,
                textStyle = MaterialTheme.typography.bodyLarge,
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(
                    keyboardType = androidx.compose.ui.text.input.KeyboardType.Number,
                    imeAction = androidx.compose.ui.text.input.ImeAction.Done,
                ),
                keyboardActions = androidx.compose.foundation.text.KeyboardActions(onDone = { commit() }),
                modifier = Modifier.width(96.dp).onFocusChanged { if (!it.isFocused) commit() },
            )
            RepeatButton("+") { step(1) }
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 20.dp)) }
    }
    LaunchedEffect(fact.valueString) { pending.value = null }
}

@Composable
private fun RepeatButton(label: String, onStep: () -> Unit) {
    val step = rememberUpdatedState(onStep)
    Surface(
        shape = MaterialTheme.shapes.small,
        color = MaterialTheme.colorScheme.secondaryContainer,
        modifier = Modifier.size(40.dp).pointerInput(Unit) {
            detectTapGestures(onPress = {
                step.value()
                coroutineScope {
                    val repeating = launch {
                        delay(REPEAT_DELAY_MS)
                        while (isActive) {
                            step.value()
                            delay(REPEAT_MS)
                        }
                    }
                    tryAwaitRelease()
                    repeating.cancel()
                }
            })
        },
    ) {
        Box(contentAlignment = Alignment.Center) { Text(label, style = MaterialTheme.typography.titleMedium) }
    }
}
