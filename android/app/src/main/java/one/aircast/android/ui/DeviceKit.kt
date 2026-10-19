package one.aircast.android.ui

import android.content.Intent
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.annotation.DrawableRes
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal data class DeviceBadge(val level: Int, val text: String)

internal data class DeviceOption(val value: String, val label: String)

internal data class DeviceChartView(val label: String, val values: List<Double>, val max: Int, val value: String, val level: Int)

internal data class DeviceRequest(val method: DeviceMethod, val path: String, val body: JSONObject? = null)

internal fun JSONArray?.deviceObjects(): List<JSONObject> = this?.let { array -> (0 until array.length()).mapNotNull(array::optJSONObject) }.orEmpty()

internal fun JSONObject?.deviceObjects(key: String): List<JSONObject> = this?.optJSONArray(key).deviceObjects()

internal fun JSONObject?.deviceTexts(key: String): List<String> =
    this?.optJSONArray(key)?.let { array -> (0 until array.length()).map(array::optText) }.orEmpty()

internal fun parseBadge(json: JSONObject?): DeviceBadge = DeviceBadge(json?.optInt("level") ?: 0, json?.optText("text").orEmpty())

internal fun parseBadges(json: JSONObject?, key: String): List<DeviceBadge> = json.deviceObjects(key).map(::parseBadge)

internal fun parseOptions(json: JSONObject?, key: String): List<DeviceOption> =
    json.deviceObjects(key).map { DeviceOption(it.optText("value"), it.optText("label")) }

internal fun parseChart(json: JSONObject?): DeviceChartView? = json?.let {
    DeviceChartView(
        label = it.optText("label"),
        values = it.optJSONArray("values")?.let { values -> (0 until values.length()).mapNotNull { index -> (values.opt(index) as? Number)?.toDouble() } }.orEmpty(),
        max = it.optInt("max", 100),
        value = it.optText("value"),
        level = it.optInt("level", DeviceLevel.Neutral),
    )
}

@Composable
internal fun deviceLevelColor(level: Int): Color = when (level) {
    DeviceLevel.Ready -> MaterialTheme.aircast.success
    DeviceLevel.Warn -> MaterialTheme.aircast.warning
    DeviceLevel.Blocked -> MaterialTheme.aircast.alert
    else -> MaterialTheme.colorScheme.onSurfaceVariant
}

@Composable
private fun levelTextColor(level: Int): Color = if (level == DeviceLevel.Neutral) MaterialTheme.colorScheme.onSurface else deviceLevelColor(level)

@DrawableRes
private fun levelIcon(level: Int): Int = when (level) {
    DeviceLevel.Ready -> R.drawable.ic_check_circle
    DeviceLevel.Neutral -> R.drawable.ic_info
    else -> R.drawable.ic_warning
}

private val LIST_PADDING = 16.dp

@Composable
internal fun DeviceSection(title: String, badge: DeviceBadge? = null, content: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth().semantics { contentDescription = title }) {
        SectionHeader(title, trailing = badge?.let { { DeviceBadgeText(it) } })
        content()
    }
}

@Composable
internal fun DeviceBadgeText(badge: DeviceBadge) {
    if (badge.text.isEmpty()) return
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Box(Modifier.size(8.dp).background(deviceLevelColor(badge.level), CircleShape))
        Text(badge.text, style = MaterialTheme.typography.labelMedium, color = deviceLevelColor(badge.level))
    }
}

@Composable
internal fun DeviceNotice(notice: DeviceBadge) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp).background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(painterResource(levelIcon(notice.level)), null, tint = deviceLevelColor(notice.level), modifier = Modifier.size(20.dp))
        Text(notice.text, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
    }
}

@Composable
internal fun DeviceText(text: String) {
    if (text.isEmpty()) return
    Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = LIST_PADDING, vertical = 8.dp))
}

@Composable
internal fun DeviceFact(label: String, value: String, level: Int = DeviceLevel.Neutral) {
    Row(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyLarge, color = levelTextColor(level), textAlign = TextAlign.End, modifier = Modifier.weight(1f))
    }
}

@Composable
internal fun rememberCopy(): (String) -> Unit {
    val clipboard = LocalClipboardManager.current
    val notice = LocalChangeNotice.current
    return remember(clipboard, notice) {
        { text ->
            clipboard.setText(AnnotatedString(text))
            notice.show("Copied", null)
        }
    }
}

