package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.size
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val APM_FOLLOW_VIEW = "view.apmFollow"
internal const val APM_FOLLOW_SCREEN = "apmFollow"
internal const val APM_FOLLOW_ENABLE = "apmFollow.enable"
internal const val APM_FOLLOW_RESET = "apmFollow.reset"
internal const val APM_FOLLOW_POSITION = "apmFollow.position"
internal const val APM_FOLLOW_POINT = "apmFollow.point"
internal const val APM_FOLLOW_OFFSETS = "apmFollow.offsets"
internal const val APM_FOLLOW_HEIGHT = "apmFollow.height"
private const val WAITING_POLL_MS = 500L

internal data class ApmFollow(
    val available: Boolean,
    val enabled: Boolean,
    val waiting: Boolean,
    val supported: Boolean,
    val unsupportedText: String,
    val showSettings: Boolean,
    val rover: Boolean,
    val positionOptions: List<String>,
    val positionIndex: Int,
    val pointOptions: List<String>,
    val pointIndex: Int,
    val angle: Double,
    val distance: Double,
    val height: Double,
)

private fun JSONObject.strings(key: String): List<String> =
    optJSONArray(key)?.let { array -> (0 until array.length()).map { array.optString(it) } }.orEmpty()

internal fun apmFollow(view: JSONObject?): ApmFollow? = view?.takeIf { it.optBoolean("available") }?.let {
    ApmFollow(
        available = true,
        enabled = it.optBoolean("enabled"),
        waiting = it.optBoolean("waiting"),
        supported = it.optBoolean("supported", true),
        unsupportedText = it.optText("unsupportedText"),
        showSettings = it.optBoolean("showSettings"),
        rover = it.optBoolean("rover"),
        positionOptions = it.strings("positionOptions"),
        positionIndex = it.optInt("positionIndex"),
        pointOptions = it.strings("pointOptions"),
        pointIndex = it.optInt("pointIndex", -1),
        angle = it.optDouble("angle"),
        distance = it.optDouble("distance"),
        height = it.optDouble("height"),
    )
}

internal fun oneDecimal(value: Double): String = "%.1f".format(java.util.Locale.ROOT, value)

@Composable
fun ApmFollowScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<ApmFollow?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { apmFollow(Qgc.get(APM_FOLLOW_VIEW)) }
        loaded = true
    }
    LaunchedEffect(read?.waiting) {
        while (read?.waiting == true) {
            kotlinx.coroutines.delay(WAITING_POLL_MS)
            revision++
        }
    }
    if (!loaded) return
    val follow = read ?: run {
        Text("This vehicle has no follow me parameters.", modifier.padding(16.dp))
        return
    }

    fun act(path: String, vararg args: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.IO) { Qgc.refusalOf(path, *args) }
            revision++
        }
    }

    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Checkbox(checked = follow.enabled, onCheckedChange = { act(APM_FOLLOW_ENABLE, it) })
            Text("Enable Follow Me")
        }
        if (follow.waiting) Text("Waiting for Vehicle to update", style = MaterialTheme.typography.bodyMedium)
        if (!follow.supported) {
            Text(follow.unsupportedText, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(vertical = 8.dp))
            Button(onClick = { act(APM_FOLLOW_RESET) }) { Text("Reset To Supported Settings") }
        }
        if (follow.showSettings) {
            SectionHeader("Follow Me Settings")
            Choice("Vehicle Position", follow.positionOptions, follow.positionIndex) { act(APM_FOLLOW_POSITION, it) }
            if (!follow.rover) Choice("Point Vehicle", follow.pointOptions, follow.pointIndex) { act(APM_FOLLOW_POINT, it) }
            if (follow.positionIndex == 1) {
                Text("Vehicle Offsets", style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 12.dp))
                androidx.compose.foundation.layout.Row {
                    OffsetGraphic(follow) { act(APM_FOLLOW_OFFSETS, it, follow.distance) }
                    if (!follow.rover) HeightGraphic(follow.height)
                }
                NumberEntry("Angle", "deg", follow.angle) { act(APM_FOLLOW_OFFSETS, it, follow.distance) }
                NumberEntry("Distance", "m", follow.distance) { act(APM_FOLLOW_OFFSETS, follow.angle, it) }
                if (!follow.rover) NumberEntry("Height", "m", follow.height) { act(APM_FOLLOW_HEIGHT, it) }
            }
        }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(top = 8.dp)) }
    }
}

