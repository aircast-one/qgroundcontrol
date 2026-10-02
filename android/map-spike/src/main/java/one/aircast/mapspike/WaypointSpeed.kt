package one.aircast.mapspike

import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import org.json.JSONObject

data class WaypointSpeed(val specified: Boolean, val value: Double?, val units: String, val path: String, val specifyPath: String)

fun waypointSpeed(view: JSONObject?): WaypointSpeed? =
    view?.optJSONObject("speedSection")?.takeIf { it.optBoolean("available") }?.let {
        WaypointSpeed(
            specified = it.optBoolean("specified"),
            value = if (it.isNull("value")) null else it.optDouble("value"),
            units = it.optText("units"),
            path = it.optText("path"),
            specifyPath = it.optText("specifyPath"),
        )
    }

sealed interface SpeedEntry {
    data object Clear : SpeedEntry
    data class Set(val value: Double) : SpeedEntry
    data object Invalid : SpeedEntry
}

fun speedEntry(text: String): SpeedEntry = text.trim().replace(',', '.').let { typed ->
    when {
        typed.isEmpty() -> SpeedEntry.Clear
        else -> typed.toDoubleOrNull()?.takeIf { it.isFinite() && it > 0.0 }?.let { SpeedEntry.Set(it) } ?: SpeedEntry.Invalid
    }
}

fun speedFieldText(speed: WaypointSpeed): String =
    speed.value?.takeIf { speed.specified }?.let { plainSpeed(it) }.orEmpty()

private fun plainSpeed(value: Double): String =
    if (value == kotlin.math.floor(value)) value.toLong().toString() else value.toString()

@Composable
fun WaypointSpeedField(index: Int, onWrite: (label: String, work: () -> Boolean) -> Unit, onRefused: (String) -> Unit) {
    val json by mapPath("view.itemFacts($index)")
    val speed = waypointSpeed(json) ?: return
    var typed by remember(index, speed.specified, speed.value) { mutableStateOf(speedFieldText(speed)) }
    OutlinedTextField(
        value = typed,
        onValueChange = { typed = it },
        label = { Text(listOf("Speed", speed.units).filter { it.isNotBlank() }.joinToString(" ")) },
        placeholder = { Text("Auto") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = {
            when (val entry = speedEntry(typed)) {
                SpeedEntry.Clear -> onWrite("Clearing the speed") { setOk(speed.specifyPath, settingJson("false")) }
                is SpeedEntry.Set -> onWrite("Setting the speed") { setOk(speed.specifyPath, settingJson("true")) && setOk(speed.path, settingJson("${entry.value}")) }
                SpeedEntry.Invalid -> onRefused("Not a speed")
            }
        }),
        textStyle = MaterialTheme.typography.bodySmall,
        modifier = Modifier.width(SPEED_FIELD_WIDTH),
    )
}

private val SPEED_FIELD_WIDTH = 120.dp
