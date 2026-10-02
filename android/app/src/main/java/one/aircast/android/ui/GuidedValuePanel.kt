package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import java.math.BigDecimal
import java.math.RoundingMode
import java.util.Locale

@Composable
internal fun GuidedValuePanel(
    title: String,
    sentence: String,
    commitLabel: String,
    commitEnabled: Boolean,
    onCommit: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    Surface(
        modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        shape = MaterialTheme.shapes.extraLarge,
    ) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 24.dp, vertical = 20.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        if (sentence.isNotBlank()) {
            Text(sentence, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        content()
        SlideToConfirm(label = slideLabel(commitLabel), enabled = commitEnabled, onConfirm = onCommit)
        TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
    }
    }
}

private val METRIC_UNITS = setOf("m", "m/s", "km/h")

internal fun guidedBounds(minimum: Double?, maximum: Double?): Pair<Double, Double>? =
    minimum?.let { low -> maximum?.let { high -> low to high } }

internal fun guidedDecimals(unit: String): Int = if (unit in METRIC_UNITS) 1 else 0

internal fun guidedRounded(value: Double, unit: String): Double =
    BigDecimal(value).setScale(guidedDecimals(unit), RoundingMode.HALF_UP).toDouble()

internal fun guidedStepped(value: Double, delta: Int, minimum: Double, maximum: Double, unit: String): Double =
    guidedRounded((value + delta).coerceIn(minimum, maximum), unit)

internal fun guidedTyped(text: String, minimum: Double, maximum: Double, unit: String): Double? =
    text.trim().replace(',', '.').toDoubleOrNull()?.takeIf { it.isFinite() }?.let { guidedRounded(it.coerceIn(minimum, maximum), unit) }

internal fun guidedValueText(value: Double, unit: String): String =
    String.format(Locale.US, "%.${guidedDecimals(unit)}f", value)

internal fun guidedQuickPicks(value: Double, minimum: Double, maximum: Double, unit: String): List<Pair<String, Double>> =
    listOf(10, 20).map { step -> "+$step $unit".trim() to guidedStepped(value, step, minimum, maximum, unit) } +
        ("Max ${guidedValueText(maximum, unit).removeSuffix(".0")} $unit".trim() to guidedRounded(maximum, unit))

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun GuidedQuickPicks(value: Double, minimum: Double, maximum: Double, unit: String, onValue: (Double) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        guidedQuickPicks(value, minimum, maximum, unit).forEach { (label, target) ->
            FilterChip(selected = false, enabled = target != value, onClick = { onValue(target) }, label = { Text(label) })
        }
    }
}

@Composable
internal fun GuidedStepper(
    value: Double,
    label: String,
    unit: String,
    minimum: Double,
    maximum: Double,
    onValue: (Double) -> Unit,
) {
    var typing by remember { mutableStateOf<String?>(null) }
    val focusManager = LocalFocusManager.current
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            val typed = typing
            if (typed == null) {
                Text(
                    guidedValueText(value, unit),
                    style = MaterialTheme.typography.displaySmall.copy(fontFeatureSettings = "tnum"),
                    modifier = Modifier.clickable { typing = guidedValueText(value, unit) },
                )
            } else {
                val focus = remember { FocusRequester() }
                LaunchedEffect(Unit) { focus.requestFocus() }
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typing = it },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = {
                        guidedTyped(typed, minimum, maximum, unit)?.let(onValue)
                        focusManager.clearFocus()
                        typing = null
                    }),
                    modifier = Modifier.width(160.dp).focusRequester(focus),
                )
            }
            Text(listOf(label, unit).filter { it.isNotBlank() }.joinToString(" "), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(-1 to "−", 1 to "+").forEach { (delta, sign) ->
                FilledTonalIconButton(
                    onClick = { onValue(guidedStepped(value, delta, minimum, maximum, unit)) },
                    modifier = Modifier.size(40.dp),
                    colors = IconButtonDefaults.filledTonalIconButtonColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHighest),
                ) { Text(sign, style = MaterialTheme.typography.titleLarge) }
            }
        }
    }
}
