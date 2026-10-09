package one.aircast.map

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeoutOrNull
import java.util.Locale

private const val REPEAT_DELAY_MS = 400L
private const val COMMIT_DELAY_MS = 500L
private const val REPEAT_INTERVAL_MS = 90L
private const val DISABLED_ALPHA = 0.38f
private val STEP_BUTTON = 40.dp
private val VALUE_MIN_WIDTH = 72.dp

fun steppedValue(current: Double, step: Double, direction: Int, range: ClosedFloatingPointRange<Double>?): Double {
    val next = current + step * direction
    return range?.let { next.coerceIn(it.start, it.endInclusive) } ?: next
}

fun stepperText(value: Double?, decimals: Int): String =
    value?.takeIf { it.isFinite() }?.let { String.format(Locale.US, "%.${decimals}f", it) } ?: "—"

fun stepperDecimals(value: Double?, step: Double): Int =
    if (listOfNotNull(value, step).all { it == kotlin.math.floor(it) }) 0 else 1

private tailrec suspend fun androidx.compose.ui.input.pointer.AwaitPointerEventScope.repeatWhileHeld(step: () -> Unit) {
    val released = withTimeoutOrNull(REPEAT_INTERVAL_MS) { waitForUpOrCancellation(); true } ?: false
    if (released) return
    step()
    repeatWhileHeld(step)
}

@Composable
private fun StepButton(sign: String, description: String, enabled: Boolean, onStep: () -> Unit) {
    val step by rememberUpdatedState(onStep)
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHighest,
        modifier = Modifier
            .size(STEP_BUTTON)
            .alpha(if (enabled) 1f else DISABLED_ALPHA)
            .semantics {
                role = Role.Button
                contentDescription = description
                onClick { step(); true }
            }
            .then(
                if (enabled) Modifier.pointerInput(Unit) {
                    awaitEachGesture {
                        awaitFirstDown().consume()
                        step()
                        val released = withTimeoutOrNull(REPEAT_DELAY_MS) { waitForUpOrCancellation(); true } ?: false
                        if (!released) repeatWhileHeld { step() }
                    }
                } else Modifier,
            ),
    ) {
        Box(contentAlignment = Alignment.Center) {
            Text(sign, style = MaterialTheme.typography.titleLarge)
        }
    }
}

@Composable
fun SettingStepper(
    label: String,
    value: Double?,
    unit: String,
    step: Double,
    onSet: (Double) -> Unit,
    modifier: Modifier = Modifier,
    note: String? = null,
    range: ClosedFloatingPointRange<Double>? = null,
    slider: Boolean = false,
    enabled: Boolean = true,
    trailing: (@Composable () -> Unit)? = null,
) {
    var shown by remember(value) { mutableStateOf(value) }
    var typing by remember { mutableStateOf(false) }
    var stepped by remember { mutableStateOf<Double?>(null) }
    val commit by rememberUpdatedState(onSet)
    LaunchedEffect(stepped) {
        stepped?.let { wanted ->
            delay(COMMIT_DELAY_MS)
            stepped = null
            commit(wanted)
        }
    }
    DisposableEffect(Unit) {
        onDispose { stepped?.let(commit) }
    }
    val decimals = stepperDecimals(shown, step)
    val set: (Double) -> Unit = { wanted ->
        shown = wanted
        onSet(wanted)
    }
    val stepTo: (Int) -> Unit = { direction ->
        shown?.let { current ->
            val wanted = steppedValue(current, step, direction, range)
            shown = wanted
            stepped = wanted
        }
    }
    Column(modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Row(Modifier.fillMaxWidth().heightIn(min = 56.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(label, style = MaterialTheme.typography.bodyLarge)
                note?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            trailing?.invoke()
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                StepButton("−", "Decrease $label", enabled && shown != null) { stepTo(-1) }
                Row(
                    Modifier.widthIn(min = VALUE_MIN_WIDTH).clickable(enabled = enabled) { typing = true }.padding(horizontal = 4.dp),
                    verticalAlignment = Alignment.Bottom,
                    horizontalArrangement = Arrangement.Center,
                ) {
                    Text(stepperText(shown, decimals), style = MaterialTheme.typography.titleLarge, textAlign = TextAlign.Center)
                    if (unit.isNotBlank()) Text(" $unit", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                StepButton("+", "Increase $label", enabled && shown != null) { stepTo(1) }
            }
        }
        if (slider && range != null) {
            Slider(
                value = (shown ?: range.start).toFloat().coerceIn(range.start.toFloat(), range.endInclusive.toFloat()),
                onValueChange = { shown = Math.round(it / step) * step },
                onValueChangeFinished = { shown?.let(onSet) },
                valueRange = range.start.toFloat()..range.endInclusive.toFloat(),
                enabled = enabled,
            )
        }
    }
    if (typing) {
        var text by remember { mutableStateOf(stepperText(shown, decimals).takeIf { shown != null }.orEmpty()) }
        val typed = typedNumber(text)?.let { wanted -> range?.let { wanted.coerceIn(it.start, it.endInclusive) } ?: wanted }
        AlertDialog(
            onDismissRequest = { typing = false },
            title = { Text(label) },
            text = {
                OutlinedTextField(
                    value = text,
                    onValueChange = { text = it },
                    suffix = { if (unit.isNotBlank()) Text(unit) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                )
            },
            confirmButton = {
                TextButton(enabled = typed != null, onClick = {
                    typing = false
                    typed?.let(set)
                }) { Text("Set") }
            },
            dismissButton = { TextButton(onClick = { typing = false }) { Text("Cancel") } },
        )
    }
}
