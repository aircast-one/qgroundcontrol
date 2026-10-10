package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import one.aircast.map.aircast
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import androidx.compose.ui.unit.Dp
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
    val flyScreen = LocalFlyScreenState.current
    val portrait = flyIsPortrait()
    BoxWithConstraints(modifier.fillMaxWidth()) {
    CompositionLocalProvider(LocalGuidedCompact provides guidedCompact(portrait, maxHeight)) {
    Surface(
        Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        shape = MaterialTheme.shapes.extraLarge,
    ) {
        DisposableEffect(Unit) {
            flyScreen.guidedPanels++
            onDispose { flyScreen.guidedPanels-- }
        }
        val flyJson by one.aircast.android.bridge.qgcPath(FLY_STATE)
        val readiness = remember(flyJson) { guidedReadiness(flyState(flyJson)) }
        val vehiclesJson by one.aircast.android.bridge.qgcPath(one.aircast.map.VEHICLES_VIEW)
        val vehicles = remember(vehiclesJson) { one.aircast.map.vehicleChoices(vehiclesJson) }
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Column(
                Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(horizontal = 8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(title, style = MaterialTheme.typography.titleLarge)
                guidedVehicle(vehicles.choices.size, vehicles.active?.name)?.let {
                    Text(it, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                }
                if (sentence.isNotBlank()) {
                    Text(sentence, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                readiness?.let { warning ->
                    Surface(
                        color = if (warning.blocks) MaterialTheme.colorScheme.errorContainer else MaterialTheme.aircast.warningContainer,
                        contentColor = if (warning.blocks) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onSurface,
                        shape = MaterialTheme.shapes.medium,
                    ) {
                        Text(warning.text, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(12.dp))
                    }
                }
                content()
            }
            val hold = holdLabel(guidedCommitLabel(commitLabel, readiness))
            val holdEnabled = commitEnabled && readiness?.blocks != true
            if (portrait) {
                HoldToConfirm(label = hold, enabled = holdEnabled, onConfirm = onCommit)
                TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("Cancel") }
            } else {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    HoldToConfirm(label = hold, enabled = holdEnabled, modifier = Modifier.weight(1f), onConfirm = onCommit)
                    TextButton(onClick = onCancel) { Text("Cancel") }
                }
            }
        }
    }
    }
    }
}

internal val LocalGuidedCompact = staticCompositionLocalOf { false }

private val COMPACT_PANEL_HEIGHT = 440.dp

internal fun guidedCompact(portrait: Boolean, available: Dp): Boolean = !portrait && available < COMPACT_PANEL_HEIGHT

internal fun guidedCommitLabel(label: String, readiness: Readiness?): String {
    val (action, target) = label.split(" \u00b7 ", limit = 2).let { it.first() to it.getOrNull(1) }
    val anyway = if (readiness != null && !readiness.blocks) "$action anyway" else action
    return listOfNotNull(anyway, target).joinToString(" \u00b7 ")
}

internal fun guidedPresets(unit: String, minimum: Double, maximum: Double): List<Double> =
    (if (unit == "ft") listOf(15.0, 30.0, 60.0, 150.0) else listOf(5.0, 10.0, 20.0, 50.0)).filter { it in minimum..maximum }

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun GuidedPresets(value: Double, minimum: Double, maximum: Double, unit: String, onValue: (Double) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        guidedPresets(unit, minimum, maximum).map { preset ->
            FilterChip(selected = preset == value, onClick = { onValue(preset) }, label = { Text("${guidedValueText(preset, unit).removeSuffix(".0")} $unit") })
        }
    }
}

internal fun guidedVehicle(vehicleCount: Int, activeName: String?): String? =
    activeName?.takeIf { vehicleCount >= 2 && it.isNotBlank() }

private val METRIC_UNITS = setOf("m", "m/s", "km/h")

internal fun guidedBounds(minimum: Double?, maximum: Double?): Pair<Double, Double>? =
    minimum?.let { low -> maximum?.let { high -> low to high } }

internal fun guidedDecimals(unit: String): Int = if (unit in METRIC_UNITS) 1 else 0

internal fun guidedRounded(value: Double, unit: String): Double =
    BigDecimal(value).setScale(guidedDecimals(unit), RoundingMode.HALF_UP).toDouble()

internal fun guidedStepped(value: Double, delta: Int, minimum: Double, maximum: Double, unit: String): Double =
    guidedRounded((value + delta).coerceIn(minimum, maximum), unit)

internal fun guidedTyped(text: String, minimum: Double, maximum: Double, unit: String): Double? =
    one.aircast.map.typedNumber(text)?.let { guidedRounded(it.coerceIn(minimum, maximum), unit) }

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
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        val narrow = maxWidth < GUIDED_STEPPER_WIDE
        val small = narrow || LocalGuidedCompact.current
        val reading: @Composable () -> Unit = {
            Column {
                val typed = typing
                if (typed == null) {
                    Text(
                        guidedReading(value, unit),
                        style = (if (small) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.displaySmall).copy(fontFeatureSettings = "tnum"),
                        maxLines = 1,
                        softWrap = false,
                        modifier = Modifier.clickable { typing = guidedValueText(value, unit) },
                    )
                } else {
                    val focus = remember { FocusRequester() }
                    LaunchedEffect(Unit) { focus.requestFocus() }
                    OutlinedTextField(
                        value = typed,
                        onValueChange = { text ->
                            typing = text
                            guidedTyped(text, minimum, maximum, unit)?.let(onValue)
                        },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = {
                            focusManager.clearFocus()
                            typing = null
                        }),
                        modifier = Modifier.width(160.dp).focusRequester(focus),
                    )
                }
                if (label.isNotBlank()) Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        val steps: @Composable () -> Unit = {
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
        if (narrow) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                reading()
                steps()
            }
        } else {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.weight(1f)) { reading() }
                steps()
            }
        }
    }
}

private val GUIDED_STEPPER_WIDE = 220.dp

internal fun guidedReading(value: Double, unit: String): String =
    listOf(guidedValueText(value, unit), unit).filter { it.isNotBlank() }.joinToString(" ")
