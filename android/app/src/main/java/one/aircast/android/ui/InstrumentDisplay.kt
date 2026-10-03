package one.aircast.android.ui

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import coil3.compose.AsyncImage
import coil3.svg.SvgDecoder
import org.json.JSONArray
import org.json.JSONObject

private const val DISPLAY_STORE = "fly-instrument-display"
private const val DEFAULT_RANGE_LOW = 0.0
private const val DEFAULT_RANGE_HIGH = 100.0
private const val GREEN = 0xFF008000
internal const val NO_COLOUR = 0L
private const val ICON_FOLDER = "InstrumentValueIcons"
internal val RANGE_COLOURS = listOf(GREEN, 0xFFFFFF00, 0xFFFFA500, 0xFFFF0000, 0xFF0000FF, 0xFFFFFFFF, NO_COLOUR)

internal enum class RangeType(val label: String) { None("None"), Color("Color"), Opacity("Opacity"), Icon("Icon") }

internal data class ValueDisplay(
    val text: String = "",
    val showUnits: Boolean = true,
    val showIcon: Boolean = false,
    val icon: String = "",
    val rangeType: RangeType = RangeType.None,
    val values: List<Double> = emptyList(),
    val colours: List<Long> = emptyList(),
    val opacities: List<Double> = emptyList(),
    val icons: List<String> = emptyList(),
)

internal fun withRangeType(display: ValueDisplay, type: RangeType, firstIcon: String): ValueDisplay {
    val values = if (type == RangeType.None) emptyList() else listOf(DEFAULT_RANGE_LOW, DEFAULT_RANGE_HIGH)
    val slots = if (type == RangeType.None) 0 else values.size + 1
    return display.copy(
        rangeType = type,
        values = values,
        colours = if (type == RangeType.Color) List(slots) { GREEN } else emptyList(),
        opacities = if (type == RangeType.Opacity) List(slots) { 1.0 } else emptyList(),
        icons = if (type == RangeType.Icon) List(slots) { firstIcon } else emptyList(),
    )
}

internal fun withRow(display: ValueDisplay, firstIcon: String): ValueDisplay = display.copy(
    values = display.values + ((display.values.lastOrNull() ?: DEFAULT_RANGE_LOW) + 1),
    colours = if (display.rangeType == RangeType.Color) display.colours + GREEN else display.colours,
    opacities = if (display.rangeType == RangeType.Opacity) display.opacities + 1.0 else display.opacities,
    icons = if (display.rangeType == RangeType.Icon) display.icons + firstIcon else display.icons,
)

internal fun withoutRow(display: ValueDisplay, index: Int): ValueDisplay = display.copy(
    values = display.values.filterIndexed { i, _ -> i != index },
    colours = display.colours.filterIndexed { i, _ -> i != index + 1 },
    opacities = display.opacities.filterIndexed { i, _ -> i != index + 1 },
    icons = display.icons.filterIndexed { i, _ -> i != index + 1 },
)

internal fun rangeIndex(raw: Double?, values: List<Double>): Int =
    raw?.takeIf { !it.isNaN() }?.let { value -> values.indexOfFirst { value <= it }.takeIf { it >= 0 } ?: values.size } ?: 0

internal fun displayColour(display: ValueDisplay, raw: Double?): Long? =
    display.takeIf { it.rangeType == RangeType.Color }?.colours?.getOrNull(rangeIndex(raw, display.values))?.takeIf { it != NO_COLOUR }

internal fun displayOpacity(display: ValueDisplay, raw: Double?): Float =
    display.takeIf { it.rangeType == RangeType.Opacity }?.opacities?.getOrNull(rangeIndex(raw, display.values))?.toFloat()?.coerceIn(0f, 1f) ?: 1f

internal fun displayIcon(display: ValueDisplay, raw: Double?): String? = when {
    display.rangeType == RangeType.Icon -> display.icons.getOrNull(rangeIndex(raw, display.values))
    display.showIcon -> display.icon
    else -> null
}?.takeIf { it.isNotBlank() }

internal fun iconShown(display: ValueDisplay): Boolean = display.rangeType == RangeType.Icon || display.showIcon

internal fun displayReading(display: ValueDisplay, value: String, units: String): String =
    if (display.showUnits && units.isNotBlank()) "$value $units" else value

