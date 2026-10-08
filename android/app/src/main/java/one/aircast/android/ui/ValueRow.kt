package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.background
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import kotlinx.coroutines.delay
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitHorizontalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.setProgress
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
internal const val VALUE_NO_LIMIT = "No limit"
private const val NO_LIMIT_NOTCH = 0.1f
private const val NO_LIMIT_SNAP = 0.25f
private val WIDE_ROW = 560.dp
private val ROW_SLIDER_WIDTH = 240.dp
private const val SLIDER_SPAN_LIMIT = 100_000.0
private val DISABLED_AT_ZERO = Regex("""(?i)(disabled if 0|0 (disables|to disable|hides|turns .* off)|set to 0 to disable|zero disables)""")

internal fun opensAsValue(fact: Fact): Boolean =
    showsAsField(fact) && !fact.isString && !fact.isBitmask && !fact.isEnum

private fun numberOf(fact: Fact): Double? = factNumber(fact)?.toDouble()

internal fun disablesAtZero(fact: Fact): Boolean =
    DISABLED_AT_ZERO.containsMatchIn(fact.description + " " + fact.longDescription)

internal fun valueText(fact: Fact): String =
    if (disablesAtZero(fact) && numberOf(fact) == 0.0) offLabel(fact)
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

internal fun offLabel(fact: Fact): String = if (fact.name in USEFUL_BANDS) VALUE_NO_LIMIT else VALUE_OFF

internal data class InlineSlider(val from: Float, val top: Float, val noLimitAt: Float?) {
    val end: Float get() = noLimitAt ?: top
}

internal fun inlineSlider(fact: Fact): InlineSlider? = sliderFor(fact)?.let { slider ->
    InlineSlider(slider.from, slider.to, (slider.to + (slider.to - slider.from) * NO_LIMIT_NOTCH).takeIf { disablesAtZero(fact) })
}

internal fun sliderPosition(slider: InlineSlider, value: Double?): Float =
    if (slider.noLimitAt != null && value == 0.0) slider.noLimitAt else (value?.toFloat() ?: slider.from).coerceIn(slider.from, slider.top)

internal fun valueAtPosition(fact: Fact, slider: InlineSlider, position: Float): Double =
    if (slider.noLimitAt != null && position > slider.top + (slider.noLimitAt - slider.top) * NO_LIMIT_SNAP) 0.0
    else clampToBounds(fact, roundedValue(fact, position.coerceAtMost(slider.top).toDouble()))

internal suspend fun writeValueRefusal(fact: Fact, value: Double): String? =
    withContext(Dispatchers.Default) { Qgc.writeRefusal(fact.path, roundedValue(fact, value).let { if (valueDecimals(fact) == 0) it.toLong() else it }) }

internal fun interface ChangeNotice {
    fun show(message: String, undo: (suspend () -> String?)?)
}

internal val LocalChangeNotice = androidx.compose.runtime.staticCompositionLocalOf { ChangeNotice { _, _ -> } }

private const val DONE_MARK_MS = 1500L

@Composable
internal fun SliderValueRow(fact: Fact, label: String, slider: InlineSlider, title: @Composable () -> Unit, onOpen: () -> Unit, onWrite: () -> Unit) {
    val scope = rememberCoroutineScope()
    val notice = LocalChangeNotice.current
    var dragging by remember(fact.path, fact.valueString) { mutableStateOf<Float?>(null) }
    var writing by remember(fact.path) { mutableStateOf(false) }
    var done by remember(fact.path) { mutableStateOf(false) }
    androidx.compose.runtime.LaunchedEffect(done) {
        if (done) {
            delay(DONE_MARK_MS)
            done = false
        }
    }
    val shownValue = dragging?.let { valueAtPosition(fact, slider, it) } ?: numberOf(fact)
    val commit: () -> Unit = {
        val before = numberOf(fact)
        dragging?.let { valueAtPosition(fact, slider, it) }?.takeIf { it != before }?.let { target ->
            scope.launch {
                writing = true
                val refusal = writeValueRefusal(fact, target)
                writing = false
                if (refusal == null) {
                    done = true
                    onWrite()
                    notice.show("$label ${valueTextOf(fact, target)}", before?.let { previous -> { writeValueRefusal(fact, previous).also { if (it == null) onWrite() } } })
                } else {
                    dragging = null
                    notice.show("$label not changed: $refusal", null)
                }
            }
        } ?: run { dragging = null }
    }
    val value: @Composable () -> Unit = {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            when {
                writing -> androidx.compose.material3.CircularProgressIndicator(Modifier.size(14.dp), strokeWidth = 2.dp)
                done -> Text("\u2713", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.primary)
            }
            Text(
                valueTextOf(fact, shownValue),
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.clickable(onClickLabel = "Set exactly", onClick = onOpen).padding(horizontal = 4.dp, vertical = 8.dp),
            )
        }
    }
    val bar: @Composable (Modifier) -> Unit = { modifier ->
        ThinSlider(
            value = dragging ?: sliderPosition(slider, numberOf(fact)),
            range = slider.from..slider.end,
            enabled = fact.acceptsWrite && !writing,
            onChange = { dragging = it },
            onDone = commit,
            modifier = modifier,
        )
    }
    androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp)) {
        if (maxWidth >= WIDE_ROW) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Box(Modifier.weight(1f)) { title() }
                value()
                bar(Modifier.width(ROW_SLIDER_WIDTH))
            }
        } else {
            androidx.compose.foundation.layout.Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.weight(1f)) { title() }
                    value()
                }
                bar(Modifier.fillMaxWidth())
            }
        }
    }
}

private val THUMB_SIZE = 18.dp
private val THUMB_GRAB = 24.dp
private val TRACK_HEIGHT = 2.dp
private val SLIDER_HEIGHT = 48.dp

