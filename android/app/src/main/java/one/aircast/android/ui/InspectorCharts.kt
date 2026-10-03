package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Surface
import androidx.compose.ui.Alignment
import one.aircast.mapspike.aircast
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import java.util.TimeZone
import java.util.Locale
import kotlinx.coroutines.delay
import androidx.compose.runtime.produceState
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import org.json.JSONArray
import org.json.JSONObject

internal const val INSPECTOR_CHARTS_VIEW = "view.inspectorCharts"
private const val CHART_ADD = "mavlinkInspector.chart.add"
private const val CHART_REMOVE = "mavlinkInspector.chart.remove"
private const val CHART_RANGE_X = "mavlinkInspector.chart.rangeX"
private const val CHART_RANGE_Y = "mavlinkInspector.chart.rangeY"

internal data class ChartPlot(val label: String, val field: String, val colour: Int, val points: List<Pair<Long, Double>>)

internal data class ChartModel(val rangeX: Int, val rangeY: Int, val windowMs: Long, val yMin: Double?, val yMax: Double?, val room: Boolean, val plots: List<ChartPlot>)

internal data class InspectorCharts(val timeScales: List<String>, val ranges: List<String>, val charts: List<ChartModel>, val charted: Map<String, Int>)

private fun strings(array: JSONArray?): List<String> = (0 until (array?.length() ?: 0)).map { array?.optString(it).orEmpty() }

private fun JSONObject.number(key: String): Double? = if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

internal fun inspectorCharts(view: JSONObject?): InspectorCharts? = view?.optJSONArray("charts")?.let { charts ->
    val charted = view.optJSONArray("selectedCharted")
    InspectorCharts(
        timeScales = strings(view.optJSONArray("timeScales")),
        ranges = strings(view.optJSONArray("ranges")),
        charts = (0 until charts.length()).mapNotNull { charts.optJSONObject(it) }.map { chart ->
            val plots = chart.optJSONArray("plots")
            ChartModel(
                rangeX = chart.optInt("rangeX"),
                rangeY = chart.optInt("rangeY"),
                windowMs = chart.optLong("windowMs", 5000),
                yMin = chart.number("yMin"),
                yMax = chart.number("yMax"),
                room = chart.optBoolean("room", true),
                plots = (0 until (plots?.length() ?: 0)).mapNotNull { plots?.optJSONObject(it) }.map { plot ->
                    val points = plot.optJSONArray("points")
                    ChartPlot(
                        label = plot.optString("label"),
                        field = plot.optString("field"),
                        colour = plot.optInt("colour"),
                        points = (0 until (points?.length() ?: 0)).mapNotNull { points?.optJSONArray(it) }.map { it.optLong(0) to it.optDouble(1) },
                    )
                },
            )
        },
        charted = (0 until (charted?.length() ?: 0)).mapNotNull { charted?.optJSONObject(it) }.associate { it.optString("field") to it.optInt("chart") },
    )
}

private val CHART_HEIGHT = 220.dp
private const val TIME_TICKS = 3
private const val TICK_LABEL_CACHE = 16
private const val CLOCK_TICK_MS = 1000L
private const val VALUE_TICKS = 4

internal fun timeTicks(windowMs: Long, nowMs: Long, zone: TimeZone = TimeZone.getDefault()): List<Pair<Float, String>> =
    (0..TIME_TICKS).map { step ->
        val at = step.toFloat() / TIME_TICKS
        val instant = nowMs - ((1f - at) * windowMs).toLong()
        val wallClock = instant + zone.getOffset(instant)
        at to "%02d:%02d".format(Locale.ROOT, (wallClock / 60_000) % 60, (wallClock / 1000) % 60)
    }

internal fun valueTicks(low: Double, high: Double): List<Pair<Float, String>> =
    (0..VALUE_TICKS).map { step ->
        val fraction = step.toDouble() / VALUE_TICKS
        (1f - fraction.toFloat()) to "%.4g".format(Locale.ROOT, low + (high - low) * fraction)
    }

internal fun latestValue(points: List<Pair<Long, Double>>): String? =
    points.minByOrNull { it.first }?.let { "%.4g".format(Locale.ROOT, it.second) }

@Composable
private fun seriesColours(): List<Color> = listOf(
    MaterialTheme.colorScheme.primary,
    MaterialTheme.aircast.mission,
    MaterialTheme.aircast.success,
    MaterialTheme.colorScheme.tertiary,
    MaterialTheme.colorScheme.error,
    MaterialTheme.aircast.warning,
)

