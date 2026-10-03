package one.aircast.android.ui

import android.os.SystemClock
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
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
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.labelMedium, modifier = Modifier.width(64.dp))
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
            modifier = Modifier.width(180.dp),
        )
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
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text("${kotlin.math.round(shown).toInt()}\u00b0", style = MaterialTheme.typography.labelMedium, modifier = Modifier.width(64.dp))
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
            modifier = Modifier.width(180.dp),
        )
    }
}

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
    val vehiclesJson by qgcPath(one.aircast.mapspike.VEHICLES_VIEW)
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
    LaunchedEffect(gimbalRefused) {
        if (gimbalRefused != null) {
            kotlinx.coroutines.delay(GIMBAL_REFUSAL_MS)
            gimbalRefused = null
        }
    }
    val recording = cameraRecording(channels.record, channelRecording, streamRecording)
    val rcGimbal = !gimbalManager && (channels.tilt > 0 || channels.pan > 0)
    Column(modifier.padding(horizontal = 8.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        if (gimbal != null) GimbalTiltSlider(gimbal.pitchDegrees)
        if (!gimbalManager && channels.tilt > 0) PwmSlider("Tilt", channels.tilt, tilt) { tilt = it }
        if (!gimbalManager && channels.pan > 0) PwmSlider("Pan", channels.pan, pan) { pan = it }
        if (channels.zoom > 0) PwmSlider("Zoom", channels.zoom, zoom) { zoom = it }
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
            if (channels.record > 0) {
                FilterChip(selected = recording, onClick = {
                    val next = !recording
                    channelRecording = next
                    send(channels.record, if (next) PWM_MAX else PWM_MIN)
                    offMainInOrder { Qgc.invoke(if (next) "video.startRecording" else "video.stopRecording") }
                }, label = { Text("Record") })
            }
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
        gimbalRefused?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.error) }
    }
}