@Composable
private fun Choice(label: String, options: List<String>, index: Int, onPick: (Int) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        Box {
            OutlinedButton(onClick = { open = true }) { Text(options.getOrElse(index) { "" }) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                options.forEachIndexed { at, option ->
                    DropdownMenuItem(text = { Text(option) }, onClick = {
                        open = false
                        onPick(at)
                    })
                }
            }
        }
    }
}

@Composable
private fun NumberEntry(label: String, units: String, value: Double, onDone: (Double) -> Unit) {
    var typed by remember(value) { mutableStateOf(oneDecimal(value)) }
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f))
        OutlinedTextField(
            value = typed,
            onValueChange = { typed = it },
            singleLine = true,
            suffix = { Text(units) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            keyboardActions = KeyboardActions(onDone = { typed.toDoubleOrNull()?.let(onDone) }),
            modifier = Modifier.weight(1f),
        )
    }
}

internal fun headingOfTap(x: Float, y: Float): Double {
    val geometric = Math.toDegrees(kotlin.math.atan2(y.toDouble(), x.toDouble()))
    return (90 - geometric).let { if (it < 0) it + 360 else if (it > 360) it - 360 else it }
}

internal fun vehicleYaw(follow: ApmFollow): Double = when {
    follow.rover || follow.pointIndex == 0 -> 0.0
    follow.pointIndex == 1 -> 180.0
    else -> -follow.angle
}

private val OFFSET_GRAPHIC = 240.dp

@Composable
private fun OffsetGraphic(follow: ApmFollow, onAngle: (Double) -> Unit) {
    val shade = MaterialTheme.colorScheme.outlineVariant
    val ink = MaterialTheme.colorScheme.onSurface
    val accent = MaterialTheme.colorScheme.primary
    Text("Click in the graphic to change angle", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    androidx.compose.foundation.Canvas(
        Modifier.size(OFFSET_GRAPHIC).pointerInput(Unit) {
            detectTapGestures { tap -> onAngle(headingOfTap(tap.x - size.width / 2f, size.height / 2f - tap.y)) }
        },
    ) {
        drawLine(shade, Offset(center.x, 0f), Offset(center.x, size.height), strokeWidth = 3.dp.toPx())
        drawLine(shade, Offset(0f, center.y), Offset(size.width, center.y), strokeWidth = 3.dp.toPx())
        val arrow = 14.dp.toPx()
        drawPath(Path().apply {
            moveTo(center.x, center.y - arrow)
            lineTo(center.x - arrow * 0.7f, center.y + arrow * 0.7f)
            lineTo(center.x + arrow * 0.7f, center.y + arrow * 0.7f)
            close()
        }, accent)
        rotate(follow.angle.toFloat()) {
            val at = Offset(center.x, 20.dp.toPx())
            drawLine(ink.copy(alpha = 0.4f), Offset(center.x, at.y + 14.dp.toPx()), Offset(center.x, center.y - arrow - 4.dp.toPx()), strokeWidth = 2.dp.toPx())
            rotate(vehicleYaw(follow).toFloat(), at) {
                drawPath(Path().apply {
                    moveTo(at.x, at.y - 12.dp.toPx())
                    lineTo(at.x - 9.dp.toPx(), at.y + 10.dp.toPx())
                    lineTo(at.x + 9.dp.toPx(), at.y + 10.dp.toPx())
                    close()
                }, ink)
            }
        }
    }
}

@Composable
private fun HeightGraphic(heightMetres: Double) {
    val ink = MaterialTheme.colorScheme.onSurface
    androidx.compose.foundation.layout.Box(Modifier.size(width = 56.dp, height = OFFSET_GRAPHIC), contentAlignment = androidx.compose.ui.Alignment.Center) {
        androidx.compose.foundation.Canvas(Modifier.size(width = 56.dp, height = OFFSET_GRAPHIC)) {
            val tick = 8.dp.toPx()
            drawLine(ink.copy(alpha = 0.4f), Offset(center.x, 0f), Offset(center.x, size.height), strokeWidth = 2.dp.toPx())
            drawLine(ink, Offset(center.x - tick, 1f), Offset(center.x + tick, 1f), strokeWidth = 2.dp.toPx())
            drawLine(ink, Offset(center.x - tick, size.height - 1f), Offset(center.x + tick, size.height - 1f), strokeWidth = 2.dp.toPx())
        }
        Text("${oneDecimal(heightMetres)} m", style = MaterialTheme.typography.labelSmall, modifier = Modifier.background(MaterialTheme.colorScheme.surface))
    }
}
