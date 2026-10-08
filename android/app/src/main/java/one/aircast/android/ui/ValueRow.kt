package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.FilledTonalIconButton
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

internal fun valueStep(fact: Fact): Double = 10.0.pow(-valueDecimals(fact))

private fun boundOf(text: String, isDefaultForType: Boolean): Double? = text.toDoubleOrNull()?.takeIf { !isDefaultForType && it.isFinite() }

internal fun valueBounds(fact: Fact): Pair<Double?, Double?> =
    boundOf(fact.minString, fact.minIsDefaultForType) to boundOf(fact.maxString, fact.maxIsDefaultForType)

internal fun derivedSlider(fact: Fact): FactSlider? {
    val (min, max) = valueBounds(fact)
    return if (min != null && max != null && max > min && max - min <= SLIDER_SPAN_LIMIT) FactSlider(min.toFloat(), max.toFloat(), valueDecimals(fact), "") else null
}

internal fun roundedValue(fact: Fact, value: Double): Double = kotlin.math.round(value / valueStep(fact)) * valueStep(fact)

internal fun steppedValue(fact: Fact, direction: Int): Double? {
    val current = numberOf(fact) ?: return null
    val (min, max) = valueBounds(fact)
    val next = roundedValue(fact, current + direction * valueStep(fact))
    return next.coerceIn(min ?: Double.NEGATIVE_INFINITY, max ?: Double.POSITIVE_INFINITY)
}

@Composable
internal fun ValueControls(fact: Fact, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }
    val write: (Double) -> Unit = { value ->
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, roundedValue(fact, value).let { if (valueDecimals(fact) == 0) it.toLong() else it }) }
            if (refusal == null) onWrite()
        }
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
        FilledTonalIconButton(onClick = { steppedValue(fact, -1)?.let(write) }) { Text("−", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { contentDescription = "Decrease" }) }
        Text(valueText(fact), style = MaterialTheme.typography.headlineSmall)
        FilledTonalIconButton(onClick = { steppedValue(fact, 1)?.let(write) }) { Text("+", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { contentDescription = "Increase" }) }
    }
    if (fact.slider == null) derivedSlider(fact)?.let { slider -> FieldSlider(numberOf(fact)?.toFloat(), slider, fact.acceptsWrite) { write(it) } }
    if (disablesAtZero(fact) && numberOf(fact) != 0.0) TextButton(onClick = { write(0.0) }) { Text("Turn off") }
    refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
}
