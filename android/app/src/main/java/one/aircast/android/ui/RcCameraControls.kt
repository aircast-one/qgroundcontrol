package one.aircast.android.ui

import android.os.SystemClock
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcValue
import one.aircast.android.bridge.settingControl

private const val FLY_VIEW_SETTINGS = "settings.flyViewSettings"
private val RAIL_SLIDER_LENGTH = 140.dp
private val RAIL_SLIDER_THICKNESS = 40.dp

private fun Modifier.railSlider(): Modifier = this
    .width(RAIL_SLIDER_THICKNESS)
    .height(RAIL_SLIDER_LENGTH)
    .graphicsLayer {
        rotationZ = 270f
        transformOrigin = TransformOrigin(0f, 0f)
    }
    .layout { measurable, constraints ->
        val placeable = measurable.measure(
            Constraints(minWidth = constraints.minHeight, maxWidth = constraints.maxHeight, minHeight = constraints.minWidth, maxHeight = constraints.maxWidth),
        )
        layout(placeable.height, placeable.width) { placeable.place(-placeable.width, 0) }
    }
private const val GIMBAL_VIEW = "view.gimbalIndicator"

internal data class RcCameraChannels(val tilt: Int, val pan: Int, val zoom: Int, val light: Int, val record: Int) {
    val any: Boolean get() = listOf(tilt, pan, zoom, light, record).any { it > 0 }
}

internal fun rcCameraChannels(tilt: Int, pan: Int, zoom: Int, light: Int, record: Int): RcCameraChannels =
    RcCameraChannels(tilt = tilt, pan = if (pan == tilt) 0 else pan, zoom = zoom, light = light, record = record)

internal fun cameraRecording(recordChannel: Int, channelRecording: Boolean, streamRecording: Boolean): Boolean =
    streamRecording || (recordChannel > 0 && channelRecording)

private fun send(channel: Int, pwm: Int) = cameraRcControls.hold(channel, pwm)

@Composable
private fun channelSetting(name: String): Int {
    val value by qgcValue(settingControl("$FLY_VIEW_SETTINGS.$name"))
    return (value as? Number)?.toInt() ?: 0
}

@Composable
private fun PwmSlider(label: String, channel: Int, pwm: Int, onPwm: (Int) -> Unit) {
    var lastSent by remember(channel) { mutableLongStateOf(0L) }
    val latest by rememberUpdatedState(pwm)
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Slider(
            value = pwm.toFloat(),
            onValueChange = { raw ->
                val next = raw.toInt()
                onPwm(next)
                val now = SystemClock.uptimeMillis()
                if (rcSendDue(now, lastSent, finished = false)) {
                    lastSent = now
                    send(channel, next)
                }
            },
            onValueChangeFinished = { send(channel, latest) },
            valueRange = PWM_MIN.toFloat()..PWM_MAX.toFloat(),
            modifier = Modifier.railSlider(),
        )
        Text(label, style = MaterialTheme.typography.labelMedium)
    }
}

private val gimbalSends = java.util.concurrent.Executors.newSingleThreadExecutor()

private fun sendTilt(pitch: Float) {
    gimbalSends.execute { gimbalRefusal(Qgc.call("gimbal.pitch", pitch.toDouble())) }
}

private const val GIMBAL_REFUSAL_MS = 4000L
internal const val GIMBAL_TILT_MIN = -90f
internal const val GIMBAL_TILT_MAX = 30f

