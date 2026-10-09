package one.aircast.android.ui

import android.content.Context
import android.os.Bundle
import android.os.VibrationEffect
import android.os.Vibrator
import android.speech.tts.TextToSpeech
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

private const val SPEECH_VIEW = "view.speech"
private const val SPEECH_POLL_MS = 500L
private val LOST_BUZZ = longArrayOf(0, 400, 150, 400, 150, 400)
private val REGAINED_BUZZ = longArrayOf(0, 120)

internal data class SpokenLine(val sequence: Long, val text: String, val volume: Float)

internal data class SpeechBatch(val last: Long, val muted: Boolean, val lines: List<SpokenLine>)

internal fun speechBatch(view: JSONObject?): SpeechBatch? = view?.takeIf { it.optText("class") == "Speech" }?.let { speech ->
    val lines = speech.optJSONArray("lines")
    SpeechBatch(
        last = speech.optLong("last"),
        muted = speech.optBoolean("muted"),
        lines = (0 until (lines?.length() ?: 0)).mapNotNull { lines?.optJSONObject(it) }.map {
            SpokenLine(it.optLong("sequence"), it.optText("text"), it.optDouble("volume", 1.0).toFloat())
        },
    )
}

internal fun speechPath(after: Long): String = "$SPEECH_VIEW($after)"

internal fun linkBuzz(lost: Boolean): LongArray = if (lost) LOST_BUZZ else REGAINED_BUZZ

internal data class ReturnAlert(val speak: Boolean, val alerted: Boolean)

internal fun returnAlert(alerted: Boolean, returnNow: Boolean, flying: Boolean): ReturnAlert =
    ReturnAlert(speak = returnNow && !alerted, alerted = flying && (alerted || returnNow))

private fun buzz(context: Context, pattern: LongArray) {
    context.getSystemService(Vibrator::class.java)?.takeIf { it.hasVibrator() }?.vibrate(VibrationEffect.createWaveform(pattern, -1))
}

@Composable
fun VoiceAlerts() {
    val context = LocalContext.current
    var engine by remember { mutableStateOf<TextToSpeech?>(null) }
    var ready by remember { mutableStateOf(false) }
    DisposableEffect(context) {
        val created = TextToSpeech(context.applicationContext) { status -> ready = status == TextToSpeech.SUCCESS }
        engine = created
        onDispose {
            ready = false
            created.shutdown()
        }
    }
    val voice = engine?.takeIf { ready }
    var muted by remember { mutableStateOf(false) }
    LaunchedEffect(voice) {
        val speaker = voice ?: return@LaunchedEffect
        var after = withContext(Dispatchers.Default) { speechBatch(Qgc.get(SPEECH_VIEW))?.last ?: 0L }
        while (currentCoroutineContext().isActive) {
            delay(SPEECH_POLL_MS)
            val batch = withContext(Dispatchers.Default) { speechBatch(Qgc.get(speechPath(after))) } ?: continue
            muted = batch.muted
            batch.lines.filter { it.sequence > after }.forEach { line ->
                speaker.speak(line.text, TextToSpeech.QUEUE_ADD, Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, line.volume) }, "qgc-${line.sequence}")
            }
            after = batch.last
        }
    }
    LinkLossBuzz(context)
    ReturnHomeAlert(context, voice?.takeUnless { muted })
}

@Composable
private fun ReturnHomeAlert(context: Context, voice: TextToSpeech?) {
    val batteryJson by qgcPath(BATTERY_VIEW)
    val flyJson by qgcPath(FLY_STATE)
    val returnNow = remember(batteryJson) { batteryHeadline(batteryJson)?.returnNow == true }
    val flying = flyJson?.optBoolean("flying") == true
    var alerted by remember { mutableStateOf(false) }
    LaunchedEffect(returnNow, flying) {
        val next = returnAlert(alerted, returnNow, flying)
        alerted = next.alerted
        if (!next.speak) return@LaunchedEffect
        voice?.speak(RETURN_NOW_SPOKEN, TextToSpeech.QUEUE_ADD, null, "return-now")
        buzz(context, linkBuzz(true))
    }
}

@Composable
private fun LinkLossBuzz(context: Context) {
    val flyJson by qgcPath(FLY_STATE)
    val lost = flyState(flyJson)?.contactLost == true
    var heard by remember { mutableStateOf<Boolean?>(null) }
    LaunchedEffect(lost) {
        if (heard != null || lost) buzz(context, linkBuzz(lost))
        heard = lost
    }
}
