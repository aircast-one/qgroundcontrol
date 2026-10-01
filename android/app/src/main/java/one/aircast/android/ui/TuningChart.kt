package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.background
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.offMainDetached
import androidx.compose.material3.Checkbox

private const val SAMPLE_MS = 10L
internal const val FLIGHT_MODE_PATH = "vehicle.flightMode"
private const val HISTORY_SECONDS = 180.0
private val SERIES_COLOURS = listOf(Color(0xFF2196F3), Color(0xFFFF9800))

internal data class Sample(val seconds: Double, val value: Double)

internal fun withSample(series: List<Sample>, sample: Sample): List<Sample> =
    (series + sample).dropWhile { it.seconds < sample.seconds - HISTORY_SECONDS }

private const val TICK_SEPARATION = 5.0

private fun grownLow(current: Double, value: Double): Double {
    val low = minOf(current, value)
    return if (low % TICK_SEPARATION != 0.0) kotlin.math.floor((low - TICK_SEPARATION) / TICK_SEPARATION) * TICK_SEPARATION else low
}

private fun grownHigh(current: Double, value: Double): Double {
    val high = maxOf(current, value)
    return if (high % TICK_SEPARATION != 0.0) kotlin.math.floor((high + TICK_SEPARATION) / TICK_SEPARATION) * TICK_SEPARATION else high
}

internal fun grownRange(range: Pair<Double, Double>?, values: List<Double>): Pair<Double, Double>? =
    values.fold(range) { held, value -> held?.let { (low, high) -> grownLow(low, value) to grownHigh(high, value) } ?: (value to value) }

private fun factValue(path: String): Double? = Qgc.get(path).optDouble("value").takeIf { !it.isNaN() }

@Composable
internal fun TuningChart(axis: TuningAxis, unit: String, windowSeconds: Double, modes: TuningModes?) {
    var running by remember(axis) { mutableStateOf(true) }
    var autoModeChange by remember { mutableStateOf(false) }
    var cleared by remember(axis) { mutableStateOf(0) }
    var series by remember(axis, cleared) { mutableStateOf(axis.plot.map { emptyList<Sample>() }) }
    var now by remember(axis, cleared) { mutableStateOf(0.0) }
    var range by remember(axis, cleared) { mutableStateOf<Pair<Double, Double>?>(null) }
    val armed = flyState(qgcPath(FLY_STATE).value)?.armed == true

    LaunchedEffect(armed) {
        if (armed && !running) running = true
    }

    LaunchedEffect(axis, cleared, running) {
        if (!running) return@LaunchedEffect
        val started = System.nanoTime() - (now * 1e9).toLong()
        while (isActive) {
            val values = withContext(Dispatchers.Default) { axis.plot.map { factValue(it.path) } }
            now = (System.nanoTime() - started) / 1e9
            series = series.zip(values) { list, value -> value?.let { withSample(list, Sample(now, it)) } ?: list }
            range = grownRange(range, values.filterNotNull())
            delay(SAMPLE_MS)
        }
    }

    val from = now - windowSeconds
    val grid = MaterialTheme.colorScheme.outlineVariant
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(axis.chartTitle, style = MaterialTheme.typography.titleSmall)
        Canvas(Modifier.fillMaxWidth().height(180.dp).background(MaterialTheme.colorScheme.surfaceContainer)) {
            drawLine(grid, Offset(0f, size.height / 2), Offset(size.width, size.height / 2))
            val (low, high) = range ?: return@Canvas
            val span = (high - low).takeIf { it > 0 } ?: 1.0
            series.forEachIndexed { index, list ->
                val shown = list.filter { it.seconds >= from }
                if (shown.size < 2) return@forEachIndexed
                val path = Path()
                shown.forEachIndexed { at, sample ->
                    val x = ((sample.seconds - from) / windowSeconds * size.width).toFloat()
                    val y = (size.height - (sample.value - low) / span * size.height).toFloat()
                    if (at == 0) path.moveTo(x, y) else path.lineTo(x, y)
                }
                drawPath(path, SERIES_COLOURS[index % SERIES_COLOURS.size], style = Stroke(2.dp.toPx()))
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            axis.plot.forEachIndexed { index, plot ->
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    Box(Modifier.size(10.dp).background(SERIES_COLOURS[index % SERIES_COLOURS.size]))
                    Text(plot.name, style = MaterialTheme.typography.labelSmall)
                }
            }
            Text(range?.let { "%.2f … %.2f %s".format(it.first, it.second, unit) }.orEmpty(), style = MaterialTheme.typography.labelSmall)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = { cleared++ }) { Text("Clear") }
            OutlinedButton(onClick = {
                running = !running
                if (modes != null && autoModeChange) {
                    val mode = if (running) modes.stabilized else modes.pause
                    offMainDetached { Qgc.writeRefusal(FLIGHT_MODE_PATH, mode) }
                }
            }) { Text(if (running) "Stop" else "Start") }
        }
        modes?.let { names ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = autoModeChange, onCheckedChange = { checked ->
                    autoModeChange = checked
                    if (checked) running = false
                })
                Text("Automatic Flight Mode Switching")
            }
            if (autoModeChange) {
                Text("Switches to 'Stabilized' when you click Start.", style = MaterialTheme.typography.bodySmall)
                Text("Switches to '${names.pause}' when you click Stop.", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