internal fun thumbFraction(value: Float, range: ClosedFloatingPointRange<Float>): Float {
    val span = range.endInclusive - range.start
    return if (span > 0f) ((value - range.start) / span).coerceIn(0f, 1f) else 0f
}

internal fun grabsThumb(touchX: Float, thumbX: Float, grabRadius: Float): Boolean = kotlin.math.abs(touchX - thumbX) <= grabRadius

@Composable
private fun ThinSlider(value: Float, range: ClosedFloatingPointRange<Float>, enabled: Boolean, onChange: (Float) -> Unit, onDone: () -> Unit, modifier: Modifier) {
    val active = MaterialTheme.colorScheme.onSurface.copy(alpha = if (enabled) 1f else DISABLED_SLIDER_ALPHA)
    val inactive = MaterialTheme.colorScheme.outlineVariant
    val latest by rememberUpdatedState(value)
    val change by rememberUpdatedState(onChange)
    val finish by rememberUpdatedState(onDone)
    androidx.compose.foundation.layout.BoxWithConstraints(
        modifier
            .height(SLIDER_HEIGHT)
            .semantics {
                progressBarRangeInfo = androidx.compose.ui.semantics.ProgressBarRangeInfo(value, range)
                if (enabled) {
                    setProgress { target ->
                        change(target.coerceIn(range))
                        finish()
                        true
                    }
                }
            },
    ) {
        val width = constraints.maxWidth.toFloat()
        val thumb = with(androidx.compose.ui.platform.LocalDensity.current) { THUMB_SIZE.toPx() }
        val grab = with(androidx.compose.ui.platform.LocalDensity.current) { THUMB_GRAB.toPx() }
        val travel = (width - thumb).coerceAtLeast(1f)
        val positionAt: (Float) -> Float = { x -> range.start + ((x - thumb / 2) / travel).coerceIn(0f, 1f) * (range.endInclusive - range.start) }
        androidx.compose.foundation.Canvas(
            Modifier.fillMaxSize().pointerInput(enabled, travel, range) {
                if (enabled) awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    if (grabsThumb(down.position.x, thumb / 2 + travel * thumbFraction(latest, range), grab)) {
                        down.consume()
                        awaitHorizontalTouchSlopOrCancellation(down.id) { moved, _ -> moved.consume() }?.let { drag ->
                            change(positionAt(drag.position.x))
                            horizontalDrag(drag.id) { moved ->
                                change(positionAt(moved.position.x))
                                moved.consume()
                            }
                            finish()
                        }
                    }
                }
            },
        ) {
            val thumbX = thumb / 2 + travel * thumbFraction(value, range)
            val y = size.height / 2
            val stroke = TRACK_HEIGHT.toPx()
            drawLine(inactive, androidx.compose.ui.geometry.Offset(thumb / 2, y), androidx.compose.ui.geometry.Offset(width - thumb / 2, y), stroke)
            drawLine(active, androidx.compose.ui.geometry.Offset(thumb / 2, y), androidx.compose.ui.geometry.Offset(thumbX, y), stroke)
            drawCircle(active, thumb / 2, androidx.compose.ui.geometry.Offset(thumbX, y))
        }
    }
}

private const val DISABLED_SLIDER_ALPHA = 0.4f

private val DANGEROUS_CHOICE = Regex("""(?i)(terminat|disarm|stop motors|kill)""")

internal fun dangerousChoice(label: String): Boolean = DANGEROUS_CHOICE.containsMatchIn(label)

internal fun safeFirst(options: List<String>): List<Int> = options.indices.sortedBy { dangerousChoice(options[it]) }

internal const val DANGER_NOTE = "Motors stop \u2014 the aircraft falls"

@Composable
internal fun SafeChoiceMenu(expanded: Boolean, options: List<String>, onDismiss: () -> Unit, onPick: (Int) -> Unit) {
    var confirming by remember { mutableStateOf<Int?>(null) }
    androidx.compose.material3.DropdownMenu(expanded = expanded, onDismissRequest = onDismiss) {
        safeFirst(options).map { index ->
            val danger = dangerousChoice(options[index])
            if (danger && index == safeFirst(options).firstOrNull { dangerousChoice(options[it]) }) androidx.compose.material3.HorizontalDivider()
            androidx.compose.material3.DropdownMenuItem(
                text = {
                    androidx.compose.foundation.layout.Column {
                        Text(options[index], color = if (danger) MaterialTheme.colorScheme.error else androidx.compose.ui.graphics.Color.Unspecified)
                        if (danger) Text(DANGER_NOTE, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                    }
                },
                onClick = {
                    onDismiss()
                    if (danger) confirming = index else onPick(index)
                },
            )
        }
    }
    confirming?.let { index ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("${options[index]}?") },
            text = { Text("If this happens in flight, the motors stop and the aircraft falls. Only choose it if a falling aircraft is safer than a flying one.") },
            confirmButton = {
                TextButton(onClick = {
                    confirming = null
                    onPick(index)
                }) { Text("Choose it", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { confirming = null }) { Text("Cancel") } },
        )
    }
}

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
    disablesAtZero(fact) && value == 0.0 -> offLabel(fact)
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
            refusal = writeValueRefusal(fact, value)
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
        val limit = offLabel(fact) == VALUE_NO_LIMIT
        if (shown == 0.0) TextButton(onClick = { write(turnOnValue(fact)) }) { Text(if (limit) "Set a limit" else "Turn on") }
        else TextButton(onClick = { write(0.0) }) { Text(if (limit) "Remove the limit" else "Turn off") }
    }
    if (typing) typedEntry()
    refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
}
