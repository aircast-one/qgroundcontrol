package one.aircast.android

import android.annotation.SuppressLint
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
import one.aircast.android.ui.SPEECH_VIEW
import one.aircast.android.ui.speechBatch
import one.aircast.android.ui.speechPath

object SpeechOut {
    private const val POLL_MS = 500L
    private const val MAX_TEXT_QUEUE = 20

    private var engine: TextToSpeech? = null
    private var appContext: Context? = null
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
        appContext = context.applicationContext
        engine = TextToSpeech(context.applicationContext) { status ->
            initialised = status == TextToSpeech.SUCCESS
        }
        poller = Executors.newSingleThreadScheduledExecutor().apply {
            scheduleWithFixedDelay(::poll, POLL_MS, POLL_MS, TimeUnit.MILLISECONDS)
        }
    }

    fun say(text: String, clip: String) {
        poller?.execute { if (ready && !muted) speak(text, clip, 1f) }
    }

    private fun poll() {
        if (initialised && !ready) {
            engine?.language = Locale.US
            engine?.setOnUtteranceProgressListener(progress)
            ready = true
        }
        val batch = speechBatch(runCatching { Qgc.get(after?.let(::speechPath) ?: SPEECH_VIEW) }.getOrNull()) ?: return
        val seen = after
        after = batch.last
        if (batch.muted && !muted && ready) flush()
        muted = batch.muted
        if (seen == null || !ready) return
        batch.lines.filter { it.sequence > seen }.forEach { speak(it.text, it.clip, it.volume) }
    }

    @SuppressLint("DiscouragedApi")
    private fun speak(text: String, clip: String, volume: Float) {
        if (waiting.get() >= MAX_TEXT_QUEUE) flush()
        val context = appContext ?: return
        context.resources.getIdentifier(clip, "raw", context.packageName).takeIf { it != 0 }?.let { engine?.addSpeech(text, context.packageName, it) }
        val params = Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, volume) }
        engine?.speak(text, TextToSpeech.QUEUE_ADD, params, "qgc-${generation.get()}-$clip")
            ?.takeIf { it == TextToSpeech.SUCCESS }?.let { waiting.incrementAndGet() }
    }

    fun stop() {
        poller?.shutdownNow()
        poller = null
        engine?.shutdown()
        engine = null
        appContext = null
        initialised = false
        ready = false
        after = null
        waiting.set(0)
        muted = false
    }
}