@Composable
internal fun CopyRow(label: String, text: String, hint: String = "", qr: Boolean = false, level: Int = DeviceLevel.Neutral) {
    val copy = rememberCopy()
    var showingQr by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium, color = if (level == DeviceLevel.Neutral) MaterialTheme.colorScheme.onSurfaceVariant else deviceLevelColor(level))
            Text(text, style = MaterialTheme.typography.bodyLarge, fontFamily = FontFamily.Monospace)
            if (hint.isNotEmpty()) Text(hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (qr) {
            IconButton(onClick = { showingQr = true }) { Icon(painterResource(R.drawable.ic_qr_code_2), "Show $label as a QR code") }
        }
        IconButton(onClick = { copy(text) }) { Icon(painterResource(R.drawable.ic_content_copy), "Copy $label") }
    }
    if (showingQr) QrDialog(label, text) { showingQr = false }
}

@Composable
internal fun QrDialog(title: String, text: String, onDismiss: () -> Unit) {
    val image by produceState<android.graphics.Bitmap?>(null, text) {
        value = deviceQr(text)?.let { BitmapFactory.decodeByteArray(it, 0, it.size) }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Box(Modifier.size(256.dp).background(Color.White, RoundedCornerShape(8.dp)).padding(8.dp), contentAlignment = Alignment.Center) {
                    image?.let { Image(it.asImageBitmap(), "QR code for $text", Modifier.size(240.dp)) }
                }
                Text(text, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, textAlign = TextAlign.Center, modifier = Modifier.widthIn(max = 256.dp))
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Done") } },
    )
}

internal class DeviceActionState(private val notice: ChangeNotice) {
    var busy by mutableStateOf(false)
        private set
    var refusal by mutableStateOf<DeviceRefusal?>(null)

    suspend fun run(request: DeviceRequest, success: String = ""): Boolean {
        busy = true
        refusal = null
        val refused = deviceCall(request.method, request.path, request.body)
        busy = false
        refusal = refused
        if (refused == null && success.isNotEmpty()) notice.show(success, null)
        return refused == null
    }
}

@Composable
internal fun rememberDeviceAction(): DeviceActionState {
    val notice = LocalChangeNotice.current
    return remember(notice) { DeviceActionState(notice) }
}

@Composable
internal fun RefusalText(refusal: DeviceRefusal?, field: String = "") {
    if (refusal == null || refusal.field != field) return
    Text(refusal.message, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = LIST_PADDING, vertical = 4.dp))
}

internal data class ActionConfirm(val title: String, val text: String, val confirm: String)

@Composable
internal fun ConfirmDialog(confirm: ActionConfirm, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(confirm.title) },
        text = { Text(confirm.text) },
        confirmButton = {
            TextButton(onClick = {
                onDismiss()
                onConfirm()
            }) { Text(confirm.confirm) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
internal fun ActionButton(label: String, request: DeviceRequest, success: String = "", confirm: ActionConfirm? = null, tonal: Boolean = true, onDone: () -> Unit = {}) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var asking by remember { mutableStateOf(false) }
    val go = { scope.launch { if (action.run(request, success)) onDone() } }
    val press = { if (confirm == null) go() else asking = true }
    Column {
        if (tonal) {
            FilledTonalButton(onClick = { press() }, enabled = !action.busy) { Text(label) }
        } else {
            TextButton(onClick = { press() }, enabled = !action.busy) { Text(label) }
        }
        RefusalText(action.refusal)
    }
    if (asking && confirm != null) ConfirmDialog(confirm, { go() }) { asking = false }
}

@Composable
internal fun DeviceSwitchRow(
    title: String,
    checked: Boolean,
    on: DeviceRequest,
    off: DeviceRequest,
    detail: String = "",
    success: String = "",
    offSuccess: String = success,
    enabled: Boolean = true,
    confirmOn: ActionConfirm? = null,
) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var pending by remember { mutableStateOf<Boolean?>(null) }
    var asking by remember { mutableStateOf(false) }
    val flip = { next: Boolean ->
        pending = next
        scope.launch {
            action.run(if (next) on else off, if (next) success else offSuccess)
            pending = null
        }
    }
    Row(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyLarge)
            if (detail.isNotEmpty()) Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Switch(
            checked = pending ?: checked,
            enabled = enabled && !action.busy,
            onCheckedChange = { next -> if (next && confirmOn != null) asking = true else flip(next) },
            modifier = Modifier.semantics { contentDescription = title },
        )
    }
    RefusalText(action.refusal)
    if (asking && confirmOn != null) ConfirmDialog(confirmOn, { flip(true) }) { asking = false }
}