internal fun displayJson(display: ValueDisplay): String = JSONObject()
    .put("text", display.text)
    .put("showUnits", display.showUnits)
    .put("showIcon", display.showIcon)
    .put("icon", display.icon)
    .put("rangeType", display.rangeType.name)
    .put("values", JSONArray(display.values))
    .put("colours", JSONArray(display.colours))
    .put("opacities", JSONArray(display.opacities))
    .put("icons", JSONArray(display.icons))
    .toString()

private fun <T> JSONObject.list(key: String, read: JSONArray.(Int) -> T?): List<T> =
    optJSONArray(key)?.let { array -> (0 until array.length()).mapNotNull { array.read(it) } }.orEmpty()

internal fun displayFrom(json: String?): ValueDisplay = runCatching {
    JSONObject(json ?: return ValueDisplay()).let { o ->
        ValueDisplay(
            text = o.optString("text"),
            showUnits = o.optBoolean("showUnits", true),
            showIcon = o.optBoolean("showIcon"),
            icon = o.optString("icon"),
            rangeType = RangeType.entries.firstOrNull { it.name == o.optString("rangeType") } ?: RangeType.None,
            values = o.list("values") { optDouble(it) },
            colours = o.list("colours") { optLong(it) },
            opacities = o.list("opacities") { optDouble(it) },
            icons = o.list("icons") { optString(it) },
        )
    }
}.getOrDefault(ValueDisplay())

internal fun readDisplays(context: Context, vehicleClass: String): Map<String, ValueDisplay> =
    context.getSharedPreferences(DISPLAY_STORE, Context.MODE_PRIVATE).all
        .filterKeys { it.startsWith("$vehicleClass/") }
        .entries.associate { (key, json) -> key.removePrefix("$vehicleClass/") to displayFrom(json as? String) }

internal fun writeDisplay(context: Context, vehicleClass: String, id: String, display: ValueDisplay) {
    context.getSharedPreferences(DISPLAY_STORE, Context.MODE_PRIVATE).edit().putString("$vehicleClass/$id", displayJson(display)).apply()
}

internal fun iconNames(context: Context): List<String> =
    context.assets.list(ICON_FOLDER).orEmpty().filter { it.endsWith(".svg") }.sorted()

internal object IconLoader {
    @Volatile private var loader: ImageLoader? = null

    fun of(context: Context): ImageLoader =
        loader ?: synchronized(this) {
            loader ?: ImageLoader.Builder(context.applicationContext).components { add(SvgDecoder.Factory()) }.build().also { loader = it }
        }
}

@Composable
internal fun ValueIcon(name: String, tint: Color, size: Dp, modifier: Modifier = Modifier) {
    AsyncImage(
        model = "file:///android_asset/$ICON_FOLDER/$name",
        imageLoader = IconLoader.of(LocalContext.current),
        contentDescription = name.removeSuffix(".svg"),
        colorFilter = ColorFilter.tint(tint),
        modifier = modifier.size(size),
    )
}

@Composable
private fun Swatch(colour: Long, chosen: Boolean, onClick: () -> Unit) {
    val ring = if (chosen) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline
    Box(Modifier.size(24.dp).background(if (colour == NO_COLOUR) Color.Transparent else Color(colour), CircleShape).border(2.dp, ring, CircleShape).clickable(onClick = onClick))
}

