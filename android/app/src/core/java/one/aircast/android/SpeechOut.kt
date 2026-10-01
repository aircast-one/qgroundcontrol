package one.aircast.android

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import java.util.Locale
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import one.aircast.android.bridge.Qgc

object SpeechOut {
    private const val POLL_MS = 500L
    private const val SPEECH_VIEW = "view.speech"

    private var engine: TextToSpeech? = null
    private var ready = false
    private var poller: ScheduledExecutorService? = null
    private var after: Long? = null

    fun start(context: Context) {
        if (engine != null) return
        engine = TextToSpeech(context.applicationContext) { status ->
            ready = status == TextToSpeech.SUCCESS
            if (ready) engine?.language = Locale.US
        }
        poller = Executors.newSingleThreadScheduledExecutor().apply {
            scheduleWithFixedDelay(::poll, POLL_MS, POLL_MS, TimeUnit.MILLISECONDS)
        }
    }

    private fun poll() {
        val view = runCatching { Qgc.get(after?.let { "$SPEECH_VIEW($it)" } ?: SPEECH_VIEW) }.getOrNull() ?: return
        val seen = after
        after = view.optLong("last", seen ?: 0L)
        if (seen == null || !ready) return
        val lines = view.optJSONArray("lines") ?: return
        (0 until lines.length()).mapNotNull { lines.optJSONObject(it) }.forEach { line ->
            val volume = Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, line.optDouble("volume", 1.0).toFloat()) }
            engine?.speak(line.optString("text"), TextToSpeech.QUEUE_ADD, volume, "qgc-${line.optLong("sequence")}")
        }
    }

    fun stop() {
        poller?.shutdownNow()
        poller = null
        engine?.shutdown()
        engine = null
        ready = false
        after = null
    }
}