@Composable
internal fun SecretField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    isError: Boolean = false,
    supportingText: String = "",
) {
    var revealed by rememberSaveable { mutableStateOf(false) }
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        placeholder = placeholder.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
        singleLine = true,
        isError = isError,
        supportingText = supportingText.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
        visualTransformation = if (revealed) VisualTransformation.None else PasswordVisualTransformation(),
        keyboardOptions = KeyboardOptions(keyboardType = if (revealed) KeyboardType.Text else KeyboardType.Password, autoCorrectEnabled = false),
        leadingIcon = { TextButton(onClick = { revealed = !revealed }) { Text(if (revealed) "Hide" else "Show") } },
        modifier = modifier.fillMaxWidth(),
    )
}

internal enum class LinkLook { Filled, Tonal, Outlined, Text }

@Composable
internal fun LinkButton(href: String, text: String, look: LinkLook = LinkLook.Tonal, external: Boolean = false, @DrawableRes icon: Int? = null) {
    val context = LocalContext.current
    val open = { runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(href)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) } }
    val content: @Composable () -> Unit = {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            icon?.let { Icon(painterResource(it), null, Modifier.size(18.dp)) }
            Text(text)
            if (external) Icon(painterResource(R.drawable.ic_open_in_new), null, Modifier.size(18.dp))
        }
    }
    when (look) {
        LinkLook.Filled -> Button(onClick = { open() }) { content() }
        LinkLook.Tonal -> FilledTonalButton(onClick = { open() }) { content() }
        LinkLook.Outlined -> OutlinedButton(onClick = { open() }) { content() }
        LinkLook.Text -> TextButton(onClick = { open() }) { content() }
    }
}

@Composable
internal fun DeviceSelect(label: String, value: String, options: List<DeviceOption>, onPick: (String) -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    ChoiceField(label, options.firstOrNull { it.value == value }?.label ?: value, options.map { it.label }, modifier.fillMaxWidth(), enabled = enabled) { index ->
        options.getOrNull(index)?.let { onPick(it.value) }
    }
}

internal data class DeviceField(
    val name: String,
    val label: String,
    val value: String,
    val placeholder: String = "",
    val hint: String = "",
    val keyboard: KeyboardType = KeyboardType.Text,
    val options: List<DeviceOption>? = null,
    val lines: Int = 1,
    val shownWhen: ((Map<String, String>) -> Boolean)? = null,
)

internal fun formBody(fields: List<DeviceField>, values: Map<String, String>): Map<String, String> =
    fields.associate { it.name to (values[it.name] ?: it.value) }

internal fun shownFields(fields: List<DeviceField>, values: Map<String, String>): List<DeviceField> =
    fields.filter { it.shownWhen?.invoke(values) ?: true }