internal fun chartPoint(ageMs: Long, value: Double, windowMs: Long, yMin: Double, yMax: Double): Pair<Float, Float> =
    (1f - ageMs.toFloat() / windowMs.coerceAtLeast(1)) to (1f - ((value - yMin) / (yMax - yMin).takeIf { it > 0 }.let { it ?: 1.0 }).toFloat().coerceIn(0f, 1f))

internal fun fieldChartable(type: String): Boolean = !type.startsWith("char")

internal fun chartToggleEnabled(charts: InspectorCharts?, field: String, type: String, chart: Int): Boolean {
    val on = charts?.charted?.get(field)
    return when {
        on == chart -> true
        !fieldChartable(type) || on != null -> false
        else -> charts?.charts?.getOrNull(chart)?.room ?: true
    }
}

internal fun toggleChartField(chart: Int, field: String, on: Boolean) {
    one.aircast.android.bridge.offMainInOrder {
        Qgc.invoke(if (on) CHART_ADD else CHART_REMOVE, chart, field)
    }
}

@Composable
private fun ChartChoice(title: String, options: List<String>, chosen: Int, onChosen: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        TextButton(onClick = { open = true }) { Text("$title: ${options.getOrNull(chosen).orEmpty()}") }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(text = { Text(option) }, onClick = {
                    open = false
                    onChosen(index)
                })
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun InspectorChartPanel(index: Int, charts: InspectorCharts, modifier: Modifier = Modifier) {
    val chart = charts.charts.getOrNull(index)?.takeIf { it.plots.isNotEmpty() } ?: return
    val grid = MaterialTheme.colorScheme.outlineVariant
    Column(modifier.fillMaxWidth().padding(horizontal = 12.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("Chart ${index + 1}", style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 12.dp))
            ChartChoice("Scale", charts.timeScales, chart.rangeX) { chosen -> offMainDetached { Qgc.invoke(CHART_RANGE_X, index, chosen) } }
            ChartChoice("Range", charts.ranges, chart.rangeY) { chosen -> offMainDetached { Qgc.invoke(CHART_RANGE_Y, index, chosen) } }
        }
        val colours = seriesColours()
        val measurer = rememberTextMeasurer(cacheSize = TICK_LABEL_CACHE)
        val ink = MaterialTheme.colorScheme.onSurfaceVariant
        val tickStyle = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace, color = ink)
        val now by produceState(System.currentTimeMillis()) {
            while (true) {
                delay(CLOCK_TICK_MS)
                value = System.currentTimeMillis()
            }
        }
        Surface(Modifier.fillMaxWidth().padding(top = 8.dp), shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainer) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Canvas(Modifier.fillMaxWidth().height(CHART_HEIGHT)) {
                    val low = chart.yMin ?: return@Canvas
                    val high = chart.yMax ?: return@Canvas
                    val values = valueTicks(low, high).map { (at, label) -> at to measurer.measure(label, tickStyle) }
                    val times = timeTicks(chart.windowMs, now).map { (at, label) -> at to measurer.measure(label, tickStyle) }
                    val gap = 4.dp.toPx()
                    val left = (values.maxOfOrNull { it.second.size.width } ?: 0) + gap
                    val plotHeight = size.height - (times.maxOfOrNull { it.second.size.height } ?: 0) - gap
                    val plotWidth = size.width - left
                    values.forEach { (at, text) ->
                        val y = at * plotHeight
                        drawLine(grid, Offset(left, y), Offset(size.width, y), strokeWidth = 1f)
                        drawText(text, topLeft = Offset(left - gap - text.size.width, (y - text.size.height / 2f).coerceIn(0f, plotHeight - text.size.height)))
                    }
                    times.forEach { (at, text) ->
                        val x = left + at * plotWidth
                        drawText(text, topLeft = Offset((x - text.size.width / 2f).coerceIn(left, size.width - text.size.width), plotHeight + gap))
                    }
                    chart.plots.forEach { plot ->
                        val path = Path().apply {
                            plot.points.forEachIndexed { at, (age, value) ->
                                val (x, y) = chartPoint(age, value, chart.windowMs, low, high)
                                val point = Offset(left + x * plotWidth, y * plotHeight)
                                if (at == 0) moveTo(point.x, point.y) else lineTo(point.x, point.y)
                            }
                        }
                        drawPath(path, colours[plot.colour % colours.size], style = Stroke(2.dp.toPx()))
                    }
                }
                FlowRow(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally)) {
                    chart.plots.forEach { plot ->
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            Box(Modifier.size(8.dp).background(colours[plot.colour % colours.size], CircleShape))
                            Text(listOfNotNull(plot.label, latestValue(plot.points)).joinToString("  "), style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            }
        }
    }
}
