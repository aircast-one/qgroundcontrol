package one.aircast.android.ui

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import org.json.JSONObject

private const val DISPLAY_STORE = "fly-instrument-display"
private const val DEFAULT_RANGE_LOW = 0.0
private const val DEFAULT_RANGE_HIGH = 100.0
private const val GREEN = 0xFF00FF00
internal val RANGE_COLOURS = listOf(GREEN, 0xFFFFFF00, 0xFFFFA500, 0xFFFF0000, 0xFF0000FF, 0xFFFFFFFF)

internal data class ValueDisplay(
    val text: String = "",
    val showUnits: Boolean = true,
    val colourRange: Boolean = false,
    val values: List<Double> = emptyList(),
    val colours: List<Long> = emptyList(),
)

internal fun withColourRange(display: ValueDisplay, on: Boolean): ValueDisplay = when (on) {
    true -> display.copy(colourRange = true, values = listOf(DEFAULT_RANGE_LOW, DEFAULT_RANGE_HIGH), colours = List(3) { GREEN })
    false -> display.copy(colourRange = false, values = emptyList(), colours = emptyList())
}

internal fun withRow(display: ValueDisplay): ValueDisplay =
    display.copy(values = display.values + ((display.values.lastOrNull() ?: DEFAULT_RANGE_LOW) + 1), colours = display.colours + GREEN)

internal fun withoutRow(display: ValueDisplay, index: Int): ValueDisplay =
    display.copy(values = display.values.filterIndexed { i, _ -> i != index }, colours = display.colours.filterIndexed { i, _ -> i != index })

internal fun rangeIndex(raw: Double?, values: List<Double>): Int =
    raw?.takeIf { !it.isNaN() }?.let { value -> values.indexOfFirst { value <= it }.takeIf { it >= 0 } ?: values.size } ?: 0

internal fun displayColour(display: ValueDisplay, raw: Double?): Long? =
    display.takeIf { it.colourRange }?.colours?.getOrNull(rangeIndex(raw, display.values))

internal fun displayReading(display: ValueDisplay, value: String, units: String): String =
    if (display.showUnits && units.isNotBlank()) "$value $units" else value

internal fun displayJson(display: ValueDisplay): String = JSONObject()
    .put("text", display.text)
    .put("showUnits", display.showUnits)
    .put("colourRange", display.colourRange)
    .put("values", JSONArray(display.values))
    .put("colours", JSONArray(display.colours))
    .toString()

internal fun displayFrom(json: String?): ValueDisplay = runCatching {
    JSONObject(json ?: return ValueDisplay()).let { o ->
        val values = o.optJSONArray("values")
        val colours = o.optJSONArray("colours")
        ValueDisplay(
            text = o.optString("text"),
            showUnits = o.optBoolean("showUnits", true),
            colourRange = o.optBoolean("colourRange"),
            values = (0 until (values?.length() ?: 0)).mapNotNull { values?.optDouble(it) },
            colours = (0 until (colours?.length() ?: 0)).mapNotNull { colours?.optLong(it) },
        )
    }
}.getOrDefault(ValueDisplay())

internal fun readDisplays(context: Context): Map<String, ValueDisplay> =
    context.getSharedPreferences(DISPLAY_STORE, Context.MODE_PRIVATE).all.mapValues { (_, json) -> displayFrom(json as? String) }

internal fun writeDisplay(context: Context, id: String, display: ValueDisplay) {
    context.getSharedPreferences(DISPLAY_STORE, Context.MODE_PRIVATE).edit().putString(id, displayJson(display)).apply()
}

@Composable
private fun Swatch(colour: Long, chosen: Boolean, onClick: () -> Unit) {
    val ring = if (chosen) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline
    Box(Modifier.size(24.dp).background(Color(colour), CircleShape).border(2.dp, ring, CircleShape).clickable(onClick = onClick))
}

@Composable
internal fun ValueDisplayDialog(label: String, initial: ValueDisplay, onDismiss: () -> Unit, onDone: (ValueDisplay) -> Unit) {
    var display by remember { mutableStateOf(initial) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Telemetry Display") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(label, style = MaterialTheme.typography.labelMedium)
                OutlinedTextField(value = display.text, onValueChange = { display = display.copy(text = it) }, label = { Text("Text") }, singleLine = true)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Show Units", modifier = Modifier.weight(1f))
                    Switch(checked = display.showUnits, onCheckedChange = { display = display.copy(showUnits = it) })
                }
                Text("Value Range", style = MaterialTheme.typography.titleSmall)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(selected = !display.colourRange, onClick = { display = withColourRange(display, false) }, label = { Text("None") })
                    FilterChip(selected = display.colourRange, onClick = { display = withColourRange(display, true) }, label = { Text("Color") })
                }
                if (display.colourRange) {
                    Text("Specify the color you want to apply based on value ranges.", style = MaterialTheme.typography.bodySmall)
                    display.colours.indices.forEach { row ->
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            display.values.getOrNull(row)?.let { limit ->
                                OutlinedTextField(
                                    value = limit.toString(),
                                    onValueChange = { typed -> typed.toDoubleOrNull()?.let { v -> display = display.copy(values = display.values.mapIndexed { i, old -> if (i == row) v else old }) } },
                                    label = { Text("≤") },
                                    singleLine = true,
                                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                    modifier = Modifier.width(96.dp),
                                )
                            } ?: Text("above", style = MaterialTheme.typography.bodySmall, modifier = Modifier.width(96.dp))
                            RANGE_COLOURS.forEach { colour ->
                                Swatch(colour, display.colours[row] == colour) {
                                    display = display.copy(colours = display.colours.mapIndexed { i, old -> if (i == row) colour else old })
                                }
                            }
                            if (row < display.values.size && display.values.size > 1) TextButton(onClick = { display = withoutRow(display, row) }) { Text("✕") }
                        }
                    }
                    TextButton(onClick = { display = withRow(display) }) { Text("Add Row") }
                }
            }
        },
        confirmButton = { TextButton(onClick = { onDone(display) }) { Text("Done") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
