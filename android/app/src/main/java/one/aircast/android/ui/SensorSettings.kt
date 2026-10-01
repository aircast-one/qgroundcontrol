package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val SENSOR_SETTINGS_VIEW = "view.sensorSettings"
internal const val SENSOR_SETTINGS_PRIORITY = "sensorSettings.priority"

internal data class CompassSettings(val index: Int, val label: String, val device: String, val use: Fact?, val priority: Int?, val orientation: Fact?, val orientationTitle: String)

internal data class Declination(val manual: Boolean, val autoDecPath: String, val value: Fact?)

internal data class SensorSettings(
    val boardRotation: Fact?,
    val boardTitle: String,
    val compassesWhileCalibrating: Boolean,
    val compasses: List<CompassSettings>,
    val priorities: List<String>,
    val helpSet: String,
    val helpCal: String,
    val simpleAccelHelp: String,
    val declination: Declination?,
)

internal fun sensorSettings(view: JSONObject?): SensorSettings? = view?.takeIf { it.optBoolean("available") }?.let {
    val compasses = it.optJSONArray("compasses")
    val priorities = it.optJSONArray("priorities")
    SensorSettings(
        boardRotation = it.optJSONObject("boardRotation")?.let(::factFromControl),
        boardTitle = it.optText("boardTitle"),
        compassesWhileCalibrating = it.optBoolean("compassesWhileCalibrating"),
        compasses = (0 until (compasses?.length() ?: 0)).mapNotNull { at ->
            compasses!!.optJSONObject(at)?.let { c ->
                CompassSettings(
                    index = c.optInt("index"),
                    label = c.optText("label"),
                    device = c.optText("device"),
                    use = c.optJSONObject("use")?.let(::factFromControl),
                    priority = if (c.isNull("priority")) null else c.optInt("priority"),
                    orientation = c.optJSONObject("orientation")?.let(::factFromControl),
                    orientationTitle = c.optText("orientationTitle"),
                )
            }
        },
        priorities = (0 until (priorities?.length() ?: 0)).map { at -> priorities!!.optString(at) },
        helpSet = it.optText("helpSet"),
        helpCal = it.optText("helpCal"),
        simpleAccelHelp = it.optText("simpleAccelHelp"),
        declination = it.optJSONObject("declination")?.let { d ->
            Declination(d.optBoolean("manual"), d.optText("autoDecPath"), d.optJSONObject("value")?.let(::factFromControl))
        },
    )
}

@Composable
internal fun SensorSettingsBlock(calibrating: Boolean, showCompasses: Boolean, onSimpleAccel: ((Boolean) -> Unit)? = null) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<SensorSettings?>(null) }
    var simple by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) { read = withContext(Dispatchers.Default) { sensorSettings(Qgc.get(SENSOR_SETTINGS_VIEW)) } }
    val settings = read ?: return
    val refresh: () -> Unit = { revision++ }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(if (calibrating) settings.helpCal else settings.helpSet, style = MaterialTheme.typography.bodyMedium)
        settings.boardRotation?.let { FactRow(it, title = settings.boardTitle, onWrite = refresh) }
        onSimpleAccel?.let { report ->
            Text(settings.simpleAccelHelp, style = MaterialTheme.typography.bodySmall)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = simple, onCheckedChange = {
                    simple = it
                    report(it)
                })
                Text("Simple Accelerometer Calibration")
            }
        }
        if (showCompasses && (!calibrating || settings.compassesWhileCalibrating)) {
            settings.compasses.forEach { compass ->
                Text(compass.label, style = MaterialTheme.typography.titleSmall)
                if (compass.device.isNotBlank()) Text(compass.device, style = MaterialTheme.typography.bodySmall)
                compass.use?.let { FactRow(it, title = "Use Compass", onWrite = refresh) }
                compass.priority?.let { slot ->
                    PriorityPicker(settings.priorities, slot) { picked ->
                        scope.launch {
                            withContext(Dispatchers.Default) { Qgc.invoke(SENSOR_SETTINGS_PRIORITY, compass.index, picked) }
                            refresh()
                        }
                    }
                }
                compass.orientation?.let { FactRow(it, title = compass.orientationTitle, onWrite = refresh) }
            }
            settings.declination?.let { declination ->
                Text("Magnetic Declination", style = MaterialTheme.typography.titleSmall)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(checked = declination.manual, onCheckedChange = { manual ->
                        scope.launch {
                            withContext(Dispatchers.Default) { Qgc.set("${declination.autoDecPath}.rawValue", if (manual) 0 else 1) }
                            refresh()
                        }
                    })
                    Text("Manual Magnetic Declination")
                }
                declination.value?.let { FactRow(it.copy(enabled = declination.manual), onWrite = refresh) }
            }
        }
    }
}

@Composable
private fun PriorityPicker(options: List<String>, current: Int, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        OutlinedButton(onClick = { open = true }) { Text(options.getOrElse(current) { "" }) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(text = { Text(option) }, onClick = {
                    open = false
                    onPick(index)
                })
            }
        }
    }
}
