package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import kotlinx.coroutines.delay
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.FactSlider
import one.aircast.android.bridge.Qgc
import kotlin.math.pow

internal const val VALUE_OFF = "Off"
private const val SLIDER_SPAN_LIMIT = 100_000.0
private val DISABLED_AT_ZERO = Regex("""(?i)(disabled if 0|0 (disables|to disable|hides|turns .* off)|set to 0 to disable|zero disables)""")

internal fun opensAsValue(fact: Fact): Boolean =
    showsAsField(fact) && !fact.isString && !fact.isBitmask && !fact.isEnum

private fun numberOf(fact: Fact): Double? = factNumber(fact)?.toDouble()

internal fun disablesAtZero(fact: Fact): Boolean =
    DISABLED_AT_ZERO.containsMatchIn(fact.description + " " + fact.longDescription)

internal fun valueText(fact: Fact): String =
    if (disablesAtZero(fact) && numberOf(fact) == 0.0) VALUE_OFF
    else listOf(fact.valueString.trim(), fact.units).filter { it.isNotBlank() }.joinToString(" ")

internal fun valueDecimals(fact: Fact): Int = fact.valueString.trim().substringAfter('.', "").takeWhile(Char::isDigit).length

internal fun valueStep(fact: Fact): Double = if (fact.name in USEFUL_BANDS) metresScale(fact) else 10.0.pow(-valueDecimals(fact))

private fun boundOf(text: String, isDefaultForType: Boolean): Double? = text.toDoubleOrNull()?.takeIf { !isDefaultForType && it.isFinite() }

internal fun valueBounds(fact: Fact): Pair<Double?, Double?> =
    boundOf(fact.minString, fact.minIsDefaultForType) to boundOf(fact.maxString, fact.maxIsDefaultForType)

internal fun derivedSlider(fact: Fact): FactSlider? {
    val (min, max) = valueBounds(fact)
    return if (min != null && max != null && max > min && max - min <= SLIDER_SPAN_LIMIT) FactSlider(min.toFloat(), max.toFloat(), valueDecimals(fact), "") else null
}

private val ALTITUDE_BAND = 20.0..500.0
private val DISTANCE_BAND = 50.0..5000.0
private const val ALTITUDE_ON = 120.0
private const val DISTANCE_ON = 500.0

private val USEFUL_BANDS = mapOf(
    "RTL_RETURN_ALT" to ALTITUDE_BAND,
    "RTL_ALT" to ALTITUDE_BAND,
    "GF_MAX_VER_DIST" to ALTITUDE_BAND,
    "FENCE_ALT_MAX" to ALTITUDE_BAND,
    "GF_MAX_HOR_DIST" to DISTANCE_BAND,
    "FENCE_RADIUS" to DISTANCE_BAND,
)

private val TURN_ON_VALUES = mapOf(
    "GF_MAX_VER_DIST" to ALTITUDE_ON,
    "FENCE_ALT_MAX" to ALTITUDE_ON,
    "GF_MAX_HOR_DIST" to DISTANCE_ON,
    "FENCE_RADIUS" to DISTANCE_ON,
)

private fun metresScale(fact: Fact): Double = if (fact.units.trim() == "cm") 100.0 else 1.0

private fun clampToBounds(fact: Fact, value: Double): Double {
    val (min, max) = valueBounds(fact)
    return value.coerceIn(min ?: Double.NEGATIVE_INFINITY, max ?: Double.POSITIVE_INFINITY)
}

internal fun sliderFor(fact: Fact): FactSlider? =
    USEFUL_BANDS[fact.name]?.let { band ->
        val from = clampToBounds(fact, band.start * metresScale(fact))
        val to = clampToBounds(fact, band.endInclusive * metresScale(fact))
        FactSlider(from.toFloat(), to.toFloat(), valueDecimals(fact), "").takeIf { to > from }
    } ?: fact.slider ?: derivedSlider(fact)

internal fun turnOnValue(fact: Fact): Double {
    val default = fact.defaultValueString.toDoubleOrNull()?.takeIf { it > 0 }
    val named = TURN_ON_VALUES[fact.name]?.times(metresScale(fact))
    val smallest = valueBounds(fact).first?.takeIf { it > 0 } ?: valueStep(fact)
    return clampToBounds(fact, default ?: named ?: smallest)
}