@Composable
private fun GimbalTiltSlider(pitch: Double?) {
    var dragging by remember { mutableStateOf<Float?>(null) }
    var lastSent by remember { mutableLongStateOf(0L) }
    val shown = dragging ?: (pitch?.toFloat() ?: 0f).coerceIn(GIMBAL_TILT_MIN, GIMBAL_TILT_MAX)
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text("${kotlin.math.round(shown).toInt()}\u00b0", style = MaterialTheme.typography.labelMedium)
        Slider(
            value = shown,
            onValueChange = { next ->
                dragging = next
                val now = SystemClock.uptimeMillis()
                if (rcSendDue(now, lastSent, finished = false)) {
                    lastSent = now
                    sendTilt(next)
                }
            },
            onValueChangeFinished = {
                dragging?.let(::sendTilt)
                dragging = null
            },
            valueRange = GIMBAL_TILT_MIN..GIMBAL_TILT_MAX,
            modifier = Modifier.railSlider().semantics { contentDescription = "Gimbal tilt" },
        )
    }
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun RcCameraControls(modifier: Modifier = Modifier) {
    val channels = rcCameraChannels(
        tilt = channelSetting("gimbalTiltChannel"),
        pan = channelSetting("gimbalPanChannel"),
        zoom = channelSetting("cameraZoomChannel"),
        light = channelSetting("cameraLightChannel"),
        record = channelSetting("cameraRecordChannel"),
    )
    val gimbalJson by qgcPath(GIMBAL_VIEW)
    val gimbalManager = gimbalJson?.optBoolean("shown") == true
    val vehiclesJson by qgcPath(one.aircast.map.VEHICLES_VIEW)
    val vehicleId = activeVehicleId(vehiclesJson)
    val streamRecording by qgcBool("video.recording")
    var tilt by remember(vehicleId) { mutableIntStateOf(PWM_CENTER) }
    var pan by remember(vehicleId) { mutableIntStateOf(PWM_CENTER) }
    var zoom by remember(vehicleId) { mutableIntStateOf(PWM_CENTER) }
    var lightOn by remember(vehicleId) { mutableStateOf(false) }
    var channelRecording by remember(vehicleId) { mutableStateOf(false) }
    DisposableEffect(vehicleId) { onDispose { cameraRcControls.release() } }
    if (!hasVehicle() || (!channels.any && !gimbalManager)) return
    val gimbal = remember(gimbalJson) { gimbalIndicator(gimbalJson) }
    var gimbalRefused by remember { mutableStateOf<String?>(null) }
    var options by remember { mutableStateOf(false) }
    LaunchedEffect(gimbalRefused) {
        if (gimbalRefused != null) {
            kotlinx.coroutines.delay(GIMBAL_REFUSAL_MS)
            gimbalRefused = null
        }
    }
    val recording = cameraRecording(channels.record, channelRecording, streamRecording)
    val rcGimbal = !gimbalManager && (channels.tilt > 0 || channels.pan > 0)
    Column(
        modifier.padding(horizontal = 4.dp, vertical = 4.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            if (gimbal != null) GimbalTiltSlider(gimbal.pitchDegrees)
            if (!gimbalManager && channels.tilt > 0) PwmSlider("Tilt", channels.tilt, tilt) { tilt = it }
            if (!gimbalManager && channels.pan > 0) PwmSlider("Pan", channels.pan, pan) { pan = it }
            if (channels.zoom > 0) PwmSlider("Zoom", channels.zoom, zoom) { zoom = it }
        }
        if (channels.record > 0) {
            FilterChip(selected = recording, onClick = {
                val next = !recording
                channelRecording = next
                send(channels.record, if (next) PWM_MAX else PWM_MIN)
                offMainInOrder { Qgc.invoke(if (next) "video.startRecording" else "video.stopRecording") }
            }, label = { Text("Record") })
        }
        if (channels.light > 0 || gimbal != null || rcGimbal) {
            IconButton(onClick = { options = true }, modifier = Modifier.size(48.dp)) {
                Icon(painterResource(R.drawable.ic_tune), "Gimbal options", Modifier.size(22.dp))
            }
        }
        if (options) {
            ModalBottomSheet(onDismissRequest = { options = false }) {
                Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Gimbal", style = MaterialTheme.typography.titleLarge)
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (channels.light > 0) {
                            FilterChip(selected = lightOn, onClick = {
                                lightOn = !lightOn
                                send(channels.light, if (lightOn) PWM_MAX else PWM_MIN)
                            }, label = { Text("Light") })
                        }
                        if (gimbal != null && gimbal.yawLockOffered) {
                            FilterChip(selected = gimbal.yawLocked, onClick = {
                                offMainDetached { gimbalRefused = gimbalRefusal(Qgc.call("gimbal.yawLock", !gimbal.yawLocked)) }
                            }, label = { Text(gimbal.yawLockLabel) })
                        }
                        if (gimbal != null) {
                            TextButton(onClick = { offMainDetached { gimbalRefused = gimbalRefusal(Qgc.call("gimbal.center")) } }) { Text("Recenter") }
                        }
                        if (rcGimbal) {
                            TextButton(onClick = {
                                tilt = PWM_CENTER
                                pan = PWM_CENTER
                                send(channels.tilt, PWM_CENTER)
                                send(channels.pan, PWM_CENTER)
                            }) { Text("Recenter") }
                        }
                    }
                    gimbalRefused?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
                }
            }
        }
        gimbalRefused?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.error) }
    }
}