@Composable
private fun FieldInput(field: DeviceField, value: String, refusal: DeviceRefusal?, onValueChange: (String) -> Unit) {
    val refused = refusal?.field == field.name
    val supporting = if (refused) refusal?.message.orEmpty() else field.hint
    when {
        field.options != null -> DeviceSelect(field.label, value, field.options, onValueChange, Modifier.fillMaxWidth())
        field.keyboard == KeyboardType.Password -> SecretField(field.label, value, onValueChange, placeholder = field.placeholder, isError = refused, supportingText = supporting)
        else -> OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            label = { Text(field.label) },
            placeholder = field.placeholder.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
            singleLine = field.lines == 1,
            minLines = field.lines,
            isError = refused,
            supportingText = supporting.takeIf { it.isNotEmpty() }?.let { { Text(it) } },
            keyboardOptions = KeyboardOptions(keyboardType = field.keyboard, autoCorrectEnabled = false),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
internal fun DeviceForm(
    fields: List<DeviceField>,
    method: DeviceMethod,
    path: String,
    submit: String,
    success: String = "",
    confirm: ActionConfirm? = null,
    note: String = "",
    extra: Map<String, String> = emptyMap(),
    onDone: () -> Unit = {},
) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    var values by remember { mutableStateOf(emptyMap<String, String>()) }
    var asking by remember { mutableStateOf(false) }
    val current = formBody(fields, values)
    val shown = shownFields(fields, current)
    val send = {
        scope.launch {
            if (action.run(DeviceRequest(method, path, JSONObject(current + extra)), success)) {
                values = emptyMap()
                onDone()
            }
        }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        shown.map { field ->
            FieldInput(field, current[field.name].orEmpty(), action.refusal) { values = values + (field.name to it) }
        }
        if (note.isNotEmpty()) Text(note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        action.refusal?.takeIf { refusal -> shown.none { it.name == refusal.field } }?.let {
            Text(it.message, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
        FilledTonalButton(onClick = { if (confirm == null) send() else asking = true }, enabled = !action.busy) { Text(submit) }
    }
    if (asking && confirm != null) ConfirmDialog(confirm, { send() }) { asking = false }
}

internal fun chartPoints(values: List<Double>, max: Int, width: Float, height: Float): List<Offset> =
    if (values.size < 2) {
        emptyList()
    } else {
        values.mapIndexed { index, value ->
            Offset(index.toFloat() / (values.size - 1) * width, height - (value.coerceIn(0.0, max.toDouble()) / (if (max > 0) max else 1)).toFloat() * height)
        }
    }

@Composable
internal fun DeviceChart(chart: DeviceChartView) {
    val stroke = if (chart.level == DeviceLevel.Neutral) MaterialTheme.colorScheme.primary else deviceLevelColor(chart.level)
    Row(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Column(Modifier.width(112.dp)) {
            Text(chart.label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(chart.value, style = MaterialTheme.typography.titleMedium, color = levelTextColor(chart.level))
        }
        Canvas(Modifier.weight(1f).height(40.dp).semantics { contentDescription = "${chart.label} history" }) {
            val points = chartPoints(chart.values, chart.max, size.width, size.height)
            if (points.isNotEmpty()) {
                val line = Path().apply {
                    moveTo(points.first().x, points.first().y)
                    points.drop(1).map { lineTo(it.x, it.y) }
                }
                drawPath(line, stroke, style = Stroke(width = 2.dp.toPx()))
            }
        }
    }
}

@Composable
internal fun DeviceDisclosure(title: String, summary: String = "", badge: DeviceBadge? = null, initiallyOpen: Boolean = false, content: @Composable () -> Unit) {
    var open by rememberSaveable(title) { mutableStateOf(initiallyOpen) }
    Column(Modifier.fillMaxWidth()) {
        Row(
            Modifier.fillMaxWidth().clickable { open = !open }.heightIn(min = 56.dp).padding(horizontal = LIST_PADDING, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.bodyLarge)
                if (summary.isNotEmpty()) Text(summary, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            badge?.let { DeviceBadgeText(it) }
            Icon(if (open) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown, if (open) "Collapse $title" else "Expand $title", tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (open) content()
    }
}

@Composable
internal fun TypedConfirmDialog(title: String, text: String, label: String, expected: String, confirm: String, onConfirm: (String) -> Unit, onDismiss: () -> Unit) {
    var typed by remember { mutableStateOf("") }
    val matches = typed.trim() == expected && expected.isNotEmpty()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                Text(text)
                OutlinedTextField(value = typed, onValueChange = { typed = it }, label = { Text(label) }, placeholder = { Text(expected) }, singleLine = true, keyboardOptions = KeyboardOptions(autoCorrectEnabled = false))
            }
        },
        confirmButton = {
            TextButton(enabled = matches, onClick = {
                onDismiss()
                onConfirm(typed.trim())
            }) { Text(confirm) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
internal fun StepText(step: Int, title: String, text: String = "") {
    Column(Modifier.fillMaxWidth().padding(start = LIST_PADDING, end = LIST_PADDING, top = 12.dp, bottom = 4.dp)) {
        Text("$step · $title", style = MaterialTheme.typography.titleSmall)
        if (text.isNotEmpty()) Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
internal fun StatusLine(text: String, level: Int = DeviceLevel.Neutral) {
    if (text.isEmpty()) return
    Text(text, style = MaterialTheme.typography.bodyMedium, color = deviceLevelColor(level), modifier = Modifier.padding(horizontal = LIST_PADDING, vertical = 8.dp))
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ButtonRow(content: @Composable () -> Unit) {
    FlowRow(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        content()
    }
}

@Composable
internal fun StackedFact(label: String, value: String) {
    Column(Modifier.fillMaxWidth().padding(horizontal = LIST_PADDING, vertical = 8.dp)) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
internal fun Fold(title: String, open: Boolean, onToggle: (Boolean) -> Unit, summary: String = "", content: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().clickable { onToggle(!open) }.heightIn(min = 48.dp).padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                if (summary.isNotEmpty()) Text(summary, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Icon(if (open) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown, if (open) "Collapse $title" else "Expand $title", tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (open) Column(Modifier.fillMaxWidth().padding(bottom = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) { content() }
    }
}