internal fun roundedValue(fact: Fact, value: Double): Double = kotlin.math.round(value / valueStep(fact)) * valueStep(fact)

private const val FAST_AFTER_REPEATS = 10
private const val FAST_STEP_FACTOR = 10

internal fun nudged(fact: Fact, from: Double, direction: Int, repeats: Int): Double =
    if (disablesAtZero(fact) && from == 0.0 && direction > 0) turnOnValue(fact)
    else clampToBounds(fact, roundedValue(fact, from + direction * valueStep(fact) * (if (repeats >= FAST_AFTER_REPEATS) FAST_STEP_FACTOR else 1)))

internal fun steppedValue(fact: Fact, direction: Int): Double? = numberOf(fact)?.let { nudged(fact, it, direction, 0) }

internal fun valueTextOf(fact: Fact, value: Double?): String = when {
    value == null -> valueText(fact)
    disablesAtZero(fact) && value == 0.0 -> VALUE_OFF
    else -> sliderValue(value.toFloat(), valueDecimals(fact), fact.units.trim())
}

private const val HOLD_DELAY_MS = 400L
private const val REPEAT_MS = 90L

@Composable
private fun StepButton(label: String, description: String, enabled: Boolean, onStep: (Int) -> Unit, onRelease: () -> Unit) {
    val scope = rememberCoroutineScope()
    val step by rememberUpdatedState(onStep)
    val release by rememberUpdatedState(onRelease)
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.secondaryContainer,
        modifier = Modifier
            .size(48.dp)
            .semantics {
                contentDescription = description
                role = Role.Button
            }
            .pointerInput(enabled) {
                if (enabled) detectTapGestures(onPress = {
                    step(0)
                    val holding = scope.launch {
                        delay(HOLD_DELAY_MS)
                        generateSequence(1) { it + 1 }.forEach { repeats ->
                            step(repeats)
                            delay(REPEAT_MS)
                        }
                    }
                    tryAwaitRelease()
                    holding.cancel()
                    release()
                })
            },
    ) {
        Box(contentAlignment = Alignment.Center) { Text(label, style = MaterialTheme.typography.titleLarge) }
    }
}

@Composable
internal fun ValueControls(fact: Fact, onWrite: () -> Unit, typedEntry: @Composable () -> Unit) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }
    var pending by remember(fact.path, fact.valueString) { mutableStateOf<Double?>(null) }
    var typing by remember(fact.path) { mutableStateOf(false) }
    val write: (Double) -> Unit = { value ->
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, roundedValue(fact, value).let { if (valueDecimals(fact) == 0) it.toLong() else it }) }
            if (refusal == null) onWrite() else pending = null
        }
    }
    val shown = pending ?: numberOf(fact)
    val commit: () -> Unit = { pending?.takeIf { it != numberOf(fact) }?.let(write) }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
        StepButton("\u2212", "Decrease", fact.acceptsWrite, { repeats -> (pending ?: numberOf(fact))?.let { pending = nudged(fact, it, -1, repeats) } }, commit)
        Text(
            valueTextOf(fact, shown),
            style = MaterialTheme.typography.headlineSmall,
            modifier = Modifier.clickable(onClickLabel = "Type a value") { typing = !typing }.padding(horizontal = 8.dp, vertical = 4.dp),
        )
        StepButton("+", "Increase", fact.acceptsWrite, { repeats -> (pending ?: numberOf(fact))?.let { pending = nudged(fact, it, 1, repeats) } }, commit)
    }
    sliderFor(fact)?.takeUnless { disablesAtZero(fact) && shown == 0.0 }?.let { slider -> FieldSlider(shown?.toFloat(), slider, fact.acceptsWrite) { write(it) } }
    if (disablesAtZero(fact)) {
        if (shown == 0.0) TextButton(onClick = { write(turnOnValue(fact)) }) { Text("Turn on") }
        else TextButton(onClick = { write(0.0) }) { Text("Turn off") }
    }
    if (typing) typedEntry()
    refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
}
