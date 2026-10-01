package one.aircast.android.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Slider
import androidx.compose.material3.RadioButton
import androidx.compose.ui.Alignment
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import androidx.compose.runtime.DisposableEffect
import one.aircast.mapspike.optText
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

internal const val PX4_TUNING_VIEW = "view.px4Tuning"
internal const val SET_TUNING_TELEMETRY = "vehicle.setPIDTuningTelemetryMode"
private const val DEFAULT_CHART_SECONDS = 8.0

internal data class TuningParam(val title: String, val description: String, val min: Float, val max: Float, val step: Float, val fact: Fact)
internal data class TuningPlot(val name: String, val path: String)
internal data class TuningAxis(val name: String, val chartTitle: String = "", val plot: List<TuningPlot> = emptyList(), val params: List<TuningParam>)
internal data class TuningTab(
    val name: String,
    val title: String,
    val unit: String,
    val extras: List<Fact>,
    val axes: List<TuningAxis>,
    val tuningMode: Int = 0,
    val autoModeChange: Boolean = false,
    val autoTuning: Boolean = false,
    val chartSeconds: Double = DEFAULT_CHART_SECONDS,
)

private fun <T> JSONArray?.mapObjects(read: (JSONObject) -> T?): List<T> =
    (0 until (this?.length() ?: 0)).mapNotNull { index -> this?.optJSONObject(index)?.let(read) }

internal fun tuningTabs(view: JSONObject?): List<TuningTab> =
    view?.takeIf { it.optBoolean("available") }?.optJSONArray("tabs").mapObjects { tab ->
        TuningTab(
            name = tab.optText("name"),
            title = tab.optText("title"),
            unit = tab.optText("unit"),
            extras = tab.optJSONArray("extras").mapObjects(::factFromControl),
            tuningMode = tab.optInt("tuningMode"),
            autoModeChange = tab.optBoolean("autoModeChange"),
            autoTuning = tab.optBoolean("autoTuning"),
            chartSeconds = tab.optDouble("chartSeconds", DEFAULT_CHART_SECONDS),
            axes = tab.optJSONArray("axes").mapObjects { axis ->
                TuningAxis(
                    name = axis.optText("name"),
                    chartTitle = axis.optText("chartTitle"),
                    plot = axis.optJSONArray("plot").mapObjects { TuningPlot(it.optText("name"), it.optText("path")) },
                    params = axis.optJSONArray("params").mapObjects { param ->
                        param.optJSONObject("fact")?.let(::factFromControl)?.let { fact ->
                            TuningParam(
                                title = param.optText("title"),
                                description = param.optText("description"),
                                min = param.optDouble("min").toFloat(),
                                max = param.optDouble("max").toFloat(),
                                step = param.optDouble("step").toFloat(),
                                fact = fact,
                            )
                        }
                    },
                )
            },
        )
    }.orEmpty()

internal data class TuningModes(val stabilized: String, val pause: String)

internal fun tuningModes(view: JSONObject?): TuningModes =
    TuningModes(view?.optText("stabilizedFlightMode").orEmpty(), view?.optText("pauseFlightMode").orEmpty())

internal fun sliderSteps(min: Float, max: Float, step: Float): Int =
    if (step <= 0f || max <= min) 0 else (((max - min) / step).roundToInt() - 1).coerceAtLeast(0)

internal fun factNumber(fact: Fact): Float? = (fact.value as? Number)?.toFloat() ?: fact.valueString.toFloatOrNull()

