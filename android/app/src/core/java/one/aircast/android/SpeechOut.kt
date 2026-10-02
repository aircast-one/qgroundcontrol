package one.aircast.android

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import java.util.Locale
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import one.aircast.android.bridge.Qgc

object SpeechOut {
    private const val POLL_MS = 500L
    private const val SPEECH_VIEW = "view.speech"
    private const val MAX_TEXT_QUEUE = 20

    private var engine: TextToSpeech? = null
    @Volatile private var initialised = false
    private var ready = false
    private var poller: ScheduledExecutorService? = null
    private var after: Long? = null
    private val waiting = AtomicInteger(0)
    private val generation = AtomicInteger(0)
    private var muted = false

    private val progress = object : UtteranceProgressListener() {
        override fun onStart(utteranceId: String?) = Unit
        override fun onDone(utteranceId: String?) = finished(utteranceId)
        @Deprecated("Deprecated in Java")
        override fun onError(utteranceId: String?) = finished(utteranceId)
        override fun onStop(utteranceId: String?, interrupted: Boolean) = finished(utteranceId)
        private fun finished(utteranceId: String?) {
            if (utteranceId?.startsWith("qgc-${generation.get()}-") == true) waiting.updateAndGet { (it - 1).coerceAtLeast(0) }
        }
    }

    private fun flush() {
        generation.incrementAndGet()
        engine?.stop()
        waiting.set(0)
    }

    fun start(context: Context) {
        if (engine != null) return
        engine = TextToSpeech(context.applicationContext) { status ->
            initialised = status == TextToSpeech.SUCCESS
        }
        poller = Executors.newSingleThreadScheduledExecutor().apply {
            scheduleWithFixedDelay(::poll, POLL_MS, POLL_MS, TimeUnit.MILLISECONDS)
        }
    }

    private fun poll() {
        if (initialised && !ready) {
            engine?.language = Locale.US
            engine?.setOnUtteranceProgressListener(progress)
            ready = true
        }
        val view = runCatching { Qgc.get(after?.let { "$SPEECH_VIEW($it)" } ?: SPEECH_VIEW) }.getOrNull() ?: return
        val seen = after
        after = view.optLong("last", seen ?: 0L)
        val nowMuted = view.optBoolean("muted")
        if (nowMuted && !muted && ready) flush()
        muted = nowMuted
        if (seen == null || !ready) return
        val lines = view.optJSONArray("lines") ?: return
        (0 until lines.length()).mapNotNull { lines.optJSONObject(it) }.forEach { line ->
            if (waiting.get() >= MAX_TEXT_QUEUE) flush()
            val volume = Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, line.optDouble("volume", 1.0).toFloat()) }
            engine?.speak(line.optString("text"), TextToSpeech.QUEUE_ADD, volume, "qgc-${generation.get()}-${line.optLong("sequence")}")
                ?.takeIf { it == TextToSpeech.SUCCESS }?.let { waiting.incrementAndGet() }
        }
    }

    fun stop() {
        poller?.shutdownNow()
        poller = null
        engine?.shutdown()
        engine = null
        initialised = false
        ready = false
        after = null
        waiting.set(0)
        muted = false
    }
}