@Composable
private fun IconPickerDialog(names: List<String>, chosen: String, onDismiss: () -> Unit, onPick: (String) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Select icon") },
        text = {
            LazyVerticalGrid(columns = GridCells.Adaptive(44.dp), modifier = Modifier.height(360.dp)) {
                items(names) { name ->
                    val ring = if (name == chosen) MaterialTheme.colorScheme.primary else Color.Transparent
                    Box(Modifier.padding(4.dp).border(2.dp, ring).clickable { onPick(name) }.padding(6.dp)) {
                        ValueIcon(name, MaterialTheme.colorScheme.onSurface, 24.dp)
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun IconButtonFor(name: String, onClick: () -> Unit) {
    Box(Modifier.border(1.dp, MaterialTheme.colorScheme.outline).clickable(onClick = onClick).padding(6.dp)) {
        ValueIcon(name, MaterialTheme.colorScheme.onSurface, 24.dp)
    }
}

@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
internal fun ValueDisplayDialog(label: String, initial: ValueDisplay, onDismiss: () -> Unit, extra: @Composable () -> Unit = {}, onDone: (ValueDisplay) -> Unit) {
    val context = LocalContext.current
    val names = remember { iconNames(context) }
    val firstIcon = names.firstOrNull().orEmpty()
    var display by remember { mutableStateOf(initial) }
    var picking by remember { mutableStateOf<((String) -> Unit)?>(null) }
    var pickingCurrent by remember { mutableStateOf("") }
    val pick: (String, (String) -> Unit) -> Unit = { current, chosen ->
        pickingCurrent = current
        picking = chosen
    }

    picking?.let { chosen ->
        IconPickerDialog(names, pickingCurrent, onDismiss = { picking = null }) { name ->
            chosen(name)
            picking = null
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Telemetry display") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(label, style = MaterialTheme.typography.labelMedium)
                extra()
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    FilterChip(
                        selected = display.showIcon,
                        onClick = { display = display.copy(showIcon = true, icon = display.icon.ifBlank { firstIcon }) },
                        label = { Text("Icon") },
                    )
                    FilterChip(selected = !display.showIcon, onClick = { display = display.copy(showIcon = false) }, label = { Text("Text") })
                    if (display.showIcon) IconButtonFor(display.icon) { pick(display.icon) { display = display.copy(icon = it) } }
                }
                if (!display.showIcon) {
                    OutlinedTextField(value = display.text, onValueChange = { display = display.copy(text = it) }, label = { Text("Text") }, singleLine = true)
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Show units", modifier = Modifier.weight(1f))
                    Switch(checked = display.showUnits, onCheckedChange = { display = display.copy(showUnits = it) })
                }
                Text("Value range", style = MaterialTheme.typography.titleSmall)
                Text("Change the color, opacity or icon when the value crosses a threshold", style = MaterialTheme.typography.bodySmall)
                androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    RangeType.entries.forEach { type ->
                        FilterChip(
                            selected = display.rangeType == type,
                            onClick = { if (display.rangeType != type) display = withRangeType(display, type, firstIcon) },
                            label = { Text(type.label) },
                        )
                    }
                }
                if (display.rangeType != RangeType.None) {
                    Text(rangeHelp(display.rangeType), style = MaterialTheme.typography.bodySmall)
                    (0..display.values.size).forEach { row ->
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
                            RangeCell(display, row, onChange = { display = it }, onPick = pick)
                            if (row < display.values.size && display.values.size > 1) TextButton(onClick = { display = withoutRow(display, row) }) { Text("✕") }
                        }
                    }
                    TextButton(onClick = { display = withRow(display, firstIcon) }) { Text("Add row") }
                }
            }
        },
        confirmButton = { TextButton(onClick = { onDone(display) }) { Text("Done") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

internal fun rangeHelp(type: RangeType): String = when (type) {
    RangeType.Color -> "Specify the color you want to apply based on value ranges. The color will be applied to the icon if available, otherwise to the value itself."
    RangeType.Opacity -> "Specify the icon opacity you want based on value ranges."
    RangeType.Icon -> "Specify the icon you want to display based on value ranges."
    RangeType.None -> ""
}

@Composable
private fun RangeCell(display: ValueDisplay, row: Int, onChange: (ValueDisplay) -> Unit, onPick: (String, (String) -> Unit) -> Unit) {
    when (display.rangeType) {
        RangeType.Color -> RANGE_COLOURS.forEach { colour ->
            Swatch(colour, display.colours.getOrNull(row) == colour) {
                onChange(display.copy(colours = display.colours.mapIndexed { i, old -> if (i == row) colour else old }))
            }
        }
        RangeType.Opacity -> OutlinedTextField(
            value = display.opacities.getOrNull(row)?.toString().orEmpty(),
            onValueChange = { typed -> typed.toDoubleOrNull()?.let { v -> onChange(display.copy(opacities = display.opacities.mapIndexed { i, old -> if (i == row) v else old })) } },
            label = { Text("Opacity") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            modifier = Modifier.width(96.dp),
        )
        RangeType.Icon -> display.icons.getOrNull(row)?.let { current ->
            IconButtonFor(current) { onPick(current) { name -> onChange(display.copy(icons = display.icons.mapIndexed { i, old -> if (i == row) name else old })) } }
        }
        RangeType.None -> Unit
    }
}

@Composable
internal fun ValueLabel(display: ValueDisplay, raw: Double?, label: String, fallback: Color) {
    val tint = displayColour(display, raw)?.let { Color(it) } ?: fallback
    val opacity = displayOpacity(display, raw)
    when (val icon = displayIcon(display, raw)) {
        null -> if (!iconShown(display)) Text(display.text.ifBlank { label }, style = MaterialTheme.typography.labelSmall, color = tint, modifier = Modifier.alpha(opacity))
        else -> ValueIcon(icon, tint, 16.dp, Modifier.alpha(opacity))
    }
}