@Composable
fun Px4TuningScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var tabs by remember { mutableStateOf<List<TuningTab>>(emptyList()) }
    var modes by remember { mutableStateOf(TuningModes("", "")) }
    var useAutoTuning by remember { mutableStateOf(false) }
    var loaded by remember { mutableStateOf(false) }
    var tabIndex by remember { mutableIntStateOf(0) }
    var axisIndex by remember { mutableIntStateOf(0) }
    var clipboard by remember { mutableStateOf<List<Pair<Fact, String>>>(emptyList()) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(revision) {
        val read = withContext(Dispatchers.Default) { Qgc.get(PX4_TUNING_VIEW) }
        tabs = tuningTabs(read)
        modes = tuningModes(read)
        loaded = true
    }

    if (!loaded) {
        Text("Reading parameters from the vehicle.", modifier.padding(16.dp))
        return
    }
    val tab = tabs.getOrNull(tabIndex)
    if (tab == null) {
        Text("This vehicle has no PX4 tuning page.", modifier.padding(16.dp))
        return
    }
    val axis = tab.axes.getOrNull(axisIndex) ?: tab.axes.firstOrNull()

    LaunchedEffect(tabIndex, axisIndex) {
        axis?.let { current -> clipboard = current.params.map { it.fact to it.fact.valueString } }
    }

    DisposableEffect(tab.tuningMode) {
        scope.launch(Dispatchers.IO) { Qgc.invoke(SET_TUNING_TELEMETRY, tab.tuningMode) }
        onDispose { offMainDetached { Qgc.invoke(SET_TUNING_TELEMETRY, 0) } }
    }

    fun write(path: String, value: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.writeRefusal(path, value) }
            revision++
        }
    }

    Column(modifier.fillMaxSize()) {
        ScrollableTabRow(selectedTabIndex = tabIndex) {
            tabs.forEachIndexed { index, each ->
                Tab(selected = index == tabIndex, onClick = { tabIndex = index; axisIndex = 0 }, text = { Text(each.name) })
            }
        }
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            tab.extras.forEach { fact -> FactRow(fact) { revision++ } }
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                tab.axes.forEachIndexed { index, each ->
                    FilterChip(selected = each == axis, onClick = { axisIndex = index }, label = { Text(each.name) })
                }
            }
            axis?.let { TuningChart(it, tab.unit, tab.chartSeconds, if (tab.autoModeChange) modes else null) }
            if (tab.autoTuning) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    RadioButton(selected = useAutoTuning, onClick = { useAutoTuning = true })
                    Text("Use auto-tuning")
                    RadioButton(selected = !useAutoTuning, onClick = { useAutoTuning = false })
                    Text("Use manual tuning")
                }
            }
            if (tab.autoTuning && useAutoTuning) {
                AutotuneSection()
            } else {
                axis?.params?.forEach { param ->
                    TuningSlider(param) { value -> write(param.fact.path, value) }
                }
                axis?.let { current ->
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = { clipboard = current.params.map { it.fact to it.fact.valueString } }) { Text("Save To Clipboard") }
                        OutlinedButton(
                            onClick = { clipboard.forEach { (fact, value) -> value.toDoubleOrNull()?.let { write(fact.path, it) } } },
                            enabled = clipboard.isNotEmpty(),
                        ) { Text("Restore From Clipboard") }
                    }
                }
            }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            if (clipboard.isNotEmpty()) {
                Text("Clipboard Values:", style = MaterialTheme.typography.labelLarge)
                clipboard.forEach { (fact, value) -> Text("${fact.name}  $value", style = MaterialTheme.typography.bodySmall) }
            }
        }
    }
}

@Composable
private fun TuningSlider(param: TuningParam, onWrite: (Float) -> Unit) {
    val current = factNumber(param.fact)
    var dragging by remember(param.fact.path, current) { mutableStateOf(current ?: param.min) }
    Column(Modifier.fillMaxWidth()) {
        Text(param.title, style = MaterialTheme.typography.bodyMedium)
        Text(param.description, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row {
            Slider(
                value = dragging.coerceIn(param.min, param.max),
                onValueChange = { dragging = it },
                onValueChangeFinished = { onWrite(dragging) },
                valueRange = param.min..param.max,
                steps = sliderSteps(param.min, param.max, param.step),
                modifier = Modifier.weight(1f),
            )
            Text(param.fact.valueString, modifier = Modifier.padding(start = 8.dp))
        }
    }
}
